//! Local mail store: one SQLite file opened through Diesel.
//!
//! This is the only crate that names `diesel` or `libsqlite3-sys`. Apple
//! targets open the file with SQLCipher; other targets use plain SQLite until
//! their CI runners have an OpenSSL to link, and refuse a key rather than
//! pretend to encrypt.

#[cfg(not(target_arch = "wasm32"))]
mod migrate;
#[cfg(not(target_arch = "wasm32"))]
mod open;
#[cfg(not(target_arch = "wasm32"))]
mod repo;
#[cfg(not(target_arch = "wasm32"))]
mod schema;

#[cfg(not(target_arch = "wasm32"))]
pub use open::{KEY_LEN, Store};
#[cfg(not(target_arch = "wasm32"))]
pub use repo::{Account, Counts, Cursor, Mailbox, StoredMessage, ThreadPage, ThreadSummary};

/// Failure returned by the store.
///
/// No variant carries the database key: SQLite reports the failing step, not
/// the statement text.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The database path is not UTF-8, which SQLite's open call needs.
    #[error("database path is not UTF-8")]
    Path,
    /// SQLite could not open the file.
    #[error("cannot open database: {0}")]
    Connection(String),
    /// A statement failed.
    #[error("database error: {0}")]
    Database(String),
    /// The key is not a raw 256-bit SQLCipher key.
    #[error("database key must be {expected} bytes, got {len}")]
    KeyLength {
        /// Bytes the caller passed.
        len: usize,
        /// Bytes SQLCipher expects for a raw key.
        expected: usize,
    },
    /// A key was given but this build links plain SQLite.
    #[error("this build cannot encrypt the database")]
    CipherUnavailable,
    /// The key does not open the file, or the file is not a database.
    #[error("wrong key or not a database")]
    WrongKey,
    /// A schema migration failed. The file keeps the last schema that applied.
    #[error("migration failed: {0}")]
    Migration(String),
    /// SQLite kept another journal mode, so readers would block the writer.
    #[error("journal mode is {mode}, not wal")]
    NotWal {
        /// Mode SQLite reported.
        mode: String,
    },
}
