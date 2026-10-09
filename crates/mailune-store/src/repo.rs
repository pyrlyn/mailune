//! Repository API: upserts, the thread list page, a thread's messages, and
//! mailbox counts. Everything goes through Diesel's typed DSL.

use diesel::dsl::max;
use diesel::prelude::*;
use mailune_protocol::{
    AccountId, Address, Envelope, Flags, MailboxId, MailboxRole, MessageId, ThreadId,
    TransportSecurity,
};

use crate::open::database_error;
use crate::schema::{accounts, flags, mailboxes, memberships, messages, sync_state, threads};
use crate::{Error, Store};

/// One configured account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Account id.
    pub id: AccountId,
    /// Primary address.
    pub email: String,
    /// Name shown in the sidebar.
    pub display_name: Option<String>,
    /// Provider kind (`imap`, `jmap`, `gmail`, `graph`).
    pub provider: String,
}

/// One mailbox (folder or label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mailbox {
    /// Owning account.
    pub account: AccountId,
    /// Provider mailbox id.
    pub id: MailboxId,
    /// Display name from the server.
    pub name: String,
    /// Special-use role, if any.
    pub role: Option<MailboxRole>,
    /// Parent mailbox for a nested folder.
    pub parent: Option<MailboxId>,
}

/// A message to store, with the facts the envelope does not carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMessage {
    /// Owning account.
    pub account: AccountId,
    /// Headers and flags.
    pub envelope: Envelope,
    /// Arrival time in seconds since the Unix epoch. Orders the list.
    pub received_at: i64,
    /// Every mailbox the message is in. Replaces the previous set.
    pub mailboxes: Vec<MailboxId>,
}

/// One row of the thread list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadSummary {
    /// Conversation id.
    pub id: ThreadId,
    /// Subject of the oldest message.
    pub subject: String,
    /// Newest arrival in the conversation, seconds since the epoch.
    pub latest_at: i64,
    /// Messages in the conversation.
    pub message_count: u32,
    /// Messages without `\Seen`.
    pub unread_count: u32,
}

/// Where the next page starts: after this row in newest-first order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// `latest_at` of the last row returned.
    pub latest_at: i64,
    /// Id of the last row returned; breaks ties on `latest_at`.
    pub thread: ThreadId,
}

/// A page of the thread list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadPage {
    /// Rows, newest first.
    pub threads: Vec<ThreadSummary>,
    /// Cursor for the next page, or `None` on the last page.
    pub next: Option<Cursor>,
}

/// Message counts for one mailbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Messages in the mailbox.
    pub total: u64,
    /// Of those, messages without `\Seen`.
    pub unread: u64,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = accounts, treat_none_as_null = true)]
struct AccountRow<'a> {
    id: &'a str,
    email: &'a str,
    display_name: Option<&'a str>,
    provider: &'a str,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = mailboxes, primary_key(account_id, id), treat_none_as_null = true)]
struct MailboxRow<'a> {
    account_id: &'a str,
    id: &'a str,
    name: &'a str,
    role: Option<&'static str>,
    parent_id: Option<&'a str>,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = messages, primary_key(account_id, id), treat_none_as_null = true)]
struct MessageRow<'a> {
    account_id: &'a str,
    id: &'a str,
    thread_id: &'a str,
    from_name: Option<&'a str>,
    from_email: &'a str,
    to_addrs: String,
    cc_addrs: String,
    subject: &'a str,
    snippet: &'a str,
    stamp: &'a str,
    received_at: i64,
    attachment_count: i32,
    transport: &'static str,
    seen: bool,
    flagged: bool,
    draft: bool,
    answered: bool,
    deleted: bool,
}

#[derive(Queryable)]
struct MessageOut {
    id: String,
    thread_id: String,
    from_name: Option<String>,
    from_email: String,
    to_addrs: String,
    cc_addrs: String,
    subject: String,
    snippet: String,
    stamp: String,
    attachment_count: i32,
    transport: String,
    seen: bool,
    flagged: bool,
    draft: bool,
    answered: bool,
    deleted: bool,
}

impl Store {
    /// Inserts or replaces an account.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the write fails.
    pub fn upsert_account(&mut self, account: &Account) -> Result<(), Error> {
        let row = AccountRow {
            id: account.id.as_str(),
            email: &account.email,
            display_name: account.display_name.as_deref(),
            provider: &account.provider,
        };
        diesel::insert_into(accounts::table)
            .values(&row)
            .on_conflict(accounts::id)
            .do_update()
            .set(&row)
            .execute(&mut self.conn)
            .map(drop)
            .map_err(database_error)
    }

    /// Inserts or replaces a mailbox.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the write fails or the account is unknown.
    pub fn upsert_mailbox(&mut self, mailbox: &Mailbox) -> Result<(), Error> {
        let row = MailboxRow {
            account_id: mailbox.account.as_str(),
            id: mailbox.id.as_str(),
            name: &mailbox.name,
            role: mailbox.role.map(role_name),
            parent_id: mailbox.parent.as_ref().map(MailboxId::as_str),
        };
        diesel::insert_into(mailboxes::table)
            .values(&row)
            .on_conflict((mailboxes::account_id, mailboxes::id))
            .do_update()
            .set(&row)
            .execute(&mut self.conn)
            .map(drop)
            .map_err(database_error)
    }

    /// Inserts or replaces a message, its mailboxes and keywords, and
    /// refreshes the thread row it belongs to (and the one it left).
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when a write fails or a mailbox is unknown.
    pub fn upsert_message(&mut self, message: &StoredMessage) -> Result<(), Error> {
        let account = message.account.as_str();
        let envelope = &message.envelope;
        let row = MessageRow {
            account_id: account,
            id: envelope.id.as_str(),
            thread_id: envelope.thread.as_str(),
            from_name: envelope.from.name.as_deref(),
            from_email: &envelope.from.email,
            to_addrs: addresses_json(&envelope.to)?,
            cc_addrs: addresses_json(&envelope.cc)?,
            subject: &envelope.subject,
            snippet: &envelope.snippet,
            stamp: &envelope.stamp,
            received_at: message.received_at,
            attachment_count: i32::try_from(envelope.attachment_count).unwrap_or(i32::MAX),
            transport: transport_name(envelope.transport),
            seen: envelope.flags.seen,
            flagged: envelope.flags.flagged,
            draft: envelope.flags.draft,
            answered: envelope.flags.answered,
            deleted: envelope.flags.deleted,
        };
        self.conn
            .transaction(|conn| {
                let previous: Option<String> = messages::table
                    .filter(messages::account_id.eq(account))
                    .filter(messages::id.eq(row.id))
                    .select(messages::thread_id)
                    .first(conn)
                    .optional()?;
                diesel::insert_into(threads::table)
                    .values((
                        threads::account_id.eq(account),
                        threads::id.eq(row.thread_id),
                        threads::subject.eq(row.subject),
                        threads::latest_at.eq(row.received_at),
                        threads::message_count.eq(0),
                        threads::unread_count.eq(0),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)?;
                diesel::insert_into(messages::table)
                    .values(&row)
                    .on_conflict((messages::account_id, messages::id))
                    .do_update()
                    .set(&row)
                    .execute(conn)?;
                replace_memberships(conn, account, row.id, &message.mailboxes)?;
                replace_keywords(conn, account, row.id, &envelope.flags.keywords)?;
                refresh_thread(conn, account, row.thread_id)?;
                if let Some(old) = previous.filter(|old| old != row.thread_id) {
                    refresh_thread(conn, account, &old)?;
                }
                Ok(())
            })
            .map_err(database_error)
    }

    /// One page of threads that have a message in `mailbox`, newest first.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails.
    pub fn thread_page(
        &mut self,
        account: &AccountId,
        mailbox: &MailboxId,
        after: Option<&Cursor>,
        limit: usize,
    ) -> Result<ThreadPage, Error> {
        let in_mailbox = messages::table
            .inner_join(
                memberships::table.on(memberships::account_id
                    .eq(messages::account_id)
                    .and(memberships::message_id.eq(messages::id))),
            )
            .filter(messages::account_id.eq(account.as_str()))
            .filter(memberships::mailbox_id.eq(mailbox.as_str()))
            .select(messages::thread_id);
        let mut query = threads::table
            .filter(threads::account_id.eq(account.as_str()))
            .filter(threads::id.eq_any(in_mailbox))
            .into_boxed();
        if let Some(cursor) = after {
            query = query.filter(
                threads::latest_at
                    .lt(cursor.latest_at)
                    .or(threads::latest_at
                        .eq(cursor.latest_at)
                        .and(threads::id.lt(cursor.thread.as_str().to_string()))),
            );
        }
        let fetch = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
        let rows: Vec<(String, String, i64, i32, i32)> = query
            .order((threads::latest_at.desc(), threads::id.desc()))
            .limit(fetch)
            .select((
                threads::id,
                threads::subject,
                threads::latest_at,
                threads::message_count,
                threads::unread_count,
            ))
            .load(&mut self.conn)
            .map_err(database_error)?;
        let more = rows.len() > limit;
        let threads: Vec<ThreadSummary> = rows
            .into_iter()
            .take(limit)
            .map(|(id, subject, latest_at, count, unread)| ThreadSummary {
                id: ThreadId::new(id),
                subject,
                latest_at,
                message_count: u32::try_from(count).unwrap_or(0),
                unread_count: u32::try_from(unread).unwrap_or(0),
            })
            .collect();
        let next = if more {
            threads.last().map(|row| Cursor {
                latest_at: row.latest_at,
                thread: row.id.clone(),
            })
        } else {
            None
        };
        Ok(ThreadPage { threads, next })
    }

    /// Messages of one thread, oldest first.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails or a stored address list is
    /// not valid JSON.
    pub fn thread_messages(
        &mut self,
        account: &AccountId,
        thread: &ThreadId,
    ) -> Result<Vec<Envelope>, Error> {
        let rows: Vec<MessageOut> = messages::table
            .filter(messages::account_id.eq(account.as_str()))
            .filter(messages::thread_id.eq(thread.as_str()))
            .order((messages::received_at.asc(), messages::id.asc()))
            .select((
                messages::id,
                messages::thread_id,
                messages::from_name,
                messages::from_email,
                messages::to_addrs,
                messages::cc_addrs,
                messages::subject,
                messages::snippet,
                messages::stamp,
                messages::attachment_count,
                messages::transport,
                messages::seen,
                messages::flagged,
                messages::draft,
                messages::answered,
                messages::deleted,
            ))
            .load(&mut self.conn)
            .map_err(database_error)?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let keywords: Vec<String> = flags::table
                .filter(flags::account_id.eq(account.as_str()))
                .filter(flags::message_id.eq(&row.id))
                .order(flags::keyword.asc())
                .select(flags::keyword)
                .load(&mut self.conn)
                .map_err(database_error)?;
            out.push(envelope_from(row, keywords)?);
        }
        Ok(out)
    }

    /// Total and unread messages in one mailbox.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails.
    pub fn mailbox_counts(
        &mut self,
        account: &AccountId,
        mailbox: &MailboxId,
    ) -> Result<Counts, Error> {
        let base = || {
            messages::table
                .inner_join(
                    memberships::table.on(memberships::account_id
                        .eq(messages::account_id)
                        .and(memberships::message_id.eq(messages::id))),
                )
                .filter(messages::account_id.eq(account.as_str()))
                .filter(memberships::mailbox_id.eq(mailbox.as_str()))
        };
        let total: i64 = base()
            .count()
            .get_result(&mut self.conn)
            .map_err(database_error)?;
        let unread: i64 = base()
            .filter(messages::seen.eq(false))
            .count()
            .get_result(&mut self.conn)
            .map_err(database_error)?;
        Ok(Counts {
            total: u64::try_from(total).unwrap_or(0),
            unread: u64::try_from(unread).unwrap_or(0),
        })
    }

    /// Saves the opaque sync cursor for one scope (a JMAP state, a Gmail
    /// historyId, a Graph deltaLink).
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the write fails or the account is unknown.
    pub fn set_sync_state(
        &mut self,
        account: &AccountId,
        scope: &str,
        state: &str,
    ) -> Result<(), Error> {
        diesel::insert_into(sync_state::table)
            .values((
                sync_state::account_id.eq(account.as_str()),
                sync_state::scope.eq(scope),
                sync_state::state.eq(state),
            ))
            .on_conflict((sync_state::account_id, sync_state::scope))
            .do_update()
            .set(sync_state::state.eq(state))
            .execute(&mut self.conn)
            .map(drop)
            .map_err(database_error)
    }

    /// The sync cursor saved for `scope`, if any.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails.
    pub fn sync_state(
        &mut self,
        account: &AccountId,
        scope: &str,
    ) -> Result<Option<String>, Error> {
        sync_state::table
            .filter(sync_state::account_id.eq(account.as_str()))
            .filter(sync_state::scope.eq(scope))
            .select(sync_state::state)
            .first(&mut self.conn)
            .optional()
            .map_err(database_error)
    }
}

fn replace_memberships(
    conn: &mut SqliteConnection,
    account: &str,
    message: &str,
    boxes: &[MailboxId],
) -> QueryResult<()> {
    diesel::delete(
        memberships::table
            .filter(memberships::account_id.eq(account))
            .filter(memberships::message_id.eq(message)),
    )
    .execute(conn)?;
    let rows: Vec<_> = boxes
        .iter()
        .map(|mailbox| {
            (
                memberships::account_id.eq(account),
                memberships::message_id.eq(message),
                memberships::mailbox_id.eq(mailbox.as_str()),
            )
        })
        .collect();
    diesel::insert_into(memberships::table)
        .values(&rows)
        .on_conflict_do_nothing()
        .execute(conn)
        .map(drop)
}

fn replace_keywords(
    conn: &mut SqliteConnection,
    account: &str,
    message: &str,
    keywords: &[String],
) -> QueryResult<()> {
    diesel::delete(
        flags::table
            .filter(flags::account_id.eq(account))
            .filter(flags::message_id.eq(message)),
    )
    .execute(conn)?;
    let rows: Vec<_> = keywords
        .iter()
        .map(|keyword| {
            (
                flags::account_id.eq(account),
                flags::message_id.eq(message),
                flags::keyword.eq(keyword.as_str()),
            )
        })
        .collect();
    diesel::insert_into(flags::table)
        .values(&rows)
        .on_conflict_do_nothing()
        .execute(conn)
        .map(drop)
}

/// Recomputes a thread's counts, newest stamp and subject from its
/// messages, and drops the row once the last message has left.
fn refresh_thread(conn: &mut SqliteConnection, account: &str, thread: &str) -> QueryResult<()> {
    let in_thread = || {
        messages::table
            .filter(messages::account_id.eq(account))
            .filter(messages::thread_id.eq(thread))
    };
    let count: i64 = in_thread().count().get_result(conn)?;
    let row = threads::table
        .filter(threads::account_id.eq(account))
        .filter(threads::id.eq(thread));
    if count == 0 {
        return diesel::delete(row).execute(conn).map(drop);
    }
    let unread: i64 = in_thread()
        .filter(messages::seen.eq(false))
        .count()
        .get_result(conn)?;
    let latest: Option<i64> = in_thread().select(max(messages::received_at)).first(conn)?;
    let subject: String = in_thread()
        .order((messages::received_at.asc(), messages::id.asc()))
        .select(messages::subject)
        .first(conn)?;
    diesel::update(row)
        .set((
            threads::subject.eq(subject),
            threads::latest_at.eq(latest.unwrap_or(0)),
            threads::message_count.eq(i32::try_from(count).unwrap_or(i32::MAX)),
            threads::unread_count.eq(i32::try_from(unread).unwrap_or(i32::MAX)),
        ))
        .execute(conn)
        .map(drop)
}

fn addresses_json(list: &[Address]) -> Result<String, Error> {
    serde_json::to_string(list).map_err(|err| Error::Database(err.to_string()))
}

fn addresses_from(json: &str) -> Result<Vec<Address>, Error> {
    serde_json::from_str(json).map_err(|err| Error::Database(err.to_string()))
}

fn envelope_from(row: MessageOut, keywords: Vec<String>) -> Result<Envelope, Error> {
    Ok(Envelope {
        id: MessageId::new(row.id),
        thread: ThreadId::new(row.thread_id),
        from: Address {
            name: row.from_name,
            email: row.from_email,
        },
        to: addresses_from(&row.to_addrs)?,
        cc: addresses_from(&row.cc_addrs)?,
        subject: row.subject,
        stamp: row.stamp,
        snippet: row.snippet,
        flags: Flags {
            seen: row.seen,
            flagged: row.flagged,
            draft: row.draft,
            answered: row.answered,
            deleted: row.deleted,
            keywords,
        },
        attachment_count: u32::try_from(row.attachment_count).unwrap_or(0),
        transport: if row.transport == "tls" {
            TransportSecurity::Tls
        } else {
            TransportSecurity::Clear
        },
    })
}

fn transport_name(transport: TransportSecurity) -> &'static str {
    match transport {
        TransportSecurity::Tls => "tls",
        TransportSecurity::Clear => "clear",
    }
}

fn role_name(role: MailboxRole) -> &'static str {
    match role {
        MailboxRole::Inbox => "inbox",
        MailboxRole::Starred => "starred",
        MailboxRole::Snoozed => "snoozed",
        MailboxRole::Important => "important",
        MailboxRole::Sent => "sent",
        MailboxRole::Drafts => "drafts",
        MailboxRole::Scheduled => "scheduled",
        MailboxRole::Outbox => "outbox",
        MailboxRole::Archive => "archive",
        MailboxRole::Spam => "spam",
        MailboxRole::Trash => "trash",
        MailboxRole::All => "all",
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{MailboxId, ThreadId};

    use super::Counts;
    use crate::testutil::{account, inbox, message, seeded};

    #[test]
    fn an_upsert_round_trips_the_envelope_and_replaces_it() {
        let (_dir, mut store) = seeded();
        let mut first = message("m1", "t1", 10, false);
        store.upsert_message(&first).unwrap();
        first.envelope.flags.seen = true;
        first.envelope.flags.keywords = vec!["home".into(), "a".into()];
        store.upsert_message(&first).unwrap();
        let got = store
            .thread_messages(&account(), &ThreadId::new("t1"))
            .unwrap();
        let mut expected = first.envelope.clone();
        expected.flags.keywords = vec!["a".into(), "home".into()];
        assert_eq!(got, vec![expected]);
    }

    #[test]
    fn threads_page_newest_first_with_a_stable_cursor() {
        let (_dir, mut store) = seeded();
        // t2 and t3 tie on latest_at; the id breaks the tie.
        for (id, thread, at) in [
            ("m1", "t1", 10),
            ("m2", "t2", 30),
            ("m3", "t3", 30),
            ("m4", "t1", 20),
            ("m5", "t4", 5),
        ] {
            store
                .upsert_message(&message(id, thread, at, false))
                .unwrap();
        }
        let mut seen = Vec::new();
        let mut cursor = None;
        loop {
            let page = store
                .thread_page(&account(), &inbox(), cursor.as_ref(), 2)
                .unwrap();
            assert!(page.threads.len() <= 2);
            seen.extend(page.threads.iter().map(|row| row.id.as_str().to_string()));
            match page.next {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        assert_eq!(seen, ["t3", "t2", "t1", "t4"]);
        let first = store.thread_page(&account(), &inbox(), None, 10).unwrap();
        let t1 = first
            .threads
            .iter()
            .find(|row| row.id.as_str() == "t1")
            .unwrap();
        assert_eq!(
            (t1.message_count, t1.latest_at, t1.subject.as_str()),
            (2, 20, "subject t1")
        );
    }

    #[test]
    fn counts_follow_membership_and_seen() {
        let (_dir, mut store) = seeded();
        store
            .upsert_message(&message("m1", "t1", 1, false))
            .unwrap();
        store.upsert_message(&message("m2", "t1", 2, true)).unwrap();
        let mut archived = message("m3", "t2", 3, false);
        archived.mailboxes = vec![MailboxId::new("archive")];
        store.upsert_message(&archived).unwrap();
        assert_eq!(
            store.mailbox_counts(&account(), &inbox()).unwrap(),
            Counts {
                total: 2,
                unread: 1
            }
        );
        let page = store.thread_page(&account(), &inbox(), None, 10).unwrap();
        assert_eq!(page.threads.len(), 1);
        assert_eq!(page.threads[0].unread_count, 1);
        // Moving m3 into the inbox makes its thread appear there too.
        archived.mailboxes = vec![inbox()];
        store.upsert_message(&archived).unwrap();
        assert_eq!(
            store.mailbox_counts(&account(), &inbox()).unwrap(),
            Counts {
                total: 3,
                unread: 2
            }
        );
    }

    #[test]
    fn a_message_that_changes_thread_leaves_no_empty_thread() {
        let (_dir, mut store) = seeded();
        store
            .upsert_message(&message("m1", "t1", 1, false))
            .unwrap();
        store
            .upsert_message(&message("m1", "t9", 1, false))
            .unwrap();
        let page = store.thread_page(&account(), &inbox(), None, 10).unwrap();
        let ids: Vec<&str> = page.threads.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["t9"]);
    }

    #[test]
    fn an_unknown_mailbox_is_refused() {
        let (_dir, mut store) = seeded();
        let mut lost = message("m1", "t1", 1, false);
        lost.mailboxes = vec![MailboxId::new("nowhere")];
        assert!(store.upsert_message(&lost).is_err());
        let page = store.thread_page(&account(), &inbox(), None, 10).unwrap();
        assert!(page.threads.is_empty());
    }

    #[test]
    fn sync_state_is_saved_per_scope() {
        let (_dir, mut store) = seeded();
        assert_eq!(store.sync_state(&account(), "jmap:Email").unwrap(), None);
        store
            .set_sync_state(&account(), "jmap:Email", "s1")
            .unwrap();
        store
            .set_sync_state(&account(), "jmap:Email", "s2")
            .unwrap();
        assert_eq!(
            store
                .sync_state(&account(), "jmap:Email")
                .unwrap()
                .as_deref(),
            Some("s2")
        );
    }
}
