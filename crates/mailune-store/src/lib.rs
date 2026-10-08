//! SQLite store.
//!
//! The database key is a byte slice from the caller. It is sent to SQLCipher
//! as hex and then discarded. It is never written into an error, a `Debug`
//! string, or a log line. Nothing here opens a socket or the OS keychain.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sql_types::Text;

/// Failure from the store. The text never includes the database key.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be opened, keyed, or switched to WAL.
    #[error("database could not be opened")]
    Open,
    /// A migration could not be applied.
    #[error("database migration failed")]
    Migrate,
    /// A typed query failed.
    #[error("database query failed")]
    Query,
}

/// A file-backed SQLite database in WAL mode.
pub struct Store {
    path: PathBuf,
    conn: SqliteConnection,
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store").field("path", &self.path).finish()
    }
}

impl Store {
    /// Opens `path`, applies `key`, and switches the file to WAL.
    ///
    /// `key` is the SQLCipher key. An empty slice is still sent; the bytes
    /// themselves are not copied into this value's `Debug`.
    ///
    /// # Errors
    ///
    /// [`Error::Open`] when the parent directory, the file, the key pragma,
    /// or the WAL switch fails.
    pub fn open(path: &Path, key: &[u8]) -> Result<Self, Error> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|_| Error::Open)?;
        }
        let path_str = path.to_str().ok_or(Error::Open)?;
        let mut conn = SqliteConnection::establish(path_str).map_err(|_| Error::Open)?;
        // SQLCipher reads the key from the first statement. Later pragmas
        // would run against a still-locked file.
        apply_key(&mut conn, key)?;
        conn.batch_execute("PRAGMA busy_timeout = 5000; PRAGMA foreign_keys = ON;")
            .map_err(|_| Error::Open)?;
        enable_wal(&mut conn)?;
        Ok(Self {
            path: path.to_path_buf(),
            conn,
        })
    }

    /// The path that was opened.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The journal mode SQLite reports for this connection.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the pragma cannot be read.
    pub fn journal_mode(&mut self) -> Result<String, Error> {
        read_journal_mode(&mut self.conn)
    }
}

/// Hex-encodes `key` into `PRAGMA key` and drops the statement.
///
/// The diesel error is thrown away: its display text can echo the SQL, which
/// would include the key.
fn apply_key(conn: &mut SqliteConnection, key: &[u8]) -> Result<(), Error> {
    let mut pragma = String::from("PRAGMA key = \"x'");
    for byte in key {
        pragma.push(nibble(byte >> 4));
        pragma.push(nibble(byte & 0x0f));
    }
    pragma.push_str("'\";");
    let failed = conn.batch_execute(&pragma).is_err();
    pragma.clear();
    if failed {
        return Err(Error::Open);
    }
    Ok(())
}

fn nibble(value: u8) -> char {
    char::from(if value < 10 {
        b'0' + value
    } else {
        b'a' + (value - 10)
    })
}

/// WAL needs an exclusive lock. SQLite skips the busy handler when two
/// connections already hold a shared lock and both try to upgrade, so the
/// first open of a fresh file can fail at once. Retry inside the same
/// five-second budget `busy_timeout` uses. WAL persists in the file.
fn enable_wal(conn: &mut SqliteConnection) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match conn.batch_execute("PRAGMA journal_mode = WAL;") {
            Ok(()) => return Ok(()),
            Err(diesel::result::Error::DatabaseError(_, info))
                if info.message() == "database is locked" && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return Err(Error::Open),
        }
    }
}

#[derive(QueryableByName)]
struct JournalMode {
    // Diesel's DSL cannot express PRAGMA.
    #[diesel(sql_type = Text)]
    journal_mode: String,
}

fn read_journal_mode(conn: &mut SqliteConnection) -> Result<String, Error> {
    // Diesel's DSL cannot express PRAGMA.
    diesel::sql_query("PRAGMA journal_mode")
        .get_result::<JournalMode>(conn)
        .map(|row| row.journal_mode)
        .map_err(|_| Error::Query)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use diesel::connection::SimpleConnection;
    use diesel::prelude::*;

    use super::Store;

    #[derive(QueryableByName)]
    struct Probe {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        x: i32,
    }

    fn scratch() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("mailune-store-{nanos}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn opens_wal_without_leaking_the_key() {
        let dir = scratch();
        let path = dir.join("mail.db");
        const KEY: &[u8] = b"super-secret-db-key";
        let mut store = Store::open(&path, KEY).unwrap();
        let rendered = format!("{store:?}");
        assert!(!rendered.contains("super-secret-db-key"));
        assert_eq!(store.journal_mode().unwrap(), "wal");

        // A directory is not a database file, so open fails. The error text
        // still must not carry the key.
        let err = Store::open(&dir, KEY).unwrap_err();
        let text = format!("{err} {err:?}");
        assert!(!text.contains("super-secret"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_different_key_cannot_read_the_file() {
        let dir = scratch();
        let path = dir.join("mail.db");
        {
            let mut store = Store::open(&path, b"key-a").unwrap();
            store
                .conn
                .batch_execute(
                    "CREATE TABLE probe(x INTEGER NOT NULL); INSERT INTO probe(x) VALUES (1);",
                )
                .unwrap();
        }
        {
            let mut store = Store::open(&path, b"key-a").unwrap();
            assert_eq!(store.journal_mode().unwrap(), "wal");
            let row: Probe = diesel::sql_query("SELECT x FROM probe")
                .get_result(&mut store.conn)
                .unwrap();
            assert_eq!(row.x, 1);
        }
        let wrong = Store::open(&path, b"key-b").unwrap_err();
        let text = format!("{wrong} {wrong:?}");
        assert!(!text.contains("key-b"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
