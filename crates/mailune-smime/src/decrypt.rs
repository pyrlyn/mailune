//! Decrypt a CMS `EnvelopedData` (RFC 5652, section 6) with RSA key transport.

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use cms::content_info::ContentInfo;
use cms::enveloped_data::{EnvelopedData, RecipientIdentifier, RecipientInfo};
use rsa::Pkcs1v15Encrypt;
use x509_cert::der::Decode;
use x509_cert::der::asn1::OctetString;
use x509_cert::der::oid::db::rfc5911::{
    ID_AES_128_CBC, ID_AES_192_CBC, ID_AES_256_CBC, ID_ENVELOPED_DATA,
};
use x509_cert::der::oid::db::rfc5912::RSA_ENCRYPTION;

use crate::{Certificate, Error, PrivateKey};

/// Decrypts `enveloped`, a DER `ContentInfo` holding `EnvelopedData`, for
/// the recipient `certificate` whose private key is `key`.
///
/// The recipient entry is found by the certificate, not by trying the key on
/// every entry, so a message to many people costs one RSA operation.
///
/// # Errors
///
/// [`Error::NotARecipient`] when no entry names `certificate`,
/// [`Error::Decrypt`] when the key or the padding is wrong,
/// [`Error::Unsupported`] for a cipher outside AES-CBC or a key transport
/// other than RSA PKCS #1 v1.5, and [`Error::Malformed`] or [`Error::Shape`]
/// when the bytes are not enveloped data.
pub fn decrypt(
    enveloped: &[u8],
    certificate: &Certificate,
    key: &PrivateKey,
) -> Result<Vec<u8>, Error> {
    let info = ContentInfo::from_der(enveloped)?;
    if info.content_type != ID_ENVELOPED_DATA {
        return Err(Error::Shape("enveloped data"));
    }
    let data: EnvelopedData = info.content.decode_as()?;
    let entry = data
        .recip_infos
        .0
        .iter()
        .find_map(|info| match info {
            RecipientInfo::Ktri(ktri) if names(certificate, &ktri.rid) => Some(ktri),
            _ => None,
        })
        .ok_or(Error::NotARecipient)?;
    if entry.key_enc_alg.oid != RSA_ENCRYPTION {
        return Err(Error::Unsupported(entry.key_enc_alg.oid.to_string()));
    }
    // PKCS #1 v1.5 decryption is the padding oracle Bleichenbacher found. A
    // mail client answers no one with the result, so there is no oracle to
    // query; still, the error says nothing about why it failed.
    let content_key = key
        .0
        .decrypt(Pkcs1v15Encrypt, entry.enc_key.as_bytes())
        .map_err(|_| Error::Decrypt)?;
    let content = &data.encrypted_content;
    let algorithm = &content.content_enc_alg;
    let iv = algorithm
        .parameters
        .as_ref()
        .ok_or(Error::Shape("an AES-CBC IV"))?
        .decode_as::<OctetString>()?;
    let ciphertext = content
        .encrypted_content
        .as_ref()
        .ok_or(Error::Shape("enveloped data with content"))?
        .as_bytes();
    let (key, iv) = (content_key.as_slice(), iv.as_bytes());
    match algorithm.oid {
        ID_AES_128_CBC => cbc_decrypt::<aes::Aes128>(key, iv, ciphertext),
        ID_AES_192_CBC => cbc_decrypt::<aes::Aes192>(key, iv, ciphertext),
        ID_AES_256_CBC => cbc_decrypt::<aes::Aes256>(key, iv, ciphertext),
        other => Err(Error::Unsupported(other.to_string())),
    }
}

fn names(certificate: &Certificate, rid: &RecipientIdentifier) -> bool {
    match rid {
        RecipientIdentifier::IssuerAndSerialNumber(ias) => certificate.is(Some(ias), None),
        RecipientIdentifier::SubjectKeyIdentifier(ski) => {
            certificate.is(None, Some(ski.0.as_bytes()))
        }
    }
}

fn cbc_decrypt<C>(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, Error>
where
    C: aes::cipher::BlockDecryptMut + aes::cipher::BlockCipher + aes::cipher::KeyInit,
{
    cbc::Decryptor::<C>::new_from_slices(key, iv)
        .map_err(|_| Error::Decrypt)?
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|_| Error::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::decrypt;
    use crate::Error;
    use crate::cert::tests::{cert, key};

    const CONTENT: &[u8] = include_bytes!("../fixtures/content.eml");
    const AES256: &[u8] = include_bytes!("../fixtures/enveloped-aes256.p7m");
    const AES128_TWO: &[u8] = include_bytes!("../fixtures/enveloped-aes128.p7m");

    #[test]
    fn an_openssl_message_decrypts_with_the_supplied_key() {
        assert_eq!(decrypt(AES256, &cert("ada"), &key("ada")).unwrap(), CONTENT);
    }

    #[test]
    fn each_of_two_recipients_decrypts() {
        assert_eq!(
            decrypt(AES128_TWO, &cert("ada"), &key("ada")).unwrap(),
            CONTENT
        );
        assert_eq!(
            decrypt(AES128_TWO, &cert("bob"), &key("bob")).unwrap(),
            CONTENT
        );
    }

    #[test]
    fn a_stranger_or_a_wrong_key_cannot_decrypt() {
        assert!(matches!(
            decrypt(AES256, &cert("bob"), &key("bob")),
            Err(Error::NotARecipient)
        ));
        assert!(matches!(
            decrypt(AES256, &cert("ada"), &key("bob")),
            Err(Error::Decrypt)
        ));
    }

    #[test]
    fn signed_data_is_not_enveloped_data() {
        let signed = include_bytes!("../fixtures/signed-detached.p7s");
        assert!(matches!(
            decrypt(signed, &cert("ada"), &key("ada")),
            Err(Error::Shape(_))
        ));
    }
}
