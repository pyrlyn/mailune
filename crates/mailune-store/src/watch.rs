//! Change feed for one database file.
//!
//! `PRAGMA data_version` advances when a different connection commits, and
//! stays put for this connection's own commits. Diesel cannot model a PRAGMA,
//! so the read is `sql_query`.

use diesel::prelude::*;
use diesel::sql_types::BigInt;

use crate::{Error, Store};

/// The `data_version` a connection has already observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeToken(i64);

/// A commit made through some other connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalidation {
    /// Tables may have changed. The pragma does not name which ones.
    Committed {
        /// The version now visible to this connection.
        version: i64,
    },
}

#[derive(QueryableByName)]
struct DataVersion {
    #[diesel(sql_type = BigInt)]
    data_version: i64,
}

fn data_version(conn: &mut diesel::SqliteConnection) -> Result<i64, Error> {
    // Diesel cannot model PRAGMA data_version.
    diesel::sql_query("PRAGMA data_version")
        .get_result::<DataVersion>(conn)
        .map(|row| row.data_version)
        .map_err(|_| Error::Query)
}

impl Store {
    /// The feed position for this connection.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the pragma cannot be read.
    pub fn change_token(&mut self) -> Result<ChangeToken, Error> {
        data_version(&mut self.conn).map(ChangeToken)
    }

    /// A commit from another connection since `since`, if one landed.
    ///
    /// Advances `since` when it reports a change. This connection's own
    /// writes do not.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the pragma cannot be read.
    pub fn poll_changes(&mut self, since: &mut ChangeToken) -> Result<Option<Invalidation>, Error> {
        let now = data_version(&mut self.conn)?;
        if now == since.0 {
            return Ok(None);
        }
        since.0 = now;
        Ok(Some(Invalidation::Committed { version: now }))
    }
}

#[cfg(test)]
mod tests {
    use crate::repo::AccountRow;
    use crate::{Invalidation, Store, scratch_dir};

    #[test]
    fn a_second_connection_sees_a_commit() {
        let dir = scratch_dir();
        let path = dir.join("mail.db");
        let mut writer = Store::open(&path, b"key").unwrap();
        let mut reader = Store::open(&path, b"key").unwrap();
        let mut reader_token = reader.change_token().unwrap();
        let mut writer_token = writer.change_token().unwrap();

        writer
            .upsert_account(&AccountRow {
                id: "a".into(),
                email: "ada@example.com".into(),
            })
            .unwrap();

        assert!(writer.poll_changes(&mut writer_token).unwrap().is_none());
        assert!(matches!(
            reader.poll_changes(&mut reader_token).unwrap(),
            Some(Invalidation::Committed { .. })
        ));
        assert!(reader.poll_changes(&mut reader_token).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
