//! Verified model blobs and a download resume offset.
//!
//! The bytes come from [`BlobStore`]. This module hashes them and can delete
//! them. It does not open a socket.

use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::Error;

/// Where blob bytes live. The catalog never fetches them.
pub trait BlobStore {
    /// The full bytes of `id`.
    ///
    /// # Errors
    ///
    /// [`Error::MissingBlob`] when `id` is not stored.
    fn read(&self, id: &str) -> Result<Vec<u8>, Error>;

    /// Removes `id`.
    ///
    /// # Errors
    ///
    /// [`Error::MissingBlob`] when `id` is not stored.
    fn delete(&mut self, id: &str) -> Result<(), Error>;
}

/// Resume cursors for blobs that live in `store`.
pub struct Catalog<S> {
    store: S,
    resume: HashMap<String, u64>,
}

impl<S> Catalog<S> {
    /// An empty catalog over `store`.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            resume: HashMap::new(),
        }
    }

    /// How far `id` had been written. An unknown id is `0`.
    #[must_use]
    pub fn resume_offset(&self, id: &str) -> u64 {
        self.resume.get(id).copied().unwrap_or(0)
    }

    /// Remembers `offset` for a later resume. This does not read the bytes.
    pub fn record_resume(&mut self, id: &str, offset: u64) {
        self.resume.insert(id.to_owned(), offset);
    }
}

impl<S: BlobStore> Catalog<S> {
    /// Hashes the blob and compares it with `expected`.
    ///
    /// # Errors
    ///
    /// [`Error::MissingBlob`] when the store has no bytes.
    /// [`Error::HashMismatch`] when the SHA-256 differs.
    pub fn verify(&self, id: &str, expected: &[u8; 32]) -> Result<(), Error> {
        let bytes = self.store.read(id)?;
        if sha256(&bytes) != *expected {
            return Err(Error::HashMismatch);
        }
        Ok(())
    }

    /// Deletes the blob and drops its resume offset.
    ///
    /// The offset is a cursor into those bytes, so it goes away with them.
    ///
    /// # Errors
    ///
    /// [`Error::MissingBlob`] when the store has no blob.
    pub fn delete(&mut self, id: &str) -> Result<(), Error> {
        self.store.delete(id)?;
        self.resume.remove(id);
        Ok(())
    }
}

/// SHA-256 of `bytes`. A digest of the wrong length becomes zeros so a
/// comparison fails closed instead of treating it as a match.
pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    from_digest(Sha256::digest(bytes).as_slice())
}

pub(crate) fn from_digest(slice: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    if slice.len() == out.len() {
        out.copy_from_slice(slice);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{BlobStore, Catalog, sha256};
    use crate::Error;

    #[derive(Default)]
    struct Mem {
        blobs: HashMap<String, Vec<u8>>,
    }

    impl BlobStore for Mem {
        fn read(&self, id: &str) -> Result<Vec<u8>, Error> {
            self.blobs.get(id).cloned().ok_or(Error::MissingBlob)
        }

        fn delete(&mut self, id: &str) -> Result<(), Error> {
            if self.blobs.remove(id).is_some() {
                Ok(())
            } else {
                Err(Error::MissingBlob)
            }
        }
    }

    #[test]
    fn a_blob_is_verified_resumed_and_deleted() {
        let bytes = b"weights";
        let mut store = Mem::default();
        store.blobs.insert("small".into(), bytes.to_vec());
        let mut catalog = Catalog::new(store);
        catalog.verify("small", &sha256(bytes)).unwrap();
        let mut wrong = sha256(bytes);
        wrong[0] ^= 0xff;
        assert!(matches!(
            catalog.verify("small", &wrong),
            Err(Error::HashMismatch)
        ));
        assert_eq!(catalog.resume_offset("small"), 0);
        catalog.record_resume("small", 12);
        assert_eq!(catalog.resume_offset("small"), 12);
        catalog.delete("small").unwrap();
        assert!(matches!(
            catalog.verify("small", &sha256(bytes)),
            Err(Error::MissingBlob)
        ));
        assert_eq!(catalog.resume_offset("small"), 0);
        assert!(matches!(catalog.delete("gone"), Err(Error::MissingBlob)));
    }
}
