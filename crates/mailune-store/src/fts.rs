//! Full-text search over subject, addresses, and body.
//!
//! FTS5 virtual tables cannot be expressed in Diesel's DSL, so the statements
//! below use `sql_query` inside this crate.

use diesel::prelude::*;
use diesel::sql_types::Text;

use crate::{Error, Store};

#[derive(QueryableByName)]
struct Hit {
    #[diesel(sql_type = Text)]
    message_id: String,
}

impl Store {
    /// Replaces the indexed text for `message_id`.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the index write fails.
    pub fn index_message(
        &mut self,
        message_id: &str,
        subject: &str,
        addresses: &str,
        body: &str,
    ) -> Result<(), Error> {
        // FTS5 virtual tables cannot be expressed in Diesel's DSL.
        diesel::sql_query("DELETE FROM message_fts WHERE message_id = ?")
            .bind::<Text, _>(message_id)
            .execute(&mut self.conn)
            .map_err(|_| Error::Query)?;
        // FTS5 virtual tables cannot be expressed in Diesel's DSL.
        diesel::sql_query(
            "INSERT INTO message_fts(message_id, subject, addresses, body) VALUES (?, ?, ?, ?)",
        )
        .bind::<Text, _>(message_id)
        .bind::<Text, _>(subject)
        .bind::<Text, _>(addresses)
        .bind::<Text, _>(body)
        .execute(&mut self.conn)
        .map(|_| ())
        .map_err(|_| Error::Query)
    }

    /// Message ids whose subject, addresses, or body match `text`.
    ///
    /// An empty `text` matches nothing. The phrase is quoted so FTS operators
    /// in the caller's text are not interpreted.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the search fails.
    pub fn search_text(&mut self, text: &str) -> Result<Vec<String>, Error> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let phrase = format!("\"{}\"", text.replace('"', " "));
        // FTS5 virtual tables cannot be expressed in Diesel's DSL.
        diesel::sql_query(
            "SELECT message_id FROM message_fts WHERE message_fts MATCH ? ORDER BY rank",
        )
        .bind::<Text, _>(phrase)
        .load::<Hit>(&mut self.conn)
        .map(|rows| rows.into_iter().map(|row| row.message_id).collect())
        .map_err(|_| Error::Query)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Store, scratch_dir};

    #[test]
    fn subject_addresses_and_body_are_searchable() {
        let dir = scratch_dir();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        store
            .index_message("m1", "Invoice", "ada@example.com", "please pay")
            .unwrap();
        store
            .index_message("m2", "Hello", "bob@example.com", "lunch tomorrow")
            .unwrap();
        store
            .index_message("m1", "Invoice", "ada@example.com", "please pay now")
            .unwrap();

        assert_eq!(
            store.search_text("Invoice").unwrap(),
            vec!["m1".to_string()]
        );
        assert_eq!(
            store.search_text("bob@example.com").unwrap(),
            vec!["m2".to_string()]
        );
        assert_eq!(store.search_text("lunch").unwrap(), vec!["m2".to_string()]);
        assert!(store.search_text("missing").unwrap().is_empty());
        assert!(store.search_text("   ").unwrap().is_empty());
        assert_eq!(
            store.search_text("pay now").unwrap(),
            vec!["m1".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
