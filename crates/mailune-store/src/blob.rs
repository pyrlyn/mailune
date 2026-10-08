//! Content-addressed bodies.
//!
//! The address is the SHA-256 of the plaintext. ChaCha20-Poly1305 wants a
//! 32-byte key, and the caller key is an arbitrary slice, so the cipher key
//! is that hash. Neither key is written into `Debug` or an error.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::{Error, hex_encode};

const NONCE_LEN: usize = 12;

/// Encrypted blobs under one directory, capped by `quota` stored bytes.
pub struct BlobStore {
    dir: PathBuf,
    key: [u8; 32],
    quota: u64,
}

impl std::fmt::Debug for BlobStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobStore")
            .field("dir", &self.dir)
            .field("quota", &self.quota)
            .finish()
    }
}

impl BlobStore {
    /// Creates `dir` when it is missing. `quota` is the maximum stored size.
    ///
    /// # Errors
    ///
    /// [`Error::Blob`] when the directory cannot be created.
    pub fn open(dir: &Path, key: &[u8], quota: u64) -> Result<Self, Error> {
        fs::create_dir_all(dir).map_err(|_| Error::Blob)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            key: derive_key(key),
            quota,
        })
    }

    /// Stores `plaintext` and drops the oldest files until the quota holds.
    ///
    /// The returned digest is the plaintext address. A second put of the same
    /// bytes returns the same digest. If the new file itself does not fit, it
    /// is removed too.
    ///
    /// # Errors
    ///
    /// [`Error::Blob`] when the write or the cipher fails.
    pub fn put(&self, plaintext: &[u8]) -> Result<String, Error> {
        let digest = hex_encode(&Sha256::digest(plaintext));
        let path = self.path(&digest)?;
        let sealed = seal(&self.key, plaintext)?;
        fs::write(&path, sealed).map_err(|_| Error::Blob)?;
        self.evict(&path)?;
        Ok(digest)
    }

    /// Decrypts the blob named by `digest`.
    ///
    /// `Ok(None)` means the file is absent, including after eviction.
    ///
    /// # Errors
    ///
    /// [`Error::Blob`] when the name is not a digest, or the ciphertext does
    /// not open with this store's key.
    pub fn get(&self, digest: &str) -> Result<Option<Vec<u8>>, Error> {
        let path = self.path(digest)?;
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Error::Blob),
        };
        open_sealed(&self.key, &bytes).map(Some)
    }
}

fn derive_key(key: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(key);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| Error::Blob)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(nonce_bytes.as_slice()).map_err(|_| Error::Blob)?;
    let ciphertext = cipher.encrypt(&nonce, plaintext).map_err(|_| Error::Blob)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn open_sealed(key: &[u8; 32], bytes: &[u8]) -> Result<Vec<u8>, Error> {
    if bytes.len() < NONCE_LEN {
        return Err(Error::Blob);
    }
    let (nonce_bytes, ciphertext) = bytes.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| Error::Blob)?;
    let nonce = Nonce::try_from(nonce_bytes).map_err(|_| Error::Blob)?;
    cipher.decrypt(&nonce, ciphertext).map_err(|_| Error::Blob)
}

impl BlobStore {
    fn path(&self, digest: &str) -> Result<PathBuf, Error> {
        // A digest is only hex, so it cannot be a path that leaves `dir`.
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::Blob);
        }
        Ok(self.dir.join(digest))
    }

    fn evict(&self, keep: &Path) -> Result<(), Error> {
        let mut files = Vec::new();
        let mut total = 0u64;
        for entry in fs::read_dir(&self.dir).map_err(|_| Error::Blob)? {
            let entry = entry.map_err(|_| Error::Blob)?;
            let meta = entry.metadata().map_err(|_| Error::Blob)?;
            if !meta.is_file() {
                continue;
            }
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let len = meta.len();
            total = total.saturating_add(len);
            files.push((modified, entry.path(), len));
        }
        files.sort_by(|left, right| {
            let left_kept = left.1 == keep;
            let right_kept = right.1 == keep;
            left_kept.cmp(&right_kept).then(left.0.cmp(&right.0))
        });
        for (_, path, len) in files {
            if total <= self.quota {
                break;
            }
            fs::remove_file(&path).map_err(|_| Error::Blob)?;
            total = total.saturating_sub(len);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BlobStore;
    use crate::scratch_dir;

    #[test]
    fn addresses_encrypts_and_evicts() {
        let dir = scratch_dir();
        let blobs = dir.join("blobs");
        const KEY: &[u8] = b"blob-secret-key";
        let store = BlobStore::open(&blobs, KEY, u64::MAX).unwrap();
        let rendered = format!("{store:?}");
        assert!(!rendered.contains("blob-secret-key"));

        let first = store.put(b"aaaa").unwrap();
        let again = store.put(b"aaaa").unwrap();
        assert_eq!(first, again);
        assert_eq!(store.get(&first).unwrap().unwrap(), b"aaaa");

        let other = BlobStore::open(&blobs, b"other-secret-key", u64::MAX).unwrap();
        assert!(other.get(&first).is_err());

        let len = std::fs::metadata(blobs.join(&first)).unwrap().len();
        let capped = BlobStore::open(&blobs, KEY, len).unwrap();
        let second = capped.put(b"bbbb").unwrap();
        assert!(capped.get(&first).unwrap().is_none());
        assert_eq!(capped.get(&second).unwrap().unwrap(), b"bbbb");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
