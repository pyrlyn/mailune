//! JMAP read sync from scripted JSON bodies.
//!
//! `jmap-client` builds a reqwest client and has no injected transport, so
//! these bodies are parsed in-process. Nothing here opens a socket.
//! `Email/get` goes through `mailune-fixture`.

#[cfg(feature = "sync")]
mod extra;
#[cfg(feature = "sync")]
mod mutate;
mod push;

#[cfg(feature = "sync")]
pub use extra::{MaskedEmail, SieveScript, create_masked, list_sieve};
#[cfg(feature = "sync")]
pub use mutate::{FlagChange, Submission, apply_flags, apply_submission};
pub use push::{StateChange, parse_event_source, parse_websocket};

#[cfg(feature = "sync")]
use mailune_protocol::Envelope;
#[cfg(feature = "sync")]
use mailune_store::{
    AccountRow, FlagRow, MailboxRow, MembershipRow, MessageRow, Store, SyncStateRow, ThreadRow,
};
use serde::Deserialize;

#[cfg(feature = "sync")]
const MAIL: &str = "urn:ietf:params:jmap:mail";

/// Failure while applying a scripted JMAP body. The text names the field, not a secret.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The document is not the JSON shape this parser accepts.
    #[error("JMAP JSON is not valid: {0}")]
    Json(String),
    /// A required field is missing or a timestamp is not RFC 3339.
    #[error("JMAP document is missing a required field")]
    Body,
    /// The store rejected a row.
    #[cfg(feature = "sync")]
    #[error(transparent)]
    Store(#[from] mailune_store::Error),
    /// `Email/get` was not the fixture shape.
    #[cfg(feature = "sync")]
    #[error(transparent)]
    Fixture(#[from] mailune_fixture::Error),
}

/// The six read-sync documents. Each is a method result, not a socket response.
pub struct ReadBodies<'a> {
    /// Session object (`username`, `primaryAccounts`).
    pub session: &'a str,
    /// `Mailbox/get` result.
    pub mailboxes: &'a str,
    /// `Thread/get` result.
    pub threads: &'a str,
    /// `Email/get` result. The fixture file is the sample.
    pub emails: &'a str,
    /// `Email/changes` result.
    pub changes: &'a str,
    /// `Email/query` result.
    pub query: &'a str,
}

/// What the read sync wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadSync {
    /// Primary mail account.
    pub account_id: String,
    /// Session username.
    pub email: String,
    /// Mailbox ids, in document order.
    pub mailbox_ids: Vec<String>,
    /// Email ids, in document order.
    pub message_ids: Vec<String>,
    /// Thread ids from `Thread/get`.
    pub thread_ids: Vec<String>,
    /// `newState` from `/changes`, else the query or email state.
    pub state: String,
    /// Ids from `/query`, in document order.
    pub query_ids: Vec<String>,
}

/// Upserts session, mailboxes, threads, emails, the change token, and the query.
///
/// # Errors
///
/// [`Error::Json`] or [`Error::Body`] when a document is the wrong shape.
/// [`Error::Store`] when a row cannot be written.
#[cfg(feature = "sync")]
pub fn sync_read(store: &mut Store, bodies: &ReadBodies<'_>) -> Result<ReadSync, Error> {
    let (account_id, email) = parse_session(bodies.session)?;
    store.upsert_account(&AccountRow {
        id: account_id.clone(),
        email: email.clone(),
    })?;

    let mailboxes = parse_mailboxes(bodies.mailboxes)?;
    for mailbox in &mailboxes {
        store.upsert_mailbox(&MailboxRow {
            id: mailbox.id.clone(),
            account_id: account_id.clone(),
            name: mailbox.name.clone(),
            role: mailbox.role.clone(),
        })?;
    }
    let inbox = inbox_id(&mailboxes).map(str::to_string);

    let thread_ids = parse_threads(bodies.threads)?;
    for id in &thread_ids {
        store.upsert_thread(&ThreadRow {
            id: id.clone(),
            account_id: account_id.clone(),
            subject: String::new(),
        })?;
    }

    let emails = mailune_fixture::jmap_email_get(bodies.emails)?;
    let mut message_ids = Vec::with_capacity(emails.len());
    for envelope in &emails {
        message_ids.push(envelope.id.as_str().to_string());
        upsert_envelope(store, &account_id, inbox.as_deref(), envelope)?;
    }

    let changed = parse_changes(bodies.changes)?;
    let (query_ids, query_state) = parse_query(bodies.query)?;
    let email_state = parse_email_state(bodies.emails)?;
    let state = changed.or(query_state).or(email_state).unwrap_or_default();
    if let Some(mailbox_id) = inbox
        && !state.is_empty()
    {
        store.upsert_sync_state(&SyncStateRow {
            account_id: account_id.clone(),
            mailbox_id,
            token: state.clone(),
        })?;
    }

    Ok(ReadSync {
        account_id,
        email,
        mailbox_ids: mailboxes.into_iter().map(|mailbox| mailbox.id).collect(),
        message_ids,
        thread_ids,
        state,
        query_ids,
    })
}

#[cfg(feature = "sync")]
fn upsert_envelope(
    store: &mut Store,
    account_id: &str,
    mailbox_id: Option<&str>,
    envelope: &Envelope,
) -> Result<(), Error> {
    let id = envelope.id.as_str();
    let thread_id = envelope.thread.as_str();
    store.upsert_thread(&ThreadRow {
        id: thread_id.to_string(),
        account_id: account_id.to_string(),
        subject: envelope.subject.clone(),
    })?;
    store.upsert_message(&MessageRow {
        id: id.to_string(),
        account_id: account_id.to_string(),
        thread_id: thread_id.to_string(),
        subject: envelope.subject.clone(),
        from_email: envelope.from.email.clone(),
        to_emails: join_emails(&envelope.to),
        stamp: envelope.stamp.clone(),
        received_at: stamp_millis(&envelope.stamp)?,
    })?;
    store.upsert_flags(&FlagRow {
        message_id: id.to_string(),
        seen: envelope.flags.seen,
        flagged: envelope.flags.flagged,
        draft: envelope.flags.draft,
        answered: envelope.flags.answered,
        deleted: envelope.flags.deleted,
        keywords: envelope.flags.keywords.join("\n"),
    })?;
    if let Some(mailbox_id) = mailbox_id {
        store.upsert_membership(&MembershipRow {
            message_id: id.to_string(),
            mailbox_id: mailbox_id.to_string(),
        })?;
    }
    let addresses = address_lines(envelope);
    store.index_message(id, &envelope.subject, &addresses, &envelope.snippet)?;
    Ok(())
}

#[cfg(feature = "sync")]
fn join_emails(addresses: &[mailune_protocol::Address]) -> String {
    addresses
        .iter()
        .map(|address| address.email.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(feature = "sync")]
fn address_lines(envelope: &Envelope) -> String {
    let mut lines = Vec::with_capacity(1 + envelope.to.len());
    lines.push(envelope.from.email.as_str());
    lines.extend(envelope.to.iter().map(|address| address.email.as_str()));
    lines.join("\n")
}

#[cfg(feature = "sync")]
fn inbox_id(mailboxes: &[ParsedMailbox]) -> Option<&str> {
    mailboxes
        .iter()
        .find(|mailbox| mailbox.role.as_deref() == Some("inbox"))
        .or_else(|| mailboxes.first())
        .map(|mailbox| mailbox.id.as_str())
}

#[cfg(feature = "sync")]
fn parse_session(json: &str) -> Result<(String, String), Error> {
    let mut doc: Session =
        serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    let account_id = if let Some(id) = doc.primary_accounts.remove(MAIL) {
        id
    } else {
        doc.primary_accounts
            .into_values()
            .next()
            .ok_or(Error::Body)?
    };
    if doc.username.is_empty() {
        return Err(Error::Body);
    }
    Ok((account_id, doc.username))
}

#[cfg(feature = "sync")]
fn parse_mailboxes(json: &str) -> Result<Vec<ParsedMailbox>, Error> {
    let doc: List<ParsedMailbox> =
        serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok(doc.list)
}

#[cfg(feature = "sync")]
fn parse_threads(json: &str) -> Result<Vec<String>, Error> {
    let doc: List<IdOnly> =
        serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok(doc.list.into_iter().map(|thread| thread.id).collect())
}

#[cfg(feature = "sync")]
fn parse_changes(json: &str) -> Result<Option<String>, Error> {
    let doc: Changes = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok(doc.new_state)
}

fn parse_query(json: &str) -> Result<(Vec<String>, Option<String>), Error> {
    let doc: Query = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok((doc.ids, doc.query_state))
}

/// Mailbox ids from a scripted `Mailbox/query` body, in document order.
///
/// # Errors
///
/// [`Error::Json`] when the body is not a query object.
pub fn query_mailbox_ids(json: &str) -> Result<Vec<String>, Error> {
    Ok(parse_query(json)?.0)
}

#[cfg(feature = "sync")]
fn parse_email_state(json: &str) -> Result<Option<String>, Error> {
    let doc: EmailState = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok(doc.state)
}

/// Civil date to Unix milliseconds. A date crate would be a second parser for one field.
#[cfg(feature = "sync")]
fn stamp_millis(stamp: &str) -> Result<i64, Error> {
    let stamp = stamp.trim();
    if stamp.is_empty() {
        return Ok(0);
    }
    let stamp = stamp.strip_suffix('Z').unwrap_or(stamp);
    let stamp = stamp.split_once('+').map(|(head, _)| head).unwrap_or(stamp);
    let (date, time) = stamp.split_once('T').ok_or(Error::Body)?;
    let time = time.split_once('.').map(|(hms, _)| hms).unwrap_or(time);
    let date: Vec<&str> = date.split('-').collect();
    let time: Vec<&str> = time.split(':').collect();
    if date.len() != 3 || time.len() != 3 {
        return Err(Error::Body);
    }
    let year: i64 = date[0].parse().map_err(|_| Error::Body)?;
    let month: u32 = date[1].parse().map_err(|_| Error::Body)?;
    let day: u32 = date[2].parse().map_err(|_| Error::Body)?;
    let hour: u32 = time[0].parse().map_err(|_| Error::Body)?;
    let minute: u32 = time[1].parse().map_err(|_| Error::Body)?;
    let second: u32 = time[2].parse().map_err(|_| Error::Body)?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(Error::Body);
    }
    let days = days_from_civil(year, month, day);
    Ok(days * 86_400_000
        + i64::from(hour) * 3_600_000
        + i64::from(minute) * 60_000
        + i64::from(second) * 1_000)
}

#[cfg(feature = "sync")]
fn days_from_civil(mut year: i64, month: u32, day: u32) -> i64 {
    if month <= 2 {
        year -= 1;
    }
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let month_prime = if month > 2 {
        i64::from(month) - 3
    } else {
        i64::from(month) + 9
    };
    let doy = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(feature = "sync")]
#[derive(Deserialize)]
struct Session {
    username: String,
    #[serde(rename = "primaryAccounts")]
    primary_accounts: std::collections::BTreeMap<String, String>,
}

#[cfg(feature = "sync")]
#[derive(Deserialize)]
struct List<T> {
    list: Vec<T>,
}

#[cfg(feature = "sync")]
#[derive(Deserialize)]
struct ParsedMailbox {
    id: String,
    name: String,
    role: Option<String>,
}

#[cfg(feature = "sync")]
#[derive(Deserialize)]
struct IdOnly {
    id: String,
}

#[cfg(feature = "sync")]
#[derive(Deserialize)]
struct Changes {
    #[serde(rename = "newState")]
    new_state: Option<String>,
}

#[derive(Deserialize)]
struct Query {
    ids: Vec<String>,
    #[serde(rename = "queryState")]
    query_state: Option<String>,
}

#[cfg(feature = "sync")]
#[derive(Deserialize)]
struct EmailState {
    state: Option<String>,
}

#[cfg(test)]
#[cfg(feature = "sync")]
mod tests {
    use super::{ReadBodies, stamp_millis, sync_read};
    use mailune_store::Store;

    const EMAILS: &str = include_str!("../../mailune-fixture/fixtures/jmap-email-get.json");

    fn open() -> (std::path::PathBuf, Store) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("mailune-jmap-{nanos}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        (dir, store)
    }

    #[test]
    fn fixture_stamp_is_unix_millis() {
        assert_eq!(
            stamp_millis("2026-03-01T12:00:00Z").unwrap(),
            1_772_366_400_000
        );
    }

    #[test]
    fn read_sync_upserts_the_fixture_email() {
        let (dir, mut store) = open();
        let applied = sync_read(
            &mut store,
            &ReadBodies {
                session: r#"{"username":"me@example.com","primaryAccounts":{"urn:ietf:params:jmap:mail":"acc-1"}}"#,
                mailboxes: r#"{"accountId":"acc-1","state":"mb","list":[{"id":"inbox","name":"Inbox","role":"inbox"}]}"#,
                threads: r#"{"accountId":"acc-1","state":"th","list":[{"id":"t1","emailIds":["m1"]}]}"#,
                emails: EMAILS,
                changes: r#"{"accountId":"acc-1","oldState":"s0","newState":"s2","hasMoreChanges":false,"created":["m1"],"updated":[],"destroyed":[]}"#,
                query: r#"{"accountId":"acc-1","queryState":"s2","ids":["m1"]}"#,
            },
        )
        .unwrap();
        assert_eq!(applied.account_id, "acc-1");
        assert_eq!(applied.email, "me@example.com");
        assert_eq!(applied.mailbox_ids, ["inbox"]);
        assert_eq!(applied.message_ids, ["m1"]);
        assert_eq!(applied.thread_ids, ["t1"]);
        assert_eq!(applied.state, "s2");
        assert_eq!(applied.query_ids, ["m1"]);
        let rows = store.messages_in_thread("t1").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].subject, "Hello");
        assert_eq!(rows[0].from_email, "ada@example.com");
        assert_eq!(rows[0].to_emails, "me@example.com");
        assert_eq!(rows[0].received_at, 1_772_366_400_000);
        assert_eq!(store.search_text("dock").unwrap(), ["m1"]);
        let account = store.account("acc-1").unwrap().unwrap();
        assert_eq!(account.email, "me@example.com");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_broken_session_is_rejected() {
        let (dir, mut store) = open();
        let err = sync_read(
            &mut store,
            &ReadBodies {
                session: "[]",
                mailboxes: r#"{"list":[]}"#,
                threads: r#"{"list":[]}"#,
                emails: r#"{"list":[]}"#,
                changes: "{}",
                query: r#"{"ids":[]}"#,
            },
        )
        .unwrap_err();
        assert!(matches!(err, super::Error::Json(_)));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_mailbox_query_keeps_document_order() {
        let ids = super::query_mailbox_ids(
            r#"{"accountId":"acc-1","queryState":"q1","ids":["inbox","archive"]}"#,
        )
        .unwrap();
        assert_eq!(ids, ["inbox", "archive"]);
    }
}
