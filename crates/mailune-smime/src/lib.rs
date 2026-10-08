//! S/MIME at the CMS layer: verify and decrypt (C4), sign and encrypt (C5).
//!
//! The input and output are DER `ContentInfo` blobs, the body of an
//! `application/pkcs7-mime` or `application/pkcs7-signature` part. Finding
//! that part, and the exact signed bytes of a `multipart/signed` message, is
//! the MIME layer's job. Keys and certificates are supplied by the caller;
//! this crate never reads the keychain.
//!
//! A verified signature means the content matches a certificate carried in
//! the message. It does not mean the certificate is trusted: chain building
//! and revocation are a separate decision, made by the caller.
//!
//! Supported, because it is what S/MIME 4.0 (RFC 8551) requires and every
//! client sends: RSA PKCS #1 v1.5 signatures over SHA-256/384/512, RSA key
//! transport, and AES-CBC content encryption. Anything else is
//! [`Error::Unsupported`], never a silent pass.

mod build;
mod cert;
mod decrypt;
mod verify;

pub use build::{Signature, encrypt, sign};
pub use cert::{Certificate, PrivateKey};
pub use decrypt::decrypt;
pub use verify::{Verified, verify};

/// Failure from an S/MIME operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not the DER structure that was expected.
    #[error("the S/MIME data could not be read")]
    Malformed(#[source] x509_cert::der::Error),
    /// The structure is well formed but not what this operation takes.
    #[error("the S/MIME data is not {0}")]
    Shape(&'static str),
    /// An algorithm outside the supported set, by OID.
    #[error("unsupported algorithm {0}")]
    Unsupported(String),
    /// A signer's certificate is not in the message.
    #[error("the signer's certificate is not in the message")]
    UnknownSigner,
    /// A signature does not match its content.
    #[error("the signature does not match the content")]
    BadSignature,
    /// No recipient entry names the supplied certificate.
    #[error("the message is not encrypted to this certificate")]
    NotARecipient,
    /// The content could not be decrypted with the supplied key.
    #[error("the message could not be decrypted")]
    Decrypt,
    /// The key or certificate could not be read, or they do not belong together.
    #[error("the key or certificate could not be read")]
    Key,
    /// The CMS builder could not assemble the structure.
    #[error("the S/MIME data could not be built")]
    Build,
}

impl From<x509_cert::der::Error> for Error {
    fn from(err: x509_cert::der::Error) -> Self {
        Self::Malformed(err)
    }
}

/// The digests a signature may use. SHA-1 is left out on purpose: RFC 8551
/// says receivers should treat it as insecure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hash {
    Sha256,
    Sha384,
    Sha512,
}

impl Hash {
    fn from_oid(oid: &x509_cert::der::oid::ObjectIdentifier) -> Result<Self, Error> {
        use x509_cert::der::oid::db::rfc5912::{ID_SHA_256, ID_SHA_384, ID_SHA_512};
        match *oid {
            ID_SHA_256 => Ok(Self::Sha256),
            ID_SHA_384 => Ok(Self::Sha384),
            ID_SHA_512 => Ok(Self::Sha512),
            _ => Err(Error::Unsupported(oid.to_string())),
        }
    }

    fn digest(self, data: &[u8]) -> Vec<u8> {
        use rsa::sha2::{Digest, Sha256, Sha384, Sha512};
        match self {
            Self::Sha256 => Sha256::digest(data).to_vec(),
            Self::Sha384 => Sha384::digest(data).to_vec(),
            Self::Sha512 => Sha512::digest(data).to_vec(),
        }
    }

    fn pkcs1v15(self) -> rsa::Pkcs1v15Sign {
        use rsa::sha2::{Sha256, Sha384, Sha512};
        match self {
            Self::Sha256 => rsa::Pkcs1v15Sign::new::<Sha256>(),
            Self::Sha384 => rsa::Pkcs1v15Sign::new::<Sha384>(),
            Self::Sha512 => rsa::Pkcs1v15Sign::new::<Sha512>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Hash;

    #[test]
    fn sha1_is_not_a_signature_digest() {
        let sha1 = x509_cert::der::oid::db::rfc5912::ID_SHA_1;
        assert!(matches!(
            Hash::from_oid(&sha1),
            Err(super::Error::Unsupported(_))
        ));
    }
}
