//! Certificates and private keys, as the caller supplies them.

use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use x509_cert::der::asn1::Ia5StringRef;
use x509_cert::der::oid::db::rfc3280::EMAIL_ADDRESS;
use x509_cert::der::{Decode, DecodePem, Encode};
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::pkix::{SubjectAltName, SubjectKeyIdentifier};

use crate::Error;

/// An X.509 certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate(pub(crate) x509_cert::Certificate);

impl Certificate {
    /// Reads a DER certificate.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Malformed`] when the bytes are not a certificate.
    pub fn from_der(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self(x509_cert::Certificate::from_der(bytes)?))
    }

    /// Reads a PEM `CERTIFICATE` block.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Malformed`] when the text is not a certificate.
    pub fn from_pem(text: &str) -> Result<Self, Error> {
        Ok(Self(x509_cert::Certificate::from_pem(text)?))
    }

    /// The certificate as DER.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Malformed`] when it cannot be encoded.
    pub fn to_der(&self) -> Result<Vec<u8>, Error> {
        Ok(self.0.to_der()?)
    }

    /// The subject as an RFC 4514 string, for display.
    pub fn subject(&self) -> String {
        self.0.tbs_certificate.subject.to_string()
    }

    /// The mail addresses the certificate is for: `rfc822Name` entries of the
    /// subject alternative name, then the legacy subject `emailAddress`.
    ///
    /// RFC 8550 says a receiver checks the `From` address against these, so
    /// a valid signature by someone else's certificate is not shown as the
    /// sender's.
    pub fn emails(&self) -> Vec<String> {
        let tbs = &self.0.tbs_certificate;
        let mut emails: Vec<String> = Vec::new();
        if let Ok(Some((_, san))) = tbs.get::<SubjectAltName>() {
            for name in san.0 {
                if let GeneralName::Rfc822Name(email) = name {
                    emails.push(email.to_string());
                }
            }
        }
        for rdn in &tbs.subject.0 {
            for atv in rdn.0.iter().filter(|atv| atv.oid == EMAIL_ADDRESS) {
                if let Ok(email) = atv.value.decode_as::<Ia5StringRef<'_>>() {
                    emails.push(email.to_string());
                }
            }
        }
        emails
    }

    pub(crate) fn public_key(&self) -> Result<RsaPublicKey, Error> {
        let spki = self.0.tbs_certificate.subject_public_key_info.to_der()?;
        RsaPublicKey::from_public_key_der(&spki).map_err(|_| Error::Key)
    }

    /// The subject key identifier extension, when the certificate has one.
    pub(crate) fn key_id(&self) -> Option<Vec<u8>> {
        match self.0.tbs_certificate.get::<SubjectKeyIdentifier>() {
            Ok(Some((_, ski))) => Some(ski.0.as_bytes().to_vec()),
            _ => None,
        }
    }

    /// Whether `sid` (issuer and serial, or key id) names this certificate.
    pub(crate) fn is(
        &self,
        issuer_serial: Option<&cms::cert::IssuerAndSerialNumber>,
        key_id: Option<&[u8]>,
    ) -> bool {
        let tbs = &self.0.tbs_certificate;
        match (issuer_serial, key_id) {
            (Some(ias), _) => ias.issuer == tbs.issuer && ias.serial_number == tbs.serial_number,
            (None, Some(id)) => self.key_id().is_some_and(|own| own == id),
            (None, None) => false,
        }
    }
}

/// An RSA private key. Its `Debug` never shows the key, and the bytes are
/// zeroed when it is dropped.
pub struct PrivateKey(pub(crate) RsaPrivateKey);

impl PrivateKey {
    /// Reads an unencrypted PKCS #8 DER key.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Key`] when the bytes are not an RSA key.
    pub fn from_pkcs8_der(bytes: &[u8]) -> Result<Self, Error> {
        RsaPrivateKey::from_pkcs8_der(bytes)
            .map(Self)
            .map_err(|_| Error::Key)
    }

    /// Reads an unencrypted PKCS #8 PEM key (`PRIVATE KEY`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Key`] when the text is not an RSA key.
    pub fn from_pkcs8_pem(text: &str) -> Result<Self, Error> {
        RsaPrivateKey::from_pkcs8_pem(text)
            .map(Self)
            .map_err(|_| Error::Key)
    }
}

impl std::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivateKey(redacted)")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{Certificate, PrivateKey};

    pub(crate) fn cert(who: &str) -> Certificate {
        let pem = match who {
            "ada" => include_str!("../fixtures/ada.crt.pem"),
            _ => include_str!("../fixtures/bob.crt.pem"),
        };
        Certificate::from_pem(pem).unwrap()
    }

    pub(crate) fn key(who: &str) -> PrivateKey {
        let pem = match who {
            "ada" => include_str!("../fixtures/ada.key.pem"),
            _ => include_str!("../fixtures/bob.key.pem"),
        };
        PrivateKey::from_pkcs8_pem(pem).unwrap()
    }

    #[test]
    fn a_certificate_names_its_subject_and_address() {
        let ada = cert("ada");
        assert!(ada.subject().contains("CN=ada"));
        assert_eq!(ada.emails(), ["ada@example.com"]);
        assert!(ada.key_id().is_some());
        let der = ada.to_der().unwrap();
        assert_eq!(Certificate::from_der(&der).unwrap(), ada);
    }

    #[test]
    fn a_private_key_is_redacted_in_debug() {
        assert_eq!(format!("{:?}", key("ada")), "PrivateKey(redacted)");
        assert!(PrivateKey::from_pkcs8_der(b"not a key").is_err());
    }
}
