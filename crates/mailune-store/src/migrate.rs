//! Embedded schema migrations. The SQL lives in `migrations/`, the only
//! place this crate writes DDL by hand.

use diesel::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::Error;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Brings the file up to the newest schema. Already-applied steps are skipped.
pub(crate) fn run(conn: &mut SqliteConnection) -> Result<(), Error> {
    conn.run_pending_migrations(MIGRATIONS)
        .map(drop)
        .map_err(|err| Error::Migration(err.to_string()))
}

#[cfg(test)]
mod tests {
    use diesel::prelude::*;
    use diesel_migrations::MigrationHarness;

    use crate::Store;
    use crate::schema::{
        accounts, contacts, flags, mailboxes, memberships, messages, ops, parts, sync_state,
        threads,
    };

    #[test]
    fn schema_v1_creates_every_table_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        drop(Store::open(&path, None).unwrap());
        let mut store = Store::open(&path, None).unwrap();
        let conn = &mut store.conn;
        let shipped = diesel::migration::MigrationSource::<diesel::sqlite::Sqlite>::migrations(
            &super::MIGRATIONS,
        )
        .unwrap()
        .len();
        assert_eq!(conn.applied_migrations().unwrap().len(), shipped);
        let counts: [i64; 10] = [
            accounts::table.count().get_result(conn).unwrap(),
            mailboxes::table.count().get_result(conn).unwrap(),
            threads::table.count().get_result(conn).unwrap(),
            messages::table.count().get_result(conn).unwrap(),
            memberships::table.count().get_result(conn).unwrap(),
            parts::table.count().get_result(conn).unwrap(),
            flags::table.count().get_result(conn).unwrap(),
            sync_state::table.count().get_result(conn).unwrap(),
            ops::table.count().get_result(conn).unwrap(),
            contacts::table.count().get_result(conn).unwrap(),
        ];
        assert_eq!(counts, [0; 10]);
    }

    #[test]
    fn every_declared_column_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        let conn = &mut store.conn;
        // Selecting the whole row names every column in schema.rs, so a typo
        // there or in up.sql fails here rather than at the first real query.
        accounts::table
            .select(accounts::all_columns)
            .load::<(String, String, Option<String>, String)>(conn)
            .unwrap();
        messages::table
            .select((messages::id, messages::body_hash, messages::deleted))
            .load::<(String, Option<String>, bool)>(conn)
            .unwrap();
        mailboxes::table
            .select(mailboxes::all_columns)
            .load::<(String, String, String, Option<String>, Option<String>)>(conn)
            .unwrap();
        threads::table
            .select(threads::all_columns)
            .load::<(String, String, String, i64, i32, i32)>(conn)
            .unwrap();
        memberships::table
            .select(memberships::all_columns)
            .load::<(String, String, String, Option<i64>)>(conn)
            .unwrap();
        parts::table
            .select(parts::all_columns)
            .load::<(
                String,
                String,
                String,
                String,
                Option<String>,
                i64,
                Option<String>,
            )>(conn)
            .unwrap();
        flags::table
            .select(flags::all_columns)
            .load::<(String, String, String)>(conn)
            .unwrap();
        sync_state::table
            .select(sync_state::all_columns)
            .load::<(String, String, String)>(conn)
            .unwrap();
        ops::table
            .select(ops::all_columns)
            .load::<(
                i32,
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<i64>,
                i64,
            )>(conn)
            .unwrap();
        contacts::table
            .select(contacts::all_columns)
            .load::<(String, String, Option<String>, i64, i32)>(conn)
            .unwrap();
        messages::table
            .select(messages::all_columns)
            .execute(conn)
            .unwrap();
    }
}
