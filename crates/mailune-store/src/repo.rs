//! Typed repository queries.
//!
//! Writes are upserts so a sync can repeat a row. Reads stay on Diesel's DSL.

use diesel::prelude::*;
use diesel::sqlite::Sqlite;

use crate::schema::{
    accounts, contacts, flags, mailboxes, memberships, messages, parts, sync_state, threads,
};
use crate::{Error, Store};

/// One account row.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = accounts)]
#[diesel(primary_key(id))]
pub struct AccountRow {
    /// Provider account id.
    pub id: String,
    /// Mailbox address for the account.
    pub email: String,
}

/// One mailbox row.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = mailboxes)]
#[diesel(primary_key(id))]
pub struct MailboxRow {
    /// Mailbox id.
    pub id: String,
    /// Owning account.
    pub account_id: String,
    /// Display name.
    pub name: String,
    /// Special-use role, when the server has one.
    pub role: Option<String>,
}

/// One conversation row.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = threads)]
#[diesel(primary_key(id))]
pub struct ThreadRow {
    /// Thread id.
    pub id: String,
    /// Owning account.
    pub account_id: String,
    /// Thread subject.
    pub subject: String,
}

/// One message row.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = messages)]
#[diesel(primary_key(id))]
pub struct MessageRow {
    /// Message id.
    pub id: String,
    /// Owning account.
    pub account_id: String,
    /// Conversation it belongs to.
    pub thread_id: String,
    /// Message subject.
    pub subject: String,
    /// `From` address.
    pub from_email: String,
    /// `To` addresses, one per line.
    pub to_emails: String,
    /// Display stamp.
    pub stamp: String,
    /// Sort key for cursor paging, milliseconds.
    pub received_at: i64,
}

/// A message sitting in a mailbox.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable)]
#[diesel(table_name = memberships)]
pub struct MembershipRow {
    /// Message id.
    pub message_id: String,
    /// Mailbox id.
    pub mailbox_id: String,
}

/// One MIME part's text.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = parts)]
#[diesel(primary_key(message_id, ordinal))]
pub struct PartRow {
    /// Owning message.
    pub message_id: String,
    /// Part order inside the message.
    pub ordinal: i32,
    /// MIME type.
    pub content_type: String,
    /// Decoded text. Bytes live in the blob store.
    pub body: String,
}

/// Flags stored for one message.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = flags)]
#[diesel(primary_key(message_id))]
pub struct FlagRow {
    /// Message id.
    pub message_id: String,
    /// `\Seen`.
    pub seen: bool,
    /// `\Flagged`.
    pub flagged: bool,
    /// `\Draft`.
    pub draft: bool,
    /// `\Answered`.
    pub answered: bool,
    /// `\Deleted`.
    pub deleted: bool,
    /// Keywords, one per line.
    pub keywords: String,
}

/// Incremental sync cursor for one mailbox.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sync_state)]
#[diesel(primary_key(account_id, mailbox_id))]
pub struct SyncStateRow {
    /// Account being synced.
    pub account_id: String,
    /// Mailbox being synced.
    pub mailbox_id: String,
    /// Provider cursor.
    pub token: String,
}

/// One address book entry.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = contacts)]
#[diesel(primary_key(account_id, email))]
pub struct ContactRow {
    /// Owning account.
    pub account_id: String,
    /// Contact address.
    pub email: String,
    /// Display name.
    pub name: String,
}

/// Position in [`Store::page_messages`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageCursor {
    /// `received_at` of the last row already returned.
    pub received_at: i64,
    /// Id of that row, so equal timestamps stay ordered.
    pub id: String,
}

/// One page of messages, oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagePage {
    /// Rows in this page.
    pub messages: Vec<MessageRow>,
    /// Present when the page was full. The following page may be empty.
    pub next: Option<MessageCursor>,
}

impl Store {
    /// Inserts or updates an account.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_account(&mut self, row: &AccountRow) -> Result<(), Error> {
        diesel::insert_into(accounts::table)
            .values(row)
            .on_conflict(accounts::id)
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// The account, if it has been stored.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the read fails.
    pub fn account(&mut self, id: &str) -> Result<Option<AccountRow>, Error> {
        accounts::table
            .filter(accounts::id.eq(id))
            .first(&mut self.conn)
            .optional()
            .map_err(|_| Error::Query)
    }

    /// Inserts or updates a mailbox.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_mailbox(&mut self, row: &MailboxRow) -> Result<(), Error> {
        diesel::insert_into(mailboxes::table)
            .values(row)
            .on_conflict(mailboxes::id)
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Inserts or updates a thread.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_thread(&mut self, row: &ThreadRow) -> Result<(), Error> {
        diesel::insert_into(threads::table)
            .values(row)
            .on_conflict(threads::id)
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Inserts or updates a message.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_message(&mut self, row: &MessageRow) -> Result<(), Error> {
        diesel::insert_into(messages::table)
            .values(row)
            .on_conflict(messages::id)
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Records that a message is in a mailbox. Repeating the pair is a no-op.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_membership(&mut self, row: &MembershipRow) -> Result<(), Error> {
        diesel::insert_into(memberships::table)
            .values(row)
            .on_conflict((memberships::message_id, memberships::mailbox_id))
            .do_nothing()
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Inserts or updates a MIME part.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_part(&mut self, row: &PartRow) -> Result<(), Error> {
        diesel::insert_into(parts::table)
            .values(row)
            .on_conflict((parts::message_id, parts::ordinal))
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Inserts or updates flags.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_flags(&mut self, row: &FlagRow) -> Result<(), Error> {
        diesel::insert_into(flags::table)
            .values(row)
            .on_conflict(flags::message_id)
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Inserts or updates a sync cursor.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_sync_state(&mut self, row: &SyncStateRow) -> Result<(), Error> {
        diesel::insert_into(sync_state::table)
            .values(row)
            .on_conflict((sync_state::account_id, sync_state::mailbox_id))
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Inserts or updates a contact.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails.
    pub fn upsert_contact(&mut self, row: &ContactRow) -> Result<(), Error> {
        diesel::insert_into(contacts::table)
            .values(row)
            .on_conflict((contacts::account_id, contacts::email))
            .do_update()
            .set(row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// Messages in `thread_id`, oldest first.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the read fails.
    pub fn messages_in_thread(&mut self, thread_id: &str) -> Result<Vec<MessageRow>, Error> {
        messages::table
            .filter(messages::thread_id.eq(thread_id))
            .order(messages::received_at.asc())
            .load(&mut self.conn)
            .map_err(|_| Error::Query)
    }

    /// Messages after `after`, oldest first, at most `limit` rows.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the read fails.
    pub fn page_messages(
        &mut self,
        after: Option<&MessageCursor>,
        limit: i64,
    ) -> Result<MessagePage, Error> {
        let mut query = messages::table.into_boxed::<Sqlite>();
        if let Some(cursor) = after {
            query = query.filter(
                messages::received_at
                    .gt(cursor.received_at)
                    .or(messages::received_at
                        .eq(cursor.received_at)
                        .and(messages::id.gt(&cursor.id))),
            );
        }
        let messages: Vec<MessageRow> = query
            .order((messages::received_at.asc(), messages::id.asc()))
            .limit(limit)
            .load(&mut self.conn)
            .map_err(|_| Error::Query)?;
        let full = i64::try_from(messages.len()).ok() == Some(limit);
        let next = full.then(|| {
            messages.last().map(|row| MessageCursor {
                received_at: row.received_at,
                id: row.id.clone(),
            })
        });
        Ok(MessagePage {
            messages,
            next: next.flatten(),
        })
    }

    /// How many messages are stored.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the read fails.
    pub fn count_messages(&mut self) -> Result<i64, Error> {
        messages::table
            .count()
            .get_result(&mut self.conn)
            .map_err(|_| Error::Query)
    }

    /// How many threads are stored.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the read fails.
    pub fn count_threads(&mut self) -> Result<i64, Error> {
        threads::table
            .count()
            .get_result(&mut self.conn)
            .map_err(|_| Error::Query)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AccountRow, ContactRow, FlagRow, MailboxRow, MembershipRow, MessageRow, PartRow,
        SyncStateRow, ThreadRow,
    };
    use crate::{Store, scratch_dir};

    fn message(id: &str, at: i64) -> MessageRow {
        MessageRow {
            id: id.to_string(),
            account_id: "a".into(),
            thread_id: "t".into(),
            subject: "Hello".into(),
            from_email: "ada@example.com".into(),
            to_emails: "bob@example.com".into(),
            stamp: "Mon".into(),
            received_at: at,
        }
    }

    #[test]
    fn upserts_page_and_count_a_thread() {
        let dir = scratch_dir();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        store
            .upsert_account(&AccountRow {
                id: "a".into(),
                email: "old@example.com".into(),
            })
            .unwrap();
        store
            .upsert_account(&AccountRow {
                id: "a".into(),
                email: "new@example.com".into(),
            })
            .unwrap();
        assert_eq!(
            store.account("a").unwrap().unwrap().email,
            "new@example.com"
        );
        store
            .upsert_mailbox(&MailboxRow {
                id: "inbox".into(),
                account_id: "a".into(),
                name: "Inbox".into(),
                role: Some("inbox".into()),
            })
            .unwrap();
        store
            .upsert_thread(&ThreadRow {
                id: "t".into(),
                account_id: "a".into(),
                subject: "Hello".into(),
            })
            .unwrap();
        store.upsert_message(&message("m1", 10)).unwrap();
        store.upsert_message(&message("m2", 20)).unwrap();
        store
            .upsert_membership(&MembershipRow {
                message_id: "m1".into(),
                mailbox_id: "inbox".into(),
            })
            .unwrap();
        store
            .upsert_membership(&MembershipRow {
                message_id: "m1".into(),
                mailbox_id: "inbox".into(),
            })
            .unwrap();
        store
            .upsert_part(&PartRow {
                message_id: "m1".into(),
                ordinal: 0,
                content_type: "text/plain".into(),
                body: "body text".into(),
            })
            .unwrap();
        store
            .upsert_flags(&FlagRow {
                message_id: "m1".into(),
                seen: false,
                flagged: true,
                draft: false,
                answered: false,
                deleted: false,
                keywords: "work".into(),
            })
            .unwrap();
        store
            .upsert_sync_state(&SyncStateRow {
                account_id: "a".into(),
                mailbox_id: "inbox".into(),
                token: "1".into(),
            })
            .unwrap();
        store
            .upsert_contact(&ContactRow {
                account_id: "a".into(),
                email: "ada@example.com".into(),
                name: "Ada".into(),
            })
            .unwrap();

        let thread = store.messages_in_thread("t").unwrap();
        assert_eq!(thread.len(), 2);
        assert_eq!(thread[0].id, "m1");
        assert_eq!(store.count_messages().unwrap(), 2);
        assert_eq!(store.count_threads().unwrap(), 1);

        let first = store.page_messages(None, 1).unwrap();
        assert_eq!(first.messages[0].id, "m1");
        let cursor = first.next.unwrap();
        let second = store.page_messages(Some(&cursor), 1).unwrap();
        assert_eq!(second.messages[0].id, "m2");
        let tail = store.page_messages(second.next.as_ref(), 1).unwrap();
        assert!(tail.messages.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
