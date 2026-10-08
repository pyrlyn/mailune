//! Model catalog: which blob a model needs and how to check it.
//!
//! The bytes come from a [`BlobSource`] the host supplies and land through
//! the contract's [`Fs`]. Nothing here opens a socket. A download records a
//! resume offset after every chunk, so a dropped transfer continues where it
//! stopped, and a blob is trusted only after its SHA-256 matches the catalog.

use std::future::Future;
use std::path::Path;

use mailune_protocol::Fs;
use sha2::{Digest, Sha256};

use crate::{Error, ModelCapability};

/// One model the app can install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// What the router sees once the blob is installed.
    pub capability: ModelCapability,
    /// Where the source fetches from. Opaque to this crate.
    pub source: String,
    /// Expected size in bytes.
    pub size: u64,
    /// Expected SHA-256, lowercase hex.
    pub sha256: String,
}

/// Where blob bytes come from: an HTTP range client in the app, a fixture in tests.
pub trait BlobSource: Send + Sync {
    /// Up to `max` bytes of `source` starting at `offset`. Empty means the end.
    ///
    /// # Errors
    ///
    /// The host's error when the transfer fails.
    fn chunk(
        &self,
        source: &str,
        offset: u64,
        max: usize,
    ) -> impl Future<Output = Result<Vec<u8>, mailune_protocol::Error>> + Send;
}

/// Progress a caller persists between attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resume {
    /// Bytes already on disk.
    pub offset: u64,
}

/// Bytes asked of the source per call.
pub const CHUNK: usize = 64 * 1024;

/// Fetches `entry` into `path` from `resume`, calling `record` with the new
/// offset after each chunk is on disk.
///
/// # Errors
///
/// The source or [`Fs`] error, after `record` has seen the last good offset.
/// [`Error::BadBlob`] when the source sends more than `entry.size`.
pub async fn download<S, F>(
    entry: &CatalogEntry,
    source: &S,
    fs: &F,
    path: &Path,
    resume: Resume,
    mut record: impl FnMut(Resume),
) -> Result<Resume, Error>
where
    S: BlobSource,
    F: Fs,
{
    // A missing partial file, or one that does not match the recorded offset,
    // cannot be trusted to line up with the source, so start over.
    let mut bytes = if resume.offset == 0 {
        Vec::new()
    } else {
        fs.read(path).await.unwrap_or_default()
    };
    if u64::try_from(bytes.len()).ok() != Some(resume.offset) {
        bytes.clear();
    }
    let mut offset = u64::try_from(bytes.len()).map_err(|_| Error::BadBlob)?;
    while offset < entry.size {
        let chunk = source.chunk(&entry.source, offset, CHUNK).await?;
        if chunk.is_empty() {
            break;
        }
        bytes.extend_from_slice(&chunk);
        offset = u64::try_from(bytes.len()).map_err(|_| Error::BadBlob)?;
        if offset > entry.size {
            fs.remove(path).await?;
            return Err(Error::BadBlob);
        }
        fs.write(path, &bytes).await?;
        record(Resume { offset });
    }
    Ok(Resume { offset })
}

/// Checks the blob at `path` against `entry`. A blob that does not match is
/// deleted so it can never be loaded.
///
/// # Errors
///
/// [`Error::BadBlob`] on a size or digest mismatch; the [`Fs`] error when the
/// blob cannot be read.
pub async fn verify<F: Fs>(entry: &CatalogEntry, fs: &F, path: &Path) -> Result<(), Error> {
    let bytes = fs.read(path).await?;
    let size_ok = u64::try_from(bytes.len()).ok() == Some(entry.size);
    if size_ok && hex(&Sha256::digest(&bytes)) == entry.sha256.to_ascii_lowercase() {
        return Ok(());
    }
    fs.remove(path).await?;
    Err(Error::BadBlob)
}

/// Deletes an installed blob.
///
/// # Errors
///
/// The [`Fs`] error.
pub async fn delete<F: Fs>(fs: &F, path: &Path) -> Result<(), Error> {
    fs.remove(path).await.map_err(Error::from)
}

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 0x0f)]),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;

    use mailune_protocol::Fs;
    use mailune_testkit::FakeHost;
    use sha2::{Digest, Sha256};

    use super::{BlobSource, CHUNK, CatalogEntry, Resume, delete, download, hex, verify};
    use crate::testing::drive;
    use crate::{Error, Feature, ModelCapability, ModelKind};

    /// Serves `blob`, failing once after `fail_after` calls.
    struct Fixture {
        blob: Vec<u8>,
        calls: Mutex<Vec<u64>>,
        fail_after: Option<usize>,
    }

    impl BlobSource for Fixture {
        async fn chunk(
            &self,
            _source: &str,
            offset: u64,
            max: usize,
        ) -> Result<Vec<u8>, mailune_protocol::Error> {
            let mut calls = self.calls.lock().unwrap();
            calls.push(offset);
            if Some(calls.len()) == self.fail_after.map(|n| n + 1) {
                return Err(mailune_protocol::Error::host("blob.chunk", "dropped"));
            }
            let start = usize::try_from(offset).unwrap().min(self.blob.len());
            let end = (start + max).min(self.blob.len());
            Ok(self.blob[start..end].to_vec())
        }
    }

    fn entry(blob: &[u8]) -> CatalogEntry {
        CatalogEntry {
            capability: ModelCapability {
                id: "tiny".into(),
                kind: ModelKind::Local,
                context_tokens: Some(2048),
                features: vec![Feature::Summarize],
            },
            source: "fixture://tiny".into(),
            size: u64::try_from(blob.len()).unwrap(),
            sha256: hex(&Sha256::digest(blob)),
        }
    }

    #[test]
    fn a_dropped_download_resumes_verifies_and_deletes() {
        let blob: Vec<u8> = (0..CHUNK * 2 + 10).map(|i| (i % 251) as u8).collect();
        let entry = entry(&blob);
        let host = FakeHost::new();
        let path = Path::new("models/tiny.gguf");
        let source = Fixture {
            blob: blob.clone(),
            calls: Mutex::new(Vec::new()),
            fail_after: Some(1),
        };
        let mut recorded = Vec::new();
        let first = drive(download(
            &entry,
            &source,
            &host,
            path,
            Resume { offset: 0 },
            |r| recorded.push(r),
        ));
        assert!(matches!(first, Err(Error::Contract(_))));
        let resume = *recorded.last().unwrap();
        assert_eq!(resume.offset, CHUNK as u64);

        let source = Fixture {
            blob,
            calls: Mutex::new(Vec::new()),
            fail_after: None,
        };
        let done = drive(download(&entry, &source, &host, path, resume, |_| {})).unwrap();
        assert_eq!(done.offset, entry.size);
        // The second attempt starts where the first stopped.
        assert_eq!(source.calls.lock().unwrap()[0], CHUNK as u64);
        drive(verify(&entry, &host, path)).unwrap();

        drive(delete(&host, path)).unwrap();
        assert!(drive(host.read(path)).is_err());
    }

    #[test]
    fn a_wrong_digest_is_rejected_and_removed() {
        let blob = b"not the model".to_vec();
        let mut entry = entry(&blob);
        entry.sha256 = hex(&Sha256::digest(b"something else"));
        let host = FakeHost::new();
        let path = Path::new("models/bad.gguf");
        drive(host.write(path, &blob)).unwrap();
        assert!(matches!(
            drive(verify(&entry, &host, path)),
            Err(Error::BadBlob)
        ));
        assert!(drive(host.read(path)).is_err());
        assert_eq!(hex(&[0x00, 0xab, 0xff]), "00abff");
    }
}
