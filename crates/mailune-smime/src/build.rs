//! Sign (CMS `SignedData`) and encrypt (CMS `EnvelopedData`).
//!
//! The algorithms are the ones RFC 8551 says every receiver supports:
//! SHA-256 with RSA PKCS #1 v1.5, and AES-256-CBC with RSA key transport.
//! Randomness comes from the OS ([`rand::rngs::OsRng`]).

use cms::builder::{
    ContentEncryptionAlgorithm, EnvelopedDataBuilder, KeyEncryptionInfo,
    KeyTransRecipientInfoBuilder, SignedDataBuilder, SignerInfoBuilder,
    create_signing_time_attribute,
};
use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::ContentInfo;
use cms::enveloped_data::RecipientIdentifier;
use cms::signed_data::{EncapsulatedContentInfo, SignerIdentifier};
use rand::rngs::OsRng;
use rsa::pkcs1v15::SigningKey;
use rsa::sha2::{Digest, Sha256};
use rsa::traits::PublicKeyParts;
use spki::AlgorithmIdentifierOwned;
use x509_cert::der::oid::db::rfc5911::{ID_DATA, ID_ENVELOPED_DATA};
use x509_cert::der::oid::db::rfc5912::ID_SHA_256;
use x509_cert::der::{Any, Encode, Tag};
use x509_cert::spki;

use crate::{Certificate, Error, PrivateKey};

/// The smallest RSA modulus this crate encrypts to, in bits. RFC 8551
/// section 2.3 says smaller keys should not be used.
const MIN_RSA_BITS: usize = 2048;

/// Where the signed content goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signature {
    /// Content outside the signature, for `multipart/signed`: readers
    /// without S/MIME still see the message.
    Detached,
    /// Content inside the signature, for `application/pkcs7-mime;
    /// smime-type=signed-data`, the form to encrypt afterwards.
    Opaque,
}

/// Signs `content` as `certificate`, whose private key is `key`.
///
/// The certificate goes into the message so the receiver can verify without
/// a directory. Returns a DER `ContentInfo`.
///
/// # Errors
///
/// [`Error::Key`] when `key` does not belong to `certificate`, and
/// [`Error::Build`] when the structure cannot be assembled.
pub fn sign(
    content: &[u8],
    certificate: &Certificate,
    key: &PrivateKey,
    shape: Signature,
) -> Result<Vec<u8>, Error> {
    // A key from another certificate would yield a signature nobody can verify.
    if certificate.public_key()? != key.0.to_public_key() {
        return Err(Error::Key);
    }
    let digest = Sha256::digest(content).to_vec();
    let encap = EncapsulatedContentInfo {
        econtent_type: ID_DATA,
        econtent: match shape {
            Signature::Detached => None,
            Signature::Opaque => Some(Any::new(Tag::OctetString, content.to_vec())?),
        },
    };
    let external = match shape {
        Signature::Detached => Some(digest.as_slice()),
        Signature::Opaque => None,
    };
    let sha256 = AlgorithmIdentifierOwned {
        oid: ID_SHA_256,
        parameters: None,
    };
    let signer = SigningKey::<Sha256>::new(key.0.clone());
    let tbs = &certificate.0.tbs_certificate;
    let sid = SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
        issuer: tbs.issuer.clone(),
        serial_number: tbs.serial_number.clone(),
    });
    let mut signer_info = SignerInfoBuilder::new(&signer, sid, sha256.clone(), &encap, external)
        .map_err(|_| Error::Build)?;
    signer_info
        .add_signed_attribute(create_signing_time_attribute().map_err(|_| Error::Build)?)
        .map_err(|_| Error::Build)?;
    let info = SignedDataBuilder::new(&encap)
        .add_digest_algorithm(sha256)
        .and_then(|builder| {
            builder.add_certificate(CertificateChoices::Certificate(certificate.0.clone()))
        })
        .and_then(|builder| {
            builder.add_signer_info::<SigningKey<Sha256>, rsa::pkcs1v15::Signature>(signer_info)
        })
        .and_then(|builder| builder.build())
        .map_err(|_| Error::Build)?;
    Ok(info.to_der()?)
}

/// Encrypts `content` to every certificate in `recipients`.
///
/// Include the sender's own certificate to keep the sent copy readable.
/// Returns a DER `ContentInfo`.
///
/// # Errors
///
/// [`Error::Shape`] when `recipients` is empty, [`Error::Unsupported`] for
/// a recipient key under 2048 bits, [`Error::Key`] for a non-RSA key, and
/// [`Error::Build`] when the structure cannot be assembled.
pub fn encrypt(content: &[u8], recipients: &[Certificate]) -> Result<Vec<u8>, Error> {
    if recipients.is_empty() {
        return Err(Error::Shape("encrypted to at least one recipient"));
    }
    // One generator per recipient: the builder borrows each until `build`.
    let mut rngs: Vec<OsRng> = vec![OsRng; recipients.len()];
    let mut builder =
        EnvelopedDataBuilder::new(None, content, ContentEncryptionAlgorithm::Aes256Cbc, None)
            .map_err(|_| Error::Build)?;
    for (recipient, rng) in recipients.iter().zip(rngs.iter_mut()) {
        let public = recipient.public_key()?;
        if public.n().bits() < MIN_RSA_BITS {
            return Err(Error::Unsupported(format!(
                "RSA key of {} bits",
                public.n().bits()
            )));
        }
        let tbs = &recipient.0.tbs_certificate;
        let rid = RecipientIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: tbs.issuer.clone(),
            serial_number: tbs.serial_number.clone(),
        });
        let entry = KeyTransRecipientInfoBuilder::new(rid, KeyEncryptionInfo::Rsa(public), rng)
            .map_err(|_| Error::Build)?;
        builder
            .add_recipient_info(entry)
            .map_err(|_| Error::Build)?;
    }
    let data = builder
        .build_with_rng(&mut OsRng)
        .map_err(|_| Error::Build)?;
    let info = ContentInfo {
        content_type: ID_ENVELOPED_DATA,
        content: Any::encode_from(&data)?,
    };
    Ok(info.to_der()?)
}

#[cfg(test)]
mod tests {
    use rsa::RsaPrivateKey;
    use rsa::pkcs8::EncodePublicKey;

    use super::{Signature, encrypt, sign};
    use crate::cert::tests::{cert, key};
    use crate::{Error, decrypt, verify};

    const BODY: &[u8] = b"Content-Type: text/plain\r\n\r\nSigned by Ada.\r\n";

    #[test]
    fn a_detached_signature_verifies_through_c4() {
        let signed = sign(BODY, &cert("ada"), &key("ada"), Signature::Detached).unwrap();
        let verified = verify(&signed, Some(BODY)).unwrap();
        assert_eq!(verified.signers, [cert("ada")]);
        assert!(verify(&signed, Some(b"something else")).is_err());
    }

    #[test]
    fn an_opaque_signature_carries_its_content() {
        let signed = sign(BODY, &cert("ada"), &key("ada"), Signature::Opaque).unwrap();
        assert_eq!(verify(&signed, None).unwrap().content, BODY);
    }

    #[test]
    fn a_key_from_another_certificate_does_not_sign() {
        assert!(matches!(
            sign(BODY, &cert("ada"), &key("bob"), Signature::Detached),
            Err(Error::Key)
        ));
    }

    #[test]
    fn each_recipient_decrypts_through_c4() {
        let sealed = encrypt(BODY, &[cert("ada"), cert("bob")]).unwrap();
        assert_eq!(decrypt(&sealed, &cert("ada"), &key("ada")).unwrap(), BODY);
        assert_eq!(decrypt(&sealed, &cert("bob"), &key("bob")).unwrap(), BODY);
    }

    #[test]
    fn sign_then_encrypt_round_trips() {
        let signed = sign(BODY, &cert("ada"), &key("ada"), Signature::Opaque).unwrap();
        let sealed = encrypt(&signed, &[cert("bob")]).unwrap();
        let opened = decrypt(&sealed, &cert("bob"), &key("bob")).unwrap();
        assert_eq!(verify(&opened, None).unwrap().content, BODY);
    }

    #[test]
    fn nobody_or_a_weak_key_is_refused() {
        assert!(matches!(encrypt(BODY, &[]), Err(Error::Shape(_))));
        // Only the bit length is checked, so the certificate can lie about
        // its key: swap a 1024-bit key into ada's certificate.
        let weak = RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
        let der = weak.to_public_key().to_public_key_der().unwrap();
        let mut small = cert("ada");
        small.0.tbs_certificate.subject_public_key_info =
            x509_cert::der::Decode::from_der(der.as_bytes()).unwrap();
        assert!(matches!(
            encrypt(BODY, &[small]),
            Err(Error::Unsupported(_))
        ));
    }
}
