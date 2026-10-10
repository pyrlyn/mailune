//! Full-text search (S5): FTS5 over subject, addresses and body text, ranked by BM25.
//!
//! Diesel's DSL cannot model an FTS5 virtual table, its `MATCH` operator or `bm25()`, so the two
//! statements that touch `message_fts` go through `sql_query` and stay in this file. Triggers in
//! the migration keep subject and addresses in step with `messages`; body text arrives later,
//! once a body is fetched and decoded, through [`Store::index_body`].

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use mailune_protocol::{AccountId, MessageId};

use crate::open::database_error;
use crate::{Error, Store};

#[derive(QueryableByName)]
struct Hit {
    #[diesel(sql_type = Text)]
    message_id: String,
}

impl Store {
    /// Replaces the indexed body text of a stored message.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownMessage`] when the message is not stored, [`Error::Database`] when the
    /// write fails.
    pub fn index_body(
        &mut self,
        account: &AccountId,
        message: &MessageId,
        text: &str,
    ) -> Result<(), Error> {
        // Diesel cannot model an FTS5 virtual table.
        let updated = diesel::sql_query(
            "UPDATE message_fts SET body = ? WHERE rowid = \
             (SELECT docid FROM search_docs WHERE account_id = ? AND message_id = ?)",
        )
        .bind::<Text, _>(text)
        .bind::<Text, _>(account.as_str())
        .bind::<Text, _>(message.as_str())
        .execute(&mut self.conn)
        .map_err(database_error)?;
        if updated == 0 {
            return Err(Error::UnknownMessage);
        }
        Ok(())
    }

    /// Messages of `account` whose subject, addresses or body hold every word of `text`, best
    /// BM25 match first; ties by message id. A subject hit outranks an address hit, which
    /// outranks a body hit.
    ///
    /// `text` is the user's words, not FTS5 syntax: each word is matched as a quoted phrase, so
    /// `ana@acme.io` finds that address and operators like `OR` or `NEAR` are plain words.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails.
    pub fn search_text(
        &mut self,
        account: &AccountId,
        text: &str,
        limit: usize,
    ) -> Result<Vec<MessageId>, Error> {
        let Some(query) = match_expression(text) else {
            return Ok(Vec::new());
        };
        if limit == 0 {
            return Ok(Vec::new());
        }
        // Diesel cannot model FTS5 `MATCH` or `bm25()`. The weights follow the column order
        // in the migration: subject, addresses, body.
        let hits: Vec<Hit> = diesel::sql_query(
            "SELECT d.message_id AS message_id \
             FROM message_fts JOIN search_docs AS d ON d.docid = message_fts.rowid \
             WHERE message_fts MATCH ? AND d.account_id = ? \
             ORDER BY bm25(message_fts, 4.0, 2.0, 1.0), d.message_id \
             LIMIT ?",
        )
        .bind::<Text, _>(query)
        .bind::<Text, _>(account.as_str())
        .bind::<BigInt, _>(i64::try_from(limit).unwrap_or(i64::MAX))
        .load(&mut self.conn)
        .map_err(database_error)?;
        Ok(hits
            .into_iter()
            .map(|hit| MessageId::new(hit.message_id))
            .collect())
    }
}

/// Every word as a quoted FTS5 phrase, joined by the implicit AND; `None` when no word can
/// match. A word without a letter or digit tokenizes to nothing, which FTS5 would reject.
fn match_expression(text: &str) -> Option<String> {
    let phrases: Vec<String> = text
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect();
    (!phrases.is_empty()).then(|| phrases.join(" "))
}

#[cfg(test)]
mod tests {
    use diesel::prelude::*;
    use diesel_migrations::MigrationHarness;
    use mailune_protocol::{AccountId, Address, MessageId};

    use super::match_expression;
    use crate::migrate::MIGRATIONS;
    use crate::schema::{messages, threads};
    use crate::testutil::{account, message, seeded};
    use crate::{Account, Error, Mailbox, Store, StoredMessage};

    const FTS_MIGRATION: &str = "20261010000004";

    fn ids(store: &mut Store, text: &str) -> Vec<String> {
        store
            .search_text(&account(), text, 10)
            .unwrap()
            .into_iter()
            .map(|id| id.as_str().to_owned())
            .collect()
    }

    fn with(id: &str, subject: &str, to: &str) -> StoredMessage {
        let mut stored = message(id, id, 1, false);
        stored.envelope.subject = subject.into();
        stored.envelope.to = vec![Address {
            name: Some("Bea Lindqvist".into()),
            email: to.into(),
        }];
        stored
    }

    #[test]
    fn subject_addresses_and_body_are_searchable() {
        let (_dir, mut store) = seeded();
        store
            .upsert_message(&with("m1", "Quarterly report", "bea@acme.io"))
            .unwrap();
        store
            .upsert_message(&with("m2", "Lunch", "carl@north.example"))
            .unwrap();
        store
            .index_body(
                &account(),
                &MessageId::new("m2"),
                "The invoice is attached.",
            )
            .unwrap();

        assert_eq!(ids(&mut store, "quarterly"), ["m1"]);
        assert_eq!(ids(&mut store, "bea@acme.io"), ["m1"]);
        assert_eq!(ids(&mut store, "lindqvist"), ["m1", "m2"]);
        assert_eq!(ids(&mut store, "INVOICE"), ["m2"]);
        assert_eq!(ids(&mut store, "invoice lunch"), ["m2"]);
        assert!(ids(&mut store, "invoice quarterly").is_empty());
        // The JSON keys of the recipient list are not words of the message.
        assert!(ids(&mut store, "email").is_empty());
    }

    #[test]
    fn a_subject_hit_ranks_above_a_body_hit() {
        let (_dir, mut store) = seeded();
        store
            .upsert_message(&with("a", "Lunch", "x@acme.io"))
            .unwrap();
        store
            .upsert_message(&with("b", "Budget review", "x@acme.io"))
            .unwrap();
        store
            .index_body(&account(), &MessageId::new("a"), "about the budget")
            .unwrap();
        assert_eq!(ids(&mut store, "budget"), ["b", "a"]);
    }

    #[test]
    fn the_index_follows_updates_and_deletes() {
        let (_dir, mut store) = seeded();
        let mut stored = with("m1", "Draft agenda", "bea@acme.io");
        store.upsert_message(&stored).unwrap();
        store
            .index_body(&account(), &MessageId::new("m1"), "see you there")
            .unwrap();

        stored.envelope.subject = "Final agenda".into();
        store.upsert_message(&stored).unwrap();
        assert!(ids(&mut store, "draft").is_empty());
        assert_eq!(ids(&mut store, "final"), ["m1"]);
        // A subject change leaves the body text alone.
        assert_eq!(ids(&mut store, "there"), ["m1"]);

        // Dropping the thread cascades to its messages, and the index follows.
        diesel::delete(threads::table)
            .execute(&mut store.conn)
            .unwrap();
        assert_eq!(
            messages::table
                .count()
                .get_result::<i64>(&mut store.conn)
                .unwrap(),
            0
        );
        assert!(ids(&mut store, "agenda").is_empty());
        assert!(ids(&mut store, "there").is_empty());
    }

    #[test]
    fn search_is_scoped_to_one_account() {
        let (_dir, mut store) = seeded();
        let other = AccountId::new("other");
        store
            .upsert_account(&Account {
                id: other.clone(),
                email: "o@example.com".into(),
                display_name: None,
                provider: "imap".into(),
            })
            .unwrap();
        store
            .upsert_mailbox(&Mailbox {
                account: other.clone(),
                id: crate::testutil::inbox(),
                name: "Inbox".into(),
                role: None,
                parent: None,
            })
            .unwrap();
        store
            .upsert_message(&with("m1", "Shared word", "a@acme.io"))
            .unwrap();
        let mut theirs = with("m1", "Shared word", "a@acme.io");
        theirs.account = other.clone();
        store.upsert_message(&theirs).unwrap();
        assert_eq!(ids(&mut store, "shared"), ["m1"]);
        assert_eq!(store.search_text(&other, "shared", 10).unwrap().len(), 1);
    }

    #[test]
    fn fts_syntax_in_the_query_is_plain_text() {
        let (_dir, mut store) = seeded();
        store
            .upsert_message(&with("m1", "NEAR the \"OR\" gate", "a@acme.io"))
            .unwrap();
        for text in [
            "near",
            "OR",
            "\"or\"",
            "gate*",
            "subject:gate",
            "NEAR(gate",
            "-gate",
        ] {
            assert!(store.search_text(&account(), text, 10).is_ok(), "{text}");
        }
        assert_eq!(ids(&mut store, "OR"), ["m1"]);
        assert!(ids(&mut store, "@@ ** ()").is_empty());
        assert!(ids(&mut store, "   ").is_empty());
        assert!(store.search_text(&account(), "gate", 0).unwrap().is_empty());
    }

    #[test]
    fn a_body_for_an_unknown_message_is_refused() {
        let (_dir, mut store) = seeded();
        assert!(matches!(
            store.index_body(&account(), &MessageId::new("nope"), "text"),
            Err(Error::UnknownMessage)
        ));
    }

    #[test]
    fn messages_stored_before_the_index_existed_are_backfilled() {
        let (dir, mut store) = seeded();
        store
            .upsert_message(&with("m1", "Old news", "a@acme.io"))
            .unwrap();
        // Rewind to the schema before S5; reopening runs its migration over the stored rows.
        while store
            .conn
            .applied_migrations()
            .unwrap()
            .iter()
            .any(|version| version.to_string() == FTS_MIGRATION)
        {
            store.conn.revert_last_migration(MIGRATIONS).unwrap();
        }
        drop(store);
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        assert_eq!(ids(&mut store, "news"), ["m1"]);
    }

    #[test]
    fn the_match_expression_quotes_every_word() {
        assert_eq!(
            match_expression(" a \"b\"  c-d ").as_deref(),
            Some("\"a\" \"\"\"b\"\"\" \"c-d\"")
        );
        assert_eq!(match_expression("-- !!"), None);
    }
}
