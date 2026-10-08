//! Microsoft Graph mail sync from scripted JSON bodies.
//!
//! `graph-rs-sdk` is not linked. Nothing here opens a socket. A delta page
//! and a `$select` message go through `mailune-fixture`, which ignores
//! `@odata.deltaLink`, so the cursor is read beside the messages.

mod mutate;

pub use mutate::{
    BatchPart, FlagPatch, Move, SentMail, apply_batch, apply_flag, apply_move, apply_send,
};

use mailune_protocol::Envelope;
use mailune_store::{
    AccountRow, FlagRow, MailboxRow, MembershipRow, MessageRow, Store, SyncStateRow, ThreadRow,
};
use serde::Deserialize;
use serde_json::Value;

/// Failure while applying a scripted Graph body. The text names the field, not a secret.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The document is not the JSON shape this parser accepts.
    #[error("Graph JSON is not valid: {0}")]
    Json(String),
    /// A required field is missing or a timestamp is not RFC 3339.
    #[error("Graph document is missing a required field")]
    Body,
    /// The store rejected a row.
    #[error(transparent)]
    Store(#[from] mailune_store::Error),
    /// A message page was not the fixture shape.
    #[error(transparent)]
    Fixture(#[from] mailune_fixture::Error),
}

/// Folders, a delta page, and one `$select` message.
pub struct GraphBodies<'a> {
    /// Account id stored on every row.
    pub account_id: &'a str,
    /// Mailbox address for the account row.
    pub email: &'a str,
    /// `mailFolders` list.
    pub folders: &'a str,
    /// Message delta page. The fixture file is the sample.
    pub delta: &'a str,
    /// One message fetched with `$select`, or a page.
    pub selected: &'a str,
}

/// What the read sync wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphSync {
    /// Folder ids, in document order.
    pub folder_ids: Vec<String>,
    /// Message ids from the delta page and `$select`, first seen first.
    pub message_ids: Vec<String>,
    /// `@odata.deltaLink`, or `@odata.nextLink` when the delta is not finished.
    pub delta_link: String,
}

/// Upserts folders, a delta page, and a `$select` message.
///
/// # Errors
///
/// [`Error::Json`] or [`Error::Body`] when a document is the wrong shape.
/// [`Error::Store`] when a row cannot be written.
pub fn sync_read(store: &mut Store, bodies: &GraphBodies<'_>) -> Result<GraphSync, Error> {
    store.upsert_account(&AccountRow {
        id: bodies.account_id.to_string(),
        email: bodies.email.to_string(),
    })?;

    let folders = parse_folders(bodies.folders)?;
    for folder in &folders {
        store.upsert_mailbox(&MailboxRow {
            id: folder.id.clone(),
            account_id: bodies.account_id.to_string(),
            name: folder.name.clone(),
            role: folder.role.clone(),
        })?;
    }
    let inbox = folders
        .iter()
        .find(|folder| folder.role.as_deref() == Some("inbox"))
        .or_else(|| folders.first())
        .map(|folder| folder.id.clone());

    let mut message_ids = Vec::new();
    for envelope in envelopes(bodies.delta)? {
        push_id(&mut message_ids, envelope.id.as_str());
        apply_envelope(store, bodies.account_id, inbox.as_deref(), &envelope)?;
    }
    if !bodies.selected.trim().is_empty() {
        for envelope in envelopes(bodies.selected)? {
            push_id(&mut message_ids, envelope.id.as_str());
            apply_envelope(store, bodies.account_id, inbox.as_deref(), &envelope)?;
        }
    }

    let delta_link = parse_link(bodies.delta)?;
    if let Some(mailbox_id) = inbox.clone()
        && !delta_link.is_empty()
    {
        store.upsert_sync_state(&SyncStateRow {
            account_id: bodies.account_id.to_string(),
            mailbox_id,
            token: delta_link.clone(),
        })?;
    }

    Ok(GraphSync {
        folder_ids: folders.into_iter().map(|folder| folder.id).collect(),
        message_ids,
        delta_link,
    })
}

pub(crate) fn envelopes(json: &str) -> Result<Vec<Envelope>, Error> {
    let value: Value = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    if value.get("value").is_some() {
        return mailune_fixture::graph_delta(json).map_err(Error::from);
    }
    let wrapped = serde_json::json!({ "value": [value] });
    let text = serde_json::to_string(&wrapped).map_err(|err| Error::Json(err.to_string()))?;
    mailune_fixture::graph_delta(&text).map_err(Error::from)
}

pub(crate) fn apply_envelope(
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
    let to = envelope
        .to
        .iter()
        .map(|address| address.email.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    store.upsert_message(&MessageRow {
        id: id.to_string(),
        account_id: account_id.to_string(),
        thread_id: thread_id.to_string(),
        subject: envelope.subject.clone(),
        from_email: envelope.from.email.clone(),
        to_emails: to.clone(),
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
    store.index_message(
        id,
        &envelope.subject,
        &format!("{}\n{to}", envelope.from.email),
        &envelope.snippet,
    )?;
    Ok(())
}

fn push_id(ids: &mut Vec<String>, id: &str) {
    if !ids.iter().any(|have| have == id) {
        ids.push(id.to_string());
    }
}

fn parse_folders(json: &str) -> Result<Vec<Folder>, Error> {
    let doc: FolderPage = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok(doc
        .value
        .into_iter()
        .map(|folder| {
            let role = if folder.id.eq_ignore_ascii_case("inbox")
                || folder.display_name.eq_ignore_ascii_case("inbox")
            {
                Some("inbox".to_string())
            } else {
                None
            };
            Folder {
                id: folder.id,
                name: folder.display_name,
                role,
            }
        })
        .collect())
}

fn parse_link(json: &str) -> Result<String, Error> {
    let doc: PageLink = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok(doc.delta_link.or(doc.next_link).unwrap_or_default())
}

/// Civil date to Unix milliseconds. The store sorts on an integer, and the
/// Graph stamp is only a display string.
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

struct Folder {
    id: String,
    name: String,
    role: Option<String>,
}

#[derive(Deserialize)]
struct FolderPage {
    value: Vec<FolderItem>,
}

#[derive(Deserialize)]
struct FolderItem {
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize)]
struct PageLink {
    #[serde(rename = "@odata.deltaLink")]
    delta_link: Option<String>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{GraphBodies, sync_read};
    use mailune_store::Store;

    const DELTA: &str = include_str!("../../mailune-fixture/fixtures/graph-delta.json");

    fn open() -> (std::path::PathBuf, Store) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("mailune-graph-{nanos}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        (dir, store)
    }

    #[test]
    fn delta_and_select_upsert_the_fixture_message() {
        let (dir, mut store) = open();
        let applied = sync_read(
            &mut store,
            &GraphBodies {
                account_id: "acc-1",
                email: "me@example.com",
                folders: r#"{"value":[{"id":"inbox","displayName":"Inbox"}]}"#,
                delta: DELTA,
                selected: r#"{"id":"AAMk2","conversationId":"AAQk","subject":"Follow up","bodyPreview":"Tomorrow","receivedDateTime":"2026-03-01T13:00:00Z","isRead":true,"from":{"emailAddress":{"name":"Ada Lovelace","address":"ada@example.com"}},"toRecipients":[{"emailAddress":{"address":"me@example.com"}}]}"#,
            },
        )
        .unwrap();
        assert_eq!(applied.folder_ids, ["inbox"]);
        assert_eq!(applied.message_ids, ["AAMk", "AAMk2"]);
        assert_eq!(
            applied.delta_link,
            "mailFolders/inbox/messages/delta?token=1"
        );
        let rows = store.messages_in_thread("AAQk").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].subject, "Hello");
        assert_eq!(rows[0].from_email, "ada@example.com");
        assert_eq!(rows[0].received_at, 1_772_366_400_000);
        assert_eq!(rows[1].subject, "Follow up");
        assert_eq!(rows[1].received_at, 1_772_370_000_000);
        assert_eq!(store.search_text("dock").unwrap(), ["AAMk"]);
        let account = store.account("acc-1").unwrap().unwrap();
        assert_eq!(account.email, "me@example.com");
        let _ = std::fs::remove_dir_all(dir);
    }
}
