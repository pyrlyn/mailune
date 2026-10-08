//! S/MIME verify and decrypt.
//!
//! `cms` 0.2.3 parses the message. The signature is checked with the RSA key
//! inside the supplied certificate, and the enveloped content key is opened
//! with the supplied private key. Nothing reads a keychain or opens a socket.
//! RSA with SHA-256 and AES-CBC key transport are the algorithms this crate
//! checks. Other recipient types are left unread.

use aes::{Aes128, Aes192, Aes256};
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, KeyIvInit};
use cms::content_info::ContentInfo;
use cms::enveloped_data::{EncryptedContentInfo, EnvelopedData, RecipientInfo};
use cms::signed_data::{SignedAttributes, SignedData, SignerIdentifier};
use const_oid::ObjectIdentifier;
use der::{Decode, Encode, Tagged};
use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::{Digest, Sha256};
use rsa::signature::Verifier;
use spki::DecodePublicKey;
use x509_cert::Certificate;

/// Failure from one S/MIME message. The text does not include key bytes.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a CMS `ContentInfo` this crate can read.
    #[error("the cms message could not be read")]
    Message,
    /// The signature did not match the supplied certificate.
    #[error("the signature could not be verified")]
    Signature,
    /// The content-encryption key or the ciphertext could not be opened.
    #[error("the message could not be decrypted")]
    Decrypt,
}

/// Verify a signed CMS message and return the encapsulated content.
///
/// `certificate` is a DER X.509 certificate. The signer identifier must be
/// that certificate's issuer and serial number.
///
/// # Errors
///
/// [`Error::Message`] when the bytes are not signed-data.
/// [`Error::Signature`] when the certificate or the signature does not match.
pub fn verify(message: &[u8], certificate: &[u8]) -> Result<Vec<u8>, Error> {
    let signed = signed_data(message)?;
    let certificate = Certificate::from_der(certificate).map_err(|_| Error::Signature)?;
    let content = signed
        .encap_content_info
        .econtent
        .as_ref()
        .map(der::Any::value)
        .ok_or(Error::Signature)?;
    let digest = Sha256::digest(content);
    let public = rsa_public(&certificate)?;
    let signer = signed
        .signer_infos
        .0
        .iter()
        .next()
        .ok_or(Error::Signature)?;
    if signer.digest_alg.oid != const_oid::db::rfc5912::ID_SHA_256 {
        return Err(Error::Signature);
    }
    if signer.signature_algorithm.oid != const_oid::db::rfc5912::SHA_256_WITH_RSA_ENCRYPTION {
        return Err(Error::Signature);
    }
    match &signer.sid {
        SignerIdentifier::IssuerAndSerialNumber(id) => {
            if id.issuer != certificate.tbs_certificate.issuer
                || id.serial_number != certificate.tbs_certificate.serial_number
            {
                return Err(Error::Signature);
            }
        }
        SignerIdentifier::SubjectKeyIdentifier(_) => return Err(Error::Signature),
    }
    let signed_attrs = signer.signed_attrs.as_ref().ok_or(Error::Signature)?;
    let advertised = message_digest(signed_attrs)?;
    if advertised != digest.as_slice() {
        return Err(Error::Signature);
    }
    // RFC 5652 signs the DER SET, not the IMPLICIT [0] tag stored on SignerInfo.
    let signed_der = signed_attrs.to_der().map_err(|_| Error::Signature)?;
    let parsed = Signature::try_from(signer.signature.as_bytes()).map_err(|_| Error::Signature)?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&signed_der, &parsed)
        .map_err(|_| Error::Signature)?;
    Ok(content.to_vec())
}

/// Decrypt an enveloped CMS message with a PKCS#1 or PKCS#8 RSA private key.
///
/// # Errors
///
/// [`Error::Message`] when the bytes are not enveloped-data.
/// [`Error::Decrypt`] when no key-transport recipient opens with this key.
pub fn decrypt(message: &[u8], private_key: &[u8]) -> Result<Vec<u8>, Error> {
    let enveloped = enveloped_data(message)?;
    let private = RsaPrivateKey::from_pkcs1_der(private_key)
        .or_else(|_| RsaPrivateKey::from_pkcs8_der(private_key))
        .map_err(|_| Error::Decrypt)?;
    for recipient in enveloped.recip_infos.0.iter() {
        let RecipientInfo::Ktri(info) = recipient else {
            continue;
        };
        if info.key_enc_alg.oid != const_oid::db::rfc5912::RSA_ENCRYPTION {
            continue;
        }
        let Ok(cek) = private.decrypt(rsa::Pkcs1v15Encrypt, info.enc_key.as_bytes()) else {
            continue;
        };
        if let Ok(plain) = open_content(&enveloped.encrypted_content, &cek) {
            return Ok(plain);
        }
    }
    Err(Error::Decrypt)
}

fn signed_data(message: &[u8]) -> Result<SignedData, Error> {
    let info = ContentInfo::from_der(message).map_err(|_| Error::Message)?;
    if info.content_type != const_oid::db::rfc5911::ID_SIGNED_DATA {
        return Err(Error::Message);
    }
    let bytes = info.content.to_der().map_err(|_| Error::Message)?;
    SignedData::from_der(&bytes).map_err(|_| Error::Message)
}

fn enveloped_data(message: &[u8]) -> Result<EnvelopedData, Error> {
    let info = ContentInfo::from_der(message).map_err(|_| Error::Message)?;
    if info.content_type != const_oid::db::rfc5911::ID_ENVELOPED_DATA {
        return Err(Error::Message);
    }
    let bytes = info.content.to_der().map_err(|_| Error::Message)?;
    EnvelopedData::from_der(&bytes).map_err(|_| Error::Message)
}

fn rsa_public(certificate: &Certificate) -> Result<RsaPublicKey, Error> {
    let der = certificate
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|_| Error::Signature)?;
    RsaPublicKey::from_public_key_der(&der).map_err(|_| Error::Signature)
}

fn message_digest(attributes: &SignedAttributes) -> Result<&[u8], Error> {
    let attribute = attributes
        .iter()
        .find(|attribute| attribute.oid == const_oid::db::rfc5911::ID_MESSAGE_DIGEST)
        .ok_or(Error::Signature)?;
    let value = attribute.values.iter().next().ok_or(Error::Signature)?;
    if value.tag() != der::Tag::OctetString {
        return Err(Error::Signature);
    }
    Ok(value.value())
}

fn open_content(content: &EncryptedContentInfo, key: &[u8]) -> Result<Vec<u8>, Error> {
    let iv = content
        .content_enc_alg
        .parameters
        .as_ref()
        .filter(|parameters| parameters.tag() == der::Tag::OctetString)
        .map(der::Any::value)
        .ok_or(Error::Decrypt)?;
    let ciphertext = content
        .encrypted_content
        .as_ref()
        .map(der::asn1::OctetString::as_bytes)
        .ok_or(Error::Decrypt)?;
    aes_cbc(content.content_enc_alg.oid, key, iv, ciphertext)
}

fn aes_cbc(
    oid: ObjectIdentifier,
    key: &[u8],
    iv: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, Error> {
    if oid == const_oid::db::rfc5911::ID_AES_128_CBC && key.len() == 16 {
        return cbc::Decryptor::<Aes128>::new_from_slices(key, iv)
            .map_err(|_| Error::Decrypt)?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| Error::Decrypt);
    }
    if oid == const_oid::db::rfc5911::ID_AES_192_CBC && key.len() == 24 {
        return cbc::Decryptor::<Aes192>::new_from_slices(key, iv)
            .map_err(|_| Error::Decrypt)?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| Error::Decrypt);
    }
    if oid == const_oid::db::rfc5911::ID_AES_256_CBC && key.len() == 32 {
        return cbc::Decryptor::<Aes256>::new_from_slices(key, iv)
            .map_err(|_| Error::Decrypt)?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| Error::Decrypt);
    }
    Err(Error::Decrypt)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::time::Duration;

    use cms::builder::{
        ContentEncryptionAlgorithm, EnvelopedDataBuilder, KeyEncryptionInfo,
        KeyTransRecipientInfoBuilder, SignedDataBuilder, SignerInfoBuilder,
    };
    use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
    use cms::content_info::ContentInfo;
    use cms::enveloped_data::RecipientIdentifier;
    use cms::signed_data::{EncapsulatedContentInfo, SignerIdentifier};
    use der::{AnyRef, Decode, Encode, Tag};
    use rand::rngs::OsRng;
    use rsa::RsaPrivateKey;
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::sha2::Sha256;
    use rsa::signature::Keypair;
    use spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned};
    use x509_cert::builder::{Builder, CertificateBuilder, Profile};
    use x509_cert::name::Name;
    use x509_cert::serial_number::SerialNumber;
    use x509_cert::time::Validity;

    use super::{decrypt, verify};

    fn certificate_and_key() -> (Vec<u8>, Vec<u8>, SigningKey<Sha256>, rsa::RsaPublicKey) {
        let mut rng = OsRng;
        let private = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public = rsa::RsaPublicKey::from(&private);
        let key_der = private.to_pkcs1_der().unwrap().as_bytes().to_vec();
        let signing = SigningKey::<Sha256>::new(private);
        let serial_number = SerialNumber::from(7u8);
        let validity = Validity::from_now(Duration::from_secs(86_400)).unwrap();
        let subject = Name::from_str("CN=Mailune").unwrap();
        let spki = SubjectPublicKeyInfoOwned::from_key(signing.verifying_key()).unwrap();
        let certificate = CertificateBuilder::new(
            Profile::Root,
            serial_number,
            validity,
            subject,
            spki,
            &signing,
        )
        .unwrap()
        .build()
        .unwrap();
        let certificate_der = certificate.to_der().unwrap();
        (certificate_der, key_der, signing, public)
    }

    fn signed_message(
        plaintext: &[u8],
        signing: &SigningKey<Sha256>,
        certificate_der: &[u8],
    ) -> Vec<u8> {
        let certificate = x509_cert::Certificate::from_der(certificate_der).unwrap();
        let content = EncapsulatedContentInfo {
            econtent_type: const_oid::db::rfc5911::ID_DATA,
            econtent: Some(der::Any::new(Tag::OctetString, plaintext.to_vec()).unwrap()),
        };
        let digest = AlgorithmIdentifierOwned {
            oid: const_oid::db::rfc5912::ID_SHA_256,
            parameters: None,
        };
        let sid = SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: certificate.tbs_certificate.issuer.clone(),
            serial_number: certificate.tbs_certificate.serial_number.clone(),
        });
        let signer = SignerInfoBuilder::new(signing, sid, digest.clone(), &content, None).unwrap();
        SignedDataBuilder::new(&content)
            .add_digest_algorithm(digest)
            .unwrap()
            .add_certificate(CertificateChoices::Certificate(certificate))
            .unwrap()
            .add_signer_info::<SigningKey<Sha256>, rsa::pkcs1v15::Signature>(signer)
            .unwrap()
            .build()
            .unwrap()
            .to_der()
            .unwrap()
    }

    fn enveloped_message(
        plaintext: &[u8],
        public: rsa::RsaPublicKey,
        certificate_der: &[u8],
    ) -> Vec<u8> {
        let certificate = x509_cert::Certificate::from_der(certificate_der).unwrap();
        let rid = RecipientIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: certificate.tbs_certificate.issuer.clone(),
            serial_number: certificate.tbs_certificate.serial_number.clone(),
        });
        let mut rng = OsRng;
        let recipient =
            KeyTransRecipientInfoBuilder::new(rid, KeyEncryptionInfo::Rsa(public), &mut rng)
                .unwrap();
        let mut builder =
            EnvelopedDataBuilder::new(None, plaintext, ContentEncryptionAlgorithm::Aes128Cbc, None)
                .unwrap();
        let enveloped = builder
            .add_recipient_info(recipient)
            .unwrap()
            .build_with_rng(&mut OsRng)
            .unwrap();
        let body = enveloped.to_der().unwrap();
        ContentInfo {
            content_type: const_oid::db::rfc5911::ID_ENVELOPED_DATA,
            content: der::Any::from(AnyRef::try_from(body.as_slice()).unwrap()),
        }
        .to_der()
        .unwrap()
    }

    #[test]
    fn a_fixture_verifies_and_decrypts_with_the_supplied_key() {
        let (certificate, private_key, signing, public) = certificate_and_key();
        let plaintext = b"hello smime";
        let signed = signed_message(plaintext, &signing, &certificate);
        assert_eq!(verify(&signed, &certificate).unwrap(), plaintext);
        let mut broken = signed.clone();
        let last = broken.len() - 1;
        broken[last] ^= 0x01;
        assert!(verify(&broken, &certificate).is_err());
        let enveloped = enveloped_message(plaintext, public, &certificate);
        assert_eq!(decrypt(&enveloped, &private_key).unwrap(), plaintext);
        let other = RsaPrivateKey::new(&mut OsRng, 1024).unwrap();
        let other_der = other.to_pkcs1_der().unwrap();
        assert!(decrypt(&enveloped, other_der.as_bytes()).is_err());
    }
}
