//! Content-addressed blob store for bodies and attachments.
//!
//! The address is the SHA-256 of the plaintext, so the same attachment in
//! two messages is stored once. Bytes are sealed with AES-256-GCM under a
//! key the caller supplies; the address is the associated data, so a row
//! cannot be swapped under another address. Least-recently-used blobs are
//! evicted once the stored total passes the quota: a body can always be
//! fetched again from the server.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use diesel::dsl::max;
use diesel::prelude::*;
use sha2::{Digest, Sha256};

use crate::open::database_error;
use crate::schema::blobs;
use crate::{Error, Store};

/// Bytes in a blob key.
pub const BLOB_KEY_LEN: usize = 32;

/// Hex SHA-256 of a blob's plaintext.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlobHash(String);

impl BlobHash {
    /// Borrows the lowercase hex digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps a digest read back from a row (for example `messages.body_hash`).
    pub fn new(hex: impl Into<String>) -> Self {
        Self(hex.into())
    }
}

/// Blob access with one key and one quota.
pub struct Blobs<'a> {
    store: &'a mut Store,
    cipher: Aes256Gcm,
    quota: u64,
}

impl std::fmt::Debug for Blobs<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Blobs")
            .field("quota", &self.quota)
            .finish_non_exhaustive()
    }
}

impl Store {
    /// Opens the blob store with `key` (32 bytes) and a quota in stored bytes.
    ///
    /// # Errors
    ///
    /// [`Error::KeyLength`] when `key` is not 32 bytes.
    pub fn blobs(&mut self, key: &[u8], quota: u64) -> Result<Blobs<'_>, Error> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| Error::KeyLength {
            len: key.len(),
            expected: BLOB_KEY_LEN,
        })?;
        Ok(Blobs {
            store: self,
            cipher,
            quota,
        })
    }
}

impl Blobs<'_> {
    /// Stores `bytes` and returns their address, then evicts older blobs
    /// until the total fits the quota. Storing the same bytes again only
    /// marks them as recently used.
    ///
    /// # Errors
    ///
    /// [`Error::BlobTooLarge`] when the sealed blob alone exceeds the quota,
    /// [`Error::BlobKey`] if sealing fails, [`Error::Database`] otherwise.
    pub fn put(&mut self, bytes: &[u8]) -> Result<BlobHash, Error> {
        let digest = Sha256::digest(bytes);
        let hash = BlobHash(hex(&digest));
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&digest[..12]);
        // The nonce is derived from the plaintext, so it only repeats for the
        // same plaintext, which yields the same ciphertext and leaks nothing new.
        let sealed = self
            .cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: bytes,
                    aad: hash.0.as_bytes(),
                },
            )
            .map_err(|_| Error::BlobKey)?;
        let size = i64::try_from(sealed.len()).unwrap_or(i64::MAX);
        if u64::try_from(size).unwrap_or(u64::MAX) > self.quota {
            return Err(Error::BlobTooLarge);
        }
        let quota = self.quota;
        self.store
            .conn
            .transaction(|conn| {
                let used = next_use(conn)?;
                diesel::insert_into(blobs::table)
                    .values((
                        blobs::hash.eq(&hash.0),
                        blobs::data.eq(&sealed),
                        blobs::size.eq(size),
                        blobs::used.eq(used),
                    ))
                    .on_conflict(blobs::hash)
                    .do_update()
                    .set(blobs::used.eq(used))
                    .execute(conn)?;
                evict(conn, quota, &hash.0)
            })
            .map_err(database_error)?;
        Ok(hash)
    }

    /// Reads the blob at `hash`, or `None` when it was never stored or has
    /// been evicted.
    ///
    /// # Errors
    ///
    /// [`Error::BlobKey`] when the key does not open the blob or the bytes
    /// do not match the address, [`Error::Database`] otherwise.
    pub fn get(&mut self, hash: &BlobHash) -> Result<Option<Vec<u8>>, Error> {
        let sealed: Option<Vec<u8>> = blobs::table
            .filter(blobs::hash.eq(&hash.0))
            .select(blobs::data)
            .first(&mut self.store.conn)
            .optional()
            .map_err(database_error)?;
        let Some(sealed) = sealed else {
            return Ok(None);
        };
        let digest = parse_hex(&hash.0).ok_or(Error::BlobKey)?;
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&digest[..12]);
        let plain = self
            .cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &sealed,
                    aad: hash.0.as_bytes(),
                },
            )
            .map_err(|_| Error::BlobKey)?;
        if Sha256::digest(&plain).as_slice() != digest.as_slice() {
            return Err(Error::BlobKey);
        }
        let conn = &mut self.store.conn;
        let used = next_use(conn).map_err(database_error)?;
        diesel::update(blobs::table.filter(blobs::hash.eq(&hash.0)))
            .set(blobs::used.eq(used))
            .execute(conn)
            .map_err(database_error)?;
        Ok(Some(plain))
    }

    /// Sealed bytes currently stored.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails.
    pub fn stored_bytes(&mut self) -> Result<u64, Error> {
        stored(&mut self.store.conn)
            .map(|total| u64::try_from(total).unwrap_or(0))
            .map_err(database_error)
    }
}

fn next_use(conn: &mut SqliteConnection) -> QueryResult<i64> {
    let latest: Option<i64> = blobs::table.select(max(blobs::used)).first(conn)?;
    Ok(latest.unwrap_or(0).saturating_add(1))
}

fn stored(conn: &mut SqliteConnection) -> QueryResult<i64> {
    let sizes: Vec<i64> = blobs::table.select(blobs::size).load(conn)?;
    Ok(sizes.into_iter().sum())
}

/// Drops least-recently-used blobs, never `keep`, until the total fits.
fn evict(conn: &mut SqliteConnection, quota: u64, keep: &str) -> QueryResult<()> {
    let mut total = u64::try_from(stored(conn)?).unwrap_or(0);
    if total <= quota {
        return Ok(());
    }
    let oldest: Vec<(String, i64)> = blobs::table
        .filter(blobs::hash.ne(keep))
        .order(blobs::used.asc())
        .select((blobs::hash, blobs::size))
        .load(conn)?;
    for (hash, size) in oldest {
        if total <= quota {
            break;
        }
        diesel::delete(blobs::table.filter(blobs::hash.eq(&hash))).execute(conn)?;
        total = total.saturating_sub(u64::try_from(size).unwrap_or(0));
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

fn parse_hex(text: &str) -> Option<[u8; 32]> {
    let raw = text.as_bytes();
    if raw.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (index, [high, low]) in raw.as_chunks::<2>().0.iter().enumerate() {
        let high = char::from(*high).to_digit(16)?;
        let low = char::from(*low).to_digit(16)?;
        out[index] = u8::try_from(high * 16 + low).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::BlobHash;
    use crate::{Error, Store};

    const KEY: [u8; 32] = [9; 32];
    // AES-GCM adds a 16-byte tag to each sealed blob.
    const TAG: u64 = 16;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        (dir, store)
    }

    #[test]
    fn the_same_bytes_share_one_address_and_round_trip() {
        let (_dir, mut store) = store();
        let mut blobs = store.blobs(&KEY, 1 << 20).unwrap();
        let first = blobs.put(b"hello body").unwrap();
        let second = blobs.put(b"hello body").unwrap();
        assert_eq!(first, second);
        assert_eq!(first.as_str().len(), 64);
        assert_eq!(
            blobs.get(&first).unwrap().as_deref(),
            Some(&b"hello body"[..])
        );
        assert_eq!(blobs.stored_bytes().unwrap(), 10 + TAG);
    }

    #[test]
    fn stored_bytes_are_not_the_plaintext() {
        let (dir, mut store) = store();
        let mut blobs = store.blobs(&KEY, 1 << 20).unwrap();
        blobs.put(b"a very recognisable secret body").unwrap();
        drop(store);
        let raw = std::fs::read(dir.path().join("mail.db")).unwrap();
        let wal = std::fs::read(dir.path().join("mail.db-wal")).unwrap_or_default();
        let needle = b"recognisable secret";
        assert!(!raw.windows(needle.len()).any(|window| window == needle));
        assert!(!wal.windows(needle.len()).any(|window| window == needle));
    }

    #[test]
    fn another_key_cannot_read_a_blob() {
        let (_dir, mut store) = store();
        let hash = store.blobs(&KEY, 1 << 20).unwrap().put(b"body").unwrap();
        let mut other = store.blobs(&[1; 32], 1 << 20).unwrap();
        assert!(matches!(other.get(&hash), Err(Error::BlobKey)));
    }

    #[test]
    fn the_least_recently_used_blob_goes_first() {
        let (_dir, mut store) = store();
        // Room for two 10-byte blobs, not three.
        let mut blobs = store.blobs(&KEY, 2 * (10 + TAG)).unwrap();
        let a = blobs.put(b"aaaaaaaaaa").unwrap();
        let b = blobs.put(b"bbbbbbbbbb").unwrap();
        // Reading `a` makes `b` the oldest.
        assert!(blobs.get(&a).unwrap().is_some());
        let c = blobs.put(b"cccccccccc").unwrap();
        assert!(blobs.get(&a).unwrap().is_some());
        assert_eq!(blobs.get(&b).unwrap(), None);
        assert!(blobs.get(&c).unwrap().is_some());
        assert!(blobs.stored_bytes().unwrap() <= 2 * (10 + TAG));
    }

    #[test]
    fn a_blob_larger_than_the_quota_is_refused() {
        let (_dir, mut store) = store();
        let mut blobs = store.blobs(&KEY, 8).unwrap();
        assert!(matches!(blobs.put(b"too big"), Err(Error::BlobTooLarge)));
        assert!(matches!(
            store.blobs(&[0; 3], 8),
            Err(Error::KeyLength { .. })
        ));
    }

    #[test]
    fn an_unknown_or_malformed_address_reads_nothing() {
        let (_dir, mut store) = store();
        let mut blobs = store.blobs(&KEY, 1 << 20).unwrap();
        assert_eq!(blobs.get(&BlobHash::new("00".repeat(32))).unwrap(), None);
        assert_eq!(blobs.get(&BlobHash::new("zz")).unwrap(), None);
    }
}
