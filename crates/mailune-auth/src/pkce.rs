//! PKCE (RFC 7636) verifier and S256 challenge.
//!
//! The verifier is a secret. [`Pkce`]'s `Debug` redacts it. Nothing here opens
//! a browser or stores a token.

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use crate::Error;

/// A verifier and its S256 challenge.
///
/// `Debug` prints the challenge and the word `redacted` for the verifier.
pub struct Pkce {
    verifier: String,
    challenge: String,
}

impl Pkce {
    /// Builds a pair from a verifier the caller already chose.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Pkce`] when the verifier is not 43 to 128 characters
    /// from the unreserved set in RFC 7636.
    pub fn from_verifier(verifier: impl Into<String>) -> Result<Self, Error> {
        let verifier = verifier.into();
        let challenge = s256_challenge(&verifier)?;
        Ok(Self {
            verifier,
            challenge,
        })
    }

    /// Builds a pair by encoding `entropy` as an unpadded base64url verifier.
    ///
    /// 32 bytes is the length RFC 7636 recommends. The bytes are not read from
    /// the operating system here.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Pkce`] when `entropy` is empty or the encoded verifier
    /// is outside the allowed length.
    pub fn from_entropy(entropy: &[u8]) -> Result<Self, Error> {
        if entropy.is_empty() {
            return Err(Error::Pkce("entropy is empty".into()));
        }
        Self::from_verifier(URL_SAFE_NO_PAD.encode(entropy))
    }

    /// The secret verifier. Do not log it.
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// The S256 challenge, safe to send to the authorization server.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

impl fmt::Debug for Pkce {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Pkce")
            .field("verifier", &"redacted")
            .field("challenge", &self.challenge)
            .finish()
    }
}

/// `BASE64URL(SHA256(verifier))` without padding, as in RFC 7636.
///
/// # Errors
///
/// Returns [`Error::Pkce`] when `verifier` is not a legal code verifier.
pub fn s256_challenge(verifier: &str) -> Result<String, Error> {
    validate(verifier)?;
    let digest = Sha256::digest(verifier.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(digest))
}

fn validate(verifier: &str) -> Result<(), Error> {
    let len = verifier.len();
    if !(43..=128).contains(&len) {
        return Err(Error::Pkce(format!("length {len} is outside 43..=128")));
    }
    let unreserved = |byte: u8| {
        matches!(
            byte,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
        )
    };
    if !verifier.bytes().all(unreserved) {
        return Err(Error::Pkce(
            "contains a character outside the unreserved set".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Pkce, s256_challenge};

    /// RFC 7636 appendix B.
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    #[test]
    fn s256_matches_the_rfc_vector() {
        assert_eq!(s256_challenge(VERIFIER).unwrap(), CHALLENGE);
        let pair = Pkce::from_verifier(VERIFIER).unwrap();
        assert_eq!(pair.challenge(), CHALLENGE);
        assert_eq!(pair.verifier(), VERIFIER);
    }

    #[test]
    fn debug_does_not_include_the_verifier() {
        let pair = Pkce::from_verifier(VERIFIER).unwrap();
        let rendered = format!("{pair:?}");
        assert!(!rendered.contains(VERIFIER));
        assert!(rendered.contains(CHALLENGE));
        assert!(rendered.contains("redacted"));
    }

    #[test]
    fn entropy_encodes_to_a_verifier_and_its_challenge() {
        let entropy = [7u8; 32];
        let pair = Pkce::from_entropy(&entropy).unwrap();
        assert_eq!(pair.verifier().len(), 43);
        assert_eq!(pair.challenge(), s256_challenge(pair.verifier()).unwrap());
        let rendered = format!("{pair:?}");
        assert!(!rendered.contains(pair.verifier()));
    }

    #[test]
    fn rejects_a_short_verifier() {
        assert!(s256_challenge("too-short").is_err());
    }
}
