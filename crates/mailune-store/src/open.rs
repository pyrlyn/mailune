//! Opening the database: key first, then WAL and the connection pragmas.
//!
//! Every statement here is a PRAGMA. Diesel's DSL has no PRAGMA builder, so
//! they go through `batch_execute` and `sql_query`, and stay in this file.

use std::fmt::Write as _;
use std::path::Path;

use diesel::connection::{Instrumentation, InstrumentationEvent, SimpleConnection};
use diesel::result::{ConnectionError, DatabaseErrorKind};
use diesel::sql_types::Text;
use diesel::{Connection, QueryableByName, RunQueryDsl, SqliteConnection};
use zeroize::Zeroizing;

use crate::Error;

/// Raw SQLCipher key length: 256 bits.
pub const KEY_LEN: usize = 32;

/// An open database. One connection; callers that need a reader open a second.
pub struct Store {
    pub(crate) conn: SqliteConnection,
    pub(crate) vectors: crate::vector_cache::VectorCache,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Store").finish_non_exhaustive()
    }
}

impl Store {
    /// Opens or creates the database at `path`.
    ///
    /// `key` is the caller's raw 32-byte key, read from the keychain by the
    /// caller. `None` opens an unencrypted file, which tests and platforms
    /// without SQLCipher use.
    ///
    /// # Errors
    ///
    /// [`Error::KeyLength`] for a key that is not 32 bytes,
    /// [`Error::CipherUnavailable`] when a key is given to a plain SQLite
    /// build, [`Error::WrongKey`] when the key does not open the file, and
    /// [`Error::NotWal`] when SQLite refuses write-ahead logging.
    pub fn open(path: &Path, key: Option<&[u8]>) -> Result<Self, Error> {
        if let Some(key) = key
            && key.len() != KEY_LEN
        {
            return Err(Error::KeyLength {
                len: key.len(),
                expected: KEY_LEN,
            });
        }
        let url = path.to_str().ok_or(Error::Path)?;
        let mut conn = SqliteConnection::establish(url).map_err(connection_error)?;
        // A process-wide query logger would otherwise see the PRAGMA key text.
        conn.set_instrumentation(Silent);
        if let Some(key) = key {
            apply_key(&mut conn, key)?;
        }
        configure(&mut conn)?;
        crate::migrate::run(&mut conn)?;
        Ok(Self {
            conn,
            vectors: crate::vector_cache::VectorCache::default(),
        })
    }

    /// SQLCipher's version string, or `None` on a plain SQLite build.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the pragma fails.
    pub fn cipher_version(&mut self) -> Result<Option<String>, Error> {
        cipher_version(&mut self.conn)
    }
}

struct Silent;

impl Instrumentation for Silent {
    fn on_connection_event(&mut self, _event: InstrumentationEvent<'_>) {}
}

fn apply_key(conn: &mut SqliteConnection, key: &[u8]) -> Result<(), Error> {
    // Capacity is exact, so the String never reallocates and leaves no
    // unzeroed copy of the key behind.
    let mut pragma = Zeroizing::new(String::with_capacity(20 + key.len() * 2));
    pragma.push_str("PRAGMA key = \"x'");
    for byte in key {
        let _ = write!(pragma, "{byte:02x}");
    }
    pragma.push_str("'\";");
    conn.batch_execute(&pragma).map_err(database_error)?;
    if cipher_version(conn)?.is_none() {
        return Err(Error::CipherUnavailable);
    }
    Ok(())
}

#[derive(QueryableByName)]
struct CipherVersion {
    #[diesel(sql_type = Text)]
    cipher_version: String,
}

#[derive(QueryableByName)]
struct DataVersion {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    data_version: i64,
}

/// SQLite's counter that moves when another connection commits to the file; this connection's
/// own writes leave it unchanged. The vector cache uses it now; the S7 change feed is meant to
/// build on the same reader.
pub(crate) fn data_version(conn: &mut SqliteConnection) -> Result<i64, Error> {
    let rows: Vec<DataVersion> = diesel::sql_query("PRAGMA data_version")
        .load(conn)
        .map_err(database_error)?;
    rows.into_iter()
        .next()
        .map(|row| row.data_version)
        .ok_or_else(|| Error::Database("PRAGMA data_version returned no row".to_owned()))
}

#[derive(QueryableByName)]
struct JournalMode {
    #[diesel(sql_type = Text)]
    journal_mode: String,
}

fn cipher_version(conn: &mut SqliteConnection) -> Result<Option<String>, Error> {
    // Plain SQLite ignores an unknown PRAGMA and returns no row.
    let rows: Vec<CipherVersion> = diesel::sql_query("PRAGMA cipher_version")
        .load(conn)
        .map_err(database_error)?;
    Ok(rows.into_iter().next().map(|row| row.cipher_version))
}

fn configure(conn: &mut SqliteConnection) -> Result<(), Error> {
    // The first read of the file: a wrong key fails here.
    let rows: Vec<JournalMode> = diesel::sql_query("PRAGMA journal_mode = WAL")
        .load(conn)
        .map_err(database_error)?;
    let mode = rows
        .into_iter()
        .next()
        .map(|row| row.journal_mode)
        .unwrap_or_default();
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(Error::NotWal { mode });
    }
    conn.batch_execute(
        "PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000; PRAGMA synchronous = NORMAL;",
    )
    .map_err(database_error)
}

fn connection_error(err: ConnectionError) -> Error {
    Error::Connection(err.to_string())
}

pub(crate) fn database_error(err: diesel::result::Error) -> Error {
    if let diesel::result::Error::DatabaseError(kind, info) = &err
        && matches!(kind, DatabaseErrorKind::Unknown)
        && info.message().contains("file is not a database")
    {
        return Error::WrongKey;
    }
    Error::Database(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::{KEY_LEN, Store};
    use crate::Error;

    fn key(byte: u8) -> [u8; KEY_LEN] {
        [byte; KEY_LEN]
    }

    #[test]
    fn a_plain_file_opens_in_wal_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        let store = Store::open(&path, None).unwrap();
        drop(store);
        // WAL leaves its side files next to the database while it is open.
        let again = Store::open(&path, None).unwrap();
        assert!(dir.path().join("mail.db-wal").exists());
        drop(again);
    }

    #[test]
    fn a_short_key_is_refused_before_the_file_is_touched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        let err = Store::open(&path, Some(&[1, 2, 3])).unwrap_err();
        assert!(matches!(
            err,
            Error::KeyLength {
                len: 3,
                expected: KEY_LEN
            }
        ));
        assert!(!path.exists());
    }

    #[test]
    fn the_debug_form_shows_no_connection_detail() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        assert_eq!(format!("{store:?}"), "Store { .. }");
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn sqlcipher_opens_with_the_key_and_refuses_another() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        let mut store = Store::open(&path, Some(&key(7))).unwrap();
        assert!(store.cipher_version().unwrap().is_some());
        drop(store);
        assert!(Store::open(&path, Some(&key(7))).is_ok());
        assert!(matches!(
            Store::open(&path, Some(&key(8))),
            Err(Error::WrongKey)
        ));
        assert!(matches!(Store::open(&path, None), Err(Error::WrongKey)));
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.starts_with(b"SQLite format 3"));
    }

    #[cfg(not(target_vendor = "apple"))]
    #[test]
    fn plain_sqlite_refuses_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        assert!(matches!(
            Store::open(&path, Some(&key(7))),
            Err(Error::CipherUnavailable)
        ));
    }
}
