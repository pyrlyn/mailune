//! Gmail read sync from scripted JSON bodies.
//!
//! `google-gmail1` is not linked. `reqwest` is not linked either: a client
//! would open a socket, and these bodies never leave the process. History
//! rows come from `mailune-fixture`. That parser drops the message id, so
//! the id and `historyId` are read beside it.

use mailune_protocol::AccountId;
use mailune_store::{
    AccountRow, FlagRow, MailboxRow, MembershipRow, MessageRow, Store, SyncStateRow, ThreadRow,
};
use serde::Deserialize;
use serde_json::Value;

/// Failure while applying a scripted Gmail body. The text names the field, not a secret.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The document is not the JSON shape this parser accepts.
    #[error("Gmail JSON is not valid: {0}")]
    Json(String),
    /// A required field is missing.
    #[error("Gmail document is missing a required field")]
    Body,
    /// The store rejected a row.
    #[error(transparent)]
    Store(#[from] mailune_store::Error),
    /// History was not the fixture shape.
    #[error(transparent)]
    Fixture(#[from] mailune_fixture::Error),
}

/// The four read-sync documents, plus the account they belong to.
pub struct GmailBodies<'a> {
    /// Account id stored on every row.
    pub account_id: &'a str,
    /// Mailbox address for the account row.
    pub email: &'a str,
    /// `users.labels.list`.
    pub labels: &'a str,
    /// `users.threads.list`.
    pub threads: &'a str,
    /// `users.history.list`. The fixture file is the sample.
    pub history: &'a str,
    /// Messages from a batch fetch, as a JSON array.
    pub batch: &'a str,
}

/// What the read sync wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmailSync {
    /// `historyId` from the history document.
    pub history_id: String,
    /// Label ids, in document order.
    pub label_ids: Vec<String>,
    /// Thread ids from the thread list.
    pub thread_ids: Vec<String>,
    /// Message ids from history and the batch, first seen first.
    pub message_ids: Vec<String>,
}

/// Upserts labels, threads, a history diff, and a batch of full messages.
///
/// # Errors
///
/// [`Error::Json`] or [`Error::Body`] when a document is the wrong shape.
/// [`Error::Store`] when a row cannot be written.
pub fn sync_read(store: &mut Store, bodies: &GmailBodies<'_>) -> Result<GmailSync, Error> {
    store.upsert_account(&AccountRow {
        id: bodies.account_id.to_string(),
        email: bodies.email.to_string(),
    })?;

    let labels = parse_labels(bodies.labels)?;
    for label in &labels {
        store.upsert_mailbox(&MailboxRow {
            id: label.id.clone(),
            account_id: bodies.account_id.to_string(),
            name: label.name.clone(),
            role: role_of(&label.id),
        })?;
    }

    let thread_ids = parse_threads(bodies.threads)?;
    for thread in &thread_ids {
        store.upsert_thread(&ThreadRow {
            id: thread.id.clone(),
            account_id: bodies.account_id.to_string(),
            subject: thread.snippet(),
        })?;
    }

    let mut message_ids = Vec::new();
    let (history_id, added) = parse_history(bodies.history)?;
    let rows = mailune_fixture::gmail_history(bodies.history, &AccountId::new(bodies.account_id))?;
    if rows.len() != added.len() {
        return Err(Error::Body);
    }
    for (row, message) in rows.iter().zip(&added) {
        push_id(&mut message_ids, &message.id);
        let snippet = message.snippet();
        let mailbox_id = row.mailbox.as_str();
        store.upsert_mailbox(&MailboxRow {
            id: mailbox_id.to_string(),
            account_id: bodies.account_id.to_string(),
            name: mailbox_id.to_string(),
            role: Some(mailbox_id.to_string()),
        })?;
        store.upsert_thread(&ThreadRow {
            id: message.thread_id.clone(),
            account_id: bodies.account_id.to_string(),
            subject: snippet.clone(),
        })?;
        store.upsert_message(&MessageRow {
            id: message.id.clone(),
            account_id: bodies.account_id.to_string(),
            thread_id: message.thread_id.clone(),
            subject: snippet.clone(),
            from_email: row.from.email.clone(),
            to_emails: String::new(),
            stamp: row.stamp.clone(),
            received_at: 0,
        })?;
        store.upsert_flags(&FlagRow {
            message_id: message.id.clone(),
            seen: !row.unread,
            flagged: row.flagged,
            draft: row.draft,
            answered: false,
            deleted: false,
            keywords: row.labels.join("\n"),
        })?;
        store.upsert_membership(&MembershipRow {
            message_id: message.id.clone(),
            mailbox_id: mailbox_id.to_string(),
        })?;
        store.index_message(&message.id, &snippet, "", &snippet)?;
    }
    if !history_id.is_empty() {
        let mailbox_id = rows
            .first()
            .map(|row| row.mailbox.as_str().to_string())
            .unwrap_or_else(|| "inbox".to_string());
        store.upsert_mailbox(&MailboxRow {
            id: mailbox_id.clone(),
            account_id: bodies.account_id.to_string(),
            name: mailbox_id.clone(),
            role: Some(mailbox_id.clone()),
        })?;
        store.upsert_sync_state(&SyncStateRow {
            account_id: bodies.account_id.to_string(),
            mailbox_id,
            token: history_id.clone(),
        })?;
    }

    for message in parse_batch(bodies.batch)? {
        push_id(&mut message_ids, &message.id);
        apply_message(store, bodies.account_id, &message)?;
    }

    Ok(GmailSync {
        history_id,
        label_ids: labels.into_iter().map(|label| label.id).collect(),
        thread_ids: thread_ids.into_iter().map(|thread| thread.id).collect(),
        message_ids,
    })
}

fn apply_message(store: &mut Store, account_id: &str, message: &BatchMessage) -> Result<(), Error> {
    let headers = message
        .payload
        .as_ref()
        .and_then(|payload| payload.headers.as_ref());
    let subject = header(headers, "Subject");
    let from = email_only(&header(headers, "From"));
    let to = header(headers, "To")
        .split(',')
        .map(email_only)
        .filter(|email| !email.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let subject = if subject.is_empty() {
        message.snippet.clone().unwrap_or_default()
    } else {
        subject
    };
    let received_at = match message.internal_date.as_deref() {
        Some(text) if !text.is_empty() => text.parse().map_err(|_| Error::Body)?,
        _ => 0,
    };
    store.upsert_thread(&ThreadRow {
        id: message.thread_id.clone(),
        account_id: account_id.to_string(),
        subject: subject.clone(),
    })?;
    store.upsert_message(&MessageRow {
        id: message.id.clone(),
        account_id: account_id.to_string(),
        thread_id: message.thread_id.clone(),
        subject: subject.clone(),
        from_email: from.clone(),
        to_emails: to.clone(),
        stamp: message.internal_date.clone().unwrap_or_default(),
        received_at,
    })?;
    let labels = message.label_ids.clone().unwrap_or_default();
    store.upsert_flags(&FlagRow {
        message_id: message.id.clone(),
        seen: !labels.iter().any(|label| label == "UNREAD"),
        flagged: labels.iter().any(|label| label == "STARRED"),
        draft: labels.iter().any(|label| label == "DRAFT"),
        answered: false,
        deleted: false,
        keywords: labels
            .iter()
            .filter(|label| !system_label(label))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    })?;
    for label in labels.iter().filter(|label| mailbox_label(label)) {
        store.upsert_mailbox(&MailboxRow {
            id: label.clone(),
            account_id: account_id.to_string(),
            name: label.clone(),
            role: role_of(label),
        })?;
        store.upsert_membership(&MembershipRow {
            message_id: message.id.clone(),
            mailbox_id: label.clone(),
        })?;
    }
    store.index_message(
        &message.id,
        &subject,
        &format!("{from}\n{to}"),
        message.snippet.as_deref().unwrap_or(""),
    )?;
    Ok(())
}

fn push_id(ids: &mut Vec<String>, id: &str) {
    if !ids.iter().any(|have| have == id) {
        ids.push(id.to_string());
    }
}

fn header(headers: Option<&Vec<Header>>, name: &str) -> String {
    headers
        .and_then(|headers| {
            headers
                .iter()
                .find(|header| header.name.eq_ignore_ascii_case(name))
                .map(|header| header.value.clone())
        })
        .unwrap_or_default()
}

fn email_only(value: &str) -> String {
    value
        .rsplit_once('<')
        .map(|(_, rest)| rest.trim().trim_end_matches('>').trim().to_string())
        .unwrap_or_else(|| value.trim().to_string())
}

fn role_of(id: &str) -> Option<String> {
    match id {
        "INBOX" | "inbox" => Some("inbox".to_string()),
        "SENT" | "sent" => Some("sent".to_string()),
        "DRAFT" | "drafts" => Some("drafts".to_string()),
        "TRASH" | "trash" => Some("trash".to_string()),
        "SPAM" | "spam" => Some("spam".to_string()),
        _ => None,
    }
}

fn system_label(label: &str) -> bool {
    matches!(
        label,
        "INBOX" | "UNREAD" | "STARRED" | "IMPORTANT" | "SENT" | "DRAFT" | "SPAM" | "TRASH"
    ) || label.starts_with("CATEGORY_")
}

fn mailbox_label(label: &str) -> bool {
    !matches!(label, "UNREAD" | "STARRED" | "IMPORTANT") && !label.starts_with("CATEGORY_")
}

fn json<T: for<'de> Deserialize<'de>>(text: &str) -> Result<T, Error> {
    serde_json::from_str(text).map_err(|err| Error::Json(err.to_string()))
}

fn parse_labels(text: &str) -> Result<Vec<Label>, Error> {
    Ok(json::<LabelList>(text)?.labels.unwrap_or_default())
}

fn parse_threads(text: &str) -> Result<Vec<ThreadItem>, Error> {
    Ok(json::<ThreadList>(text)?.threads.unwrap_or_default())
}

fn parse_history(text: &str) -> Result<(String, Vec<AddedMessage>), Error> {
    let doc: HistoryDoc = json(text)?;
    let mut added = Vec::new();
    for record in doc.history.into_iter().flatten() {
        for item in record.messages_added.into_iter().flatten() {
            added.push(item.message);
        }
    }
    Ok((doc.history_id.unwrap_or_default(), added))
}

fn parse_batch(text: &str) -> Result<Vec<BatchMessage>, Error> {
    let value: Value = json(text)?;
    let list = if let Some(messages) = value.as_array() {
        messages.clone()
    } else if let Some(messages) = value.get("messages").and_then(Value::as_array) {
        messages.clone()
    } else {
        vec![value]
    };
    list.into_iter()
        .map(|item| serde_json::from_value(item).map_err(|err| Error::Json(err.to_string())))
        .collect()
}

#[derive(Deserialize)]
struct LabelList {
    labels: Option<Vec<Label>>,
}

#[derive(Deserialize)]
struct Label {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct ThreadList {
    threads: Option<Vec<ThreadItem>>,
}

#[derive(Deserialize)]
struct ThreadItem {
    id: String,
    snippet: Option<String>,
}

impl ThreadItem {
    fn snippet(&self) -> String {
        self.snippet.clone().unwrap_or_default()
    }
}

#[derive(Deserialize)]
struct HistoryDoc {
    #[serde(rename = "historyId")]
    history_id: Option<String>,
    history: Option<Vec<HistoryRecord>>,
}

#[derive(Deserialize)]
struct HistoryRecord {
    #[serde(rename = "messagesAdded")]
    messages_added: Option<Vec<Added>>,
}

#[derive(Deserialize)]
struct Added {
    message: AddedMessage,
}

#[derive(Deserialize)]
struct AddedMessage {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: String,
    snippet: Option<String>,
}

impl AddedMessage {
    fn snippet(&self) -> String {
        self.snippet.clone().unwrap_or_default()
    }
}

#[derive(Deserialize)]
struct BatchMessage {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: String,
    #[serde(rename = "labelIds")]
    label_ids: Option<Vec<String>>,
    snippet: Option<String>,
    #[serde(rename = "internalDate")]
    internal_date: Option<String>,
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    headers: Option<Vec<Header>>,
}

#[derive(Deserialize)]
struct Header {
    name: String,
    value: String,
}

#[cfg(test)]
mod tests {
    use super::{GmailBodies, sync_read};
    use mailune_store::Store;

    const HISTORY: &str = include_str!("../../mailune-fixture/fixtures/gmail-history.json");

    fn open() -> (std::path::PathBuf, Store) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("mailune-gmail-{nanos}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        (dir, store)
    }

    #[test]
    fn history_and_batch_upsert_the_fixture_thread() {
        let (dir, mut store) = open();
        let applied = sync_read(
            &mut store,
            &GmailBodies {
                account_id: "acc-1",
                email: "me@example.com",
                labels: r#"{"labels":[{"id":"INBOX","name":"INBOX","type":"system"},{"id":"Label_1","name":"Work","type":"user"}]}"#,
                threads: r#"{"threads":[{"id":"t1","snippet":"See you at the dock","historyId":"99"}]}"#,
                history: HISTORY,
                batch: r#"[{"id":"m1","threadId":"t1","labelIds":["INBOX","UNREAD","STARRED"],"snippet":"See you at the dock","internalDate":"1772366400000","payload":{"headers":[{"name":"From","value":"Ada Lovelace <ada@example.com>"},{"name":"To","value":"me@example.com"},{"name":"Subject","value":"Hello"}]}}]"#,
            },
        )
        .unwrap();
        assert_eq!(applied.history_id, "100");
        assert_eq!(applied.label_ids, ["INBOX", "Label_1"]);
        assert_eq!(applied.thread_ids, ["t1"]);
        assert_eq!(applied.message_ids, ["m1"]);
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
}
