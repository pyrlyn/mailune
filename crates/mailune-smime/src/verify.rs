//! Verify a CMS `SignedData` (RFC 5652, section 5).

use cms::content_info::ContentInfo;
use cms::signed_data::{CertificateSet, SignedData, SignerIdentifier, SignerInfo};
use x509_cert::der::asn1::OctetString;
use x509_cert::der::oid::ObjectIdentifier;
use x509_cert::der::oid::db::rfc5911::{
    ID_CONTENT_TYPE, ID_DATA, ID_MESSAGE_DIGEST, ID_SIGNED_DATA,
};
use x509_cert::der::oid::db::rfc5912::{
    RSA_ENCRYPTION, SHA_256_WITH_RSA_ENCRYPTION, SHA_384_WITH_RSA_ENCRYPTION,
    SHA_512_WITH_RSA_ENCRYPTION,
};
use x509_cert::der::{Decode, Encode};

use crate::{Certificate, Error, Hash};

/// A signed message whose every signature checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// The signed content: the encapsulated bytes, or the detached bytes the
    /// caller passed.
    pub content: Vec<u8>,
    /// The certificate of each signer, in signer order. Not checked for
    /// trust; compare [`Certificate::emails`] with the `From` address.
    pub signers: Vec<Certificate>,
}

/// Verifies `signed`, a DER `ContentInfo` holding `SignedData`.
///
/// Pass `detached` for a `multipart/signed` message (the exact bytes of the
/// first part); leave it `None` when the content is inside `signed`. Every
/// signer must verify: one bad signature fails the whole message, so a
/// forged signer cannot hide behind a good one.
///
/// # Errors
///
/// [`Error::BadSignature`] when any signature or message digest does not
/// match, [`Error::UnknownSigner`] when a signer's certificate is missing,
/// [`Error::Unsupported`] for an algorithm outside the supported set, and
/// [`Error::Malformed`] or [`Error::Shape`] when the bytes are not signed data.
pub fn verify(signed: &[u8], detached: Option<&[u8]>) -> Result<Verified, Error> {
    let info = ContentInfo::from_der(signed)?;
    if info.content_type != ID_SIGNED_DATA {
        return Err(Error::Shape("signed data"));
    }
    let data: SignedData = info.content.decode_as()?;
    let encap = &data.encap_content_info;
    let content = match (&encap.econtent, detached) {
        (Some(inner), None) => inner.decode_as::<OctetString>()?.into_bytes(),
        (None, Some(bytes)) => bytes.to_vec(),
        // Two contents would leave it unclear which one was signed.
        (Some(_), Some(_)) => return Err(Error::Shape("detached and encapsulated at once")),
        (None, None) => return Err(Error::Shape("signed data with content")),
    };
    if data.signer_infos.0.is_empty() {
        return Err(Error::Shape("signed data with a signer"));
    }
    let mut signers = Vec::new();
    for signer in data.signer_infos.0.iter() {
        let cert = find_signer(data.certificates.as_ref(), &signer.sid)?;
        check(signer, encap.econtent_type, &content, &cert)?;
        signers.push(cert);
    }
    Ok(Verified { content, signers })
}

fn check(
    signer: &SignerInfo,
    content_type: ObjectIdentifier,
    content: &[u8],
    cert: &Certificate,
) -> Result<(), Error> {
    let hash = Hash::from_oid(&signer.digest_alg.oid)?;
    signature_matches(&signer.signature_algorithm.oid, hash)?;
    let digest = hash.digest(content);
    let signed_bytes = match &signer.signed_attrs {
        Some(attrs) => {
            let wanted = single_value(attrs, ID_MESSAGE_DIGEST)?
                .decode_as::<OctetString>()?
                .into_bytes();
            if wanted != digest {
                return Err(Error::BadSignature);
            }
            let stated: ObjectIdentifier = single_value(attrs, ID_CONTENT_TYPE)?.decode_as()?;
            if stated != content_type {
                return Err(Error::BadSignature);
            }
            // The signature covers the attributes as a DER SET, not as the
            // implicit [0] they are tagged with inside SignerInfo.
            hash.digest(&attrs.to_der()?)
        }
        // Without attributes the content type is not signed, so only plain
        // data is allowed (RFC 5652, section 5.3).
        None if content_type == ID_DATA => digest,
        None => return Err(Error::Shape("signed attributes for non-data content")),
    };
    cert.public_key()?
        .verify(hash.pkcs1v15(), &signed_bytes, signer.signature.as_bytes())
        .map_err(|_| Error::BadSignature)
}

/// RFC 8551 lets the signature algorithm be plain `rsaEncryption`, or the
/// combined OID, which then has to name the same digest.
fn signature_matches(oid: &ObjectIdentifier, hash: Hash) -> Result<(), Error> {
    let fits = match *oid {
        RSA_ENCRYPTION => true,
        SHA_256_WITH_RSA_ENCRYPTION => hash == Hash::Sha256,
        SHA_384_WITH_RSA_ENCRYPTION => hash == Hash::Sha384,
        SHA_512_WITH_RSA_ENCRYPTION => hash == Hash::Sha512,
        _ => return Err(Error::Unsupported(oid.to_string())),
    };
    if fits {
        Ok(())
    } else {
        Err(Error::BadSignature)
    }
}

/// The one value of the one attribute called `oid`. RFC 5652 forbids
/// repeating either, and a repeat could smuggle a second digest.
fn single_value(
    attrs: &cms::signed_data::SignedAttributes,
    oid: ObjectIdentifier,
) -> Result<&x509_cert::der::Any, Error> {
    let mut found = attrs.iter().filter(|attr| attr.oid == oid);
    match (found.next(), found.next()) {
        (Some(attr), None) if attr.values.len() == 1 => {
            attr.values.iter().next().ok_or(Error::BadSignature)
        }
        _ => Err(Error::BadSignature),
    }
}

fn find_signer(
    certs: Option<&CertificateSet>,
    sid: &SignerIdentifier,
) -> Result<Certificate, Error> {
    let (issuer_serial, key_id) = match sid {
        SignerIdentifier::IssuerAndSerialNumber(ias) => (Some(ias), None),
        SignerIdentifier::SubjectKeyIdentifier(ski) => (None, Some(ski.0.as_bytes())),
    };
    certs
        .into_iter()
        .flat_map(|set| set.0.iter())
        .filter_map(|choice| match choice {
            cms::cert::CertificateChoices::Certificate(cert) => Some(Certificate(cert.clone())),
            cms::cert::CertificateChoices::Other(_) => None,
        })
        .find(|cert| cert.is(issuer_serial, key_id))
        .ok_or(Error::UnknownSigner)
}

#[cfg(test)]
mod tests {
    use super::verify;
    use crate::Error;
    use crate::cert::tests::cert;

    const CONTENT: &[u8] = include_bytes!("../fixtures/content.eml");
    const DETACHED: &[u8] = include_bytes!("../fixtures/signed-detached.p7s");
    const OPAQUE: &[u8] = include_bytes!("../fixtures/signed-opaque-keyid.p7m");

    #[test]
    fn an_openssl_detached_signature_verifies() {
        let verified = verify(DETACHED, Some(CONTENT)).unwrap();
        assert_eq!(verified.content, CONTENT);
        assert_eq!(verified.signers, [cert("ada")]);
    }

    #[test]
    fn an_opaque_signature_by_key_id_verifies() {
        let verified = verify(OPAQUE, None).unwrap();
        assert_eq!(verified.content, CONTENT);
        assert_eq!(verified.signers[0].emails(), ["ada@example.com"]);
    }

    #[test]
    fn changed_content_fails() {
        let mut changed = CONTENT.to_vec();
        changed[CONTENT.len() - 3] ^= 1;
        assert!(matches!(
            verify(DETACHED, Some(&changed)),
            Err(Error::BadSignature)
        ));
    }

    #[test]
    fn a_changed_signature_fails() {
        let mut changed = DETACHED.to_vec();
        // The RSA signature is the last 256 bytes of a single-signer blob.
        let at = changed.len() - 10;
        changed[at] ^= 1;
        assert!(matches!(
            verify(&changed, Some(CONTENT)),
            Err(Error::BadSignature)
        ));
    }

    #[test]
    fn wrong_shapes_are_refused() {
        assert!(matches!(verify(DETACHED, None), Err(Error::Shape(_))));
        assert!(matches!(
            verify(OPAQUE, Some(CONTENT)),
            Err(Error::Shape(_))
        ));
        let enveloped = include_bytes!("../fixtures/enveloped-aes256.p7m");
        assert!(matches!(verify(enveloped, None), Err(Error::Shape(_))));
        assert!(matches!(verify(b"junk", None), Err(Error::Malformed(_))));
    }
}
