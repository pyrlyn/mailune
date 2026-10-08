//! OpenPGP sign and encrypt.
//!
//! Key bytes stay off [`Key`]'s `Debug`. Nothing here reads a keychain or
//! opens a socket. The randomness is [`rand::rngs::OsRng`], which the test
//! process already has; it is not a network call.

use std::fmt;

use pgp::composed::{
    EncryptionCaps, KeyType, Message, MessageBuilder, SecretKeyParamsBuilder, SignedPublicKey,
    SignedSecretKey, SubkeyParamsBuilder,
};
use pgp::crypto::ecc_curve::ECCCurve;
use pgp::crypto::hash::HashAlgorithm;
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::types::Password;
use rand::rngs::OsRng;

/// Failure from key generation or a message. The text does not include key
/// bytes or the plaintext.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The key could not be generated.
    #[error("openpgp key could not be created")]
    Key,
    /// Sign or encrypt failed.
    #[error("openpgp message could not be protected")]
    Protect,
    /// Decrypt or verify failed.
    #[error("openpgp message could not be opened")]
    Open,
}

/// A secret key and the certificate derived from it.
///
/// `Debug` prints `redacted` and not the secret packets.
pub struct Key {
    secret: SignedSecretKey,
    public: SignedPublicKey,
}

impl Key {
    /// Generates a signing primary key and an encryption subkey.
    ///
    /// `uid` is the OpenPGP user id, for example `Ada <ada@example.com>`.
    ///
    /// # Errors
    ///
    /// [`Error::Key`] when the parameters cannot be built or generated.
    pub fn generate(uid: &str) -> Result<Self, Error> {
        let mut encrypt = SubkeyParamsBuilder::default();
        encrypt
            .key_type(KeyType::ECDH(ECCCurve::Curve25519Legacy))
            .can_encrypt(EncryptionCaps::All);
        let encrypt = encrypt.build().map_err(|_| Error::Key)?;

        let mut params = SecretKeyParamsBuilder::default();
        params
            .key_type(KeyType::Ed25519Legacy)
            .can_certify(true)
            .can_sign(true)
            .primary_user_id(uid.to_string())
            .subkeys(vec![encrypt]);
        let secret = params
            .build()
            .map_err(|_| Error::Key)?
            .generate(OsRng)
            .map_err(|_| Error::Key)?;
        let public = secret.to_public_key();
        Ok(Self { secret, public })
    }

    /// Signs `plaintext` and encrypts it to this key.
    ///
    /// # Errors
    ///
    /// [`Error::Protect`] when the message cannot be signed or encrypted.
    pub fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
        let encryption = self.public.public_subkeys.first().ok_or(Error::Protect)?;
        let mut builder = MessageBuilder::from_bytes("", plaintext.to_vec())
            .seipd_v1(OsRng, SymmetricKeyAlgorithm::AES256);
        builder.sign(
            &self.secret.primary_key,
            Password::empty(),
            HashAlgorithm::Sha256,
        );
        builder
            .encrypt_to_key(OsRng, encryption)
            .map_err(|_| Error::Protect)?;
        builder.to_vec(OsRng).map_err(|_| Error::Protect)
    }

    /// Decrypts `ciphertext` and checks the signature.
    ///
    /// # Errors
    ///
    /// [`Error::Open`] when the bytes do not decrypt or the signature does not
    /// verify.
    pub fn open(&self, ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
        let mut message = Message::from_bytes(ciphertext)
            .map_err(|_| Error::Open)?
            .decrypt(&Password::empty(), &self.secret)
            .map_err(|_| Error::Open)?;
        if message.is_compressed() {
            message = message.decompress().map_err(|_| Error::Open)?;
        }
        let plain = message.as_data_vec().map_err(|_| Error::Open)?;
        message.verify(&self.public).map_err(|_| Error::Open)?;
        Ok(plain)
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Key")
            .field("key", &"redacted")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use pgp::ser::Serialize;

    use super::Key;

    #[test]
    fn a_generated_key_signs_encrypts_then_decrypts_and_verifies() {
        let key = Key::generate("Ada <ada@example.com>").unwrap();
        let plain = b"hello from mailune";
        let sealed = key.protect(plain).unwrap();
        assert_ne!(sealed, plain);
        let opened = key.open(&sealed).unwrap();
        assert_eq!(opened, plain);
    }

    #[test]
    fn debug_does_not_include_the_key_bytes() {
        let key = Key::generate("Ada <ada@example.com>").unwrap();
        let mut raw = Vec::new();
        key.secret.to_writer(&mut raw).unwrap();
        let rendered = format!("{key:?}");
        assert!(rendered.contains("redacted"), "{rendered}");
        assert!(!rendered.contains("primary_key"), "{rendered}");
        let sample = format!("{:?}", &raw[..16]);
        assert!(!rendered.contains(&sample), "{rendered}");
    }

    #[test]
    fn a_tampered_message_does_not_open() {
        let key = Key::generate("Ada <ada@example.com>").unwrap();
        let mut sealed = key.protect(b"hello from mailune").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0xff;
        assert!(key.open(&sealed).is_err());
    }
}
