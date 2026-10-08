//! Static provider fixtures turned into protocol records.
//!
//! The bytes are already on disk. Nothing here opens a socket. JMAP `Email/get`
//! and a Graph delta page become envelopes. A Gmail history list has no `From`
//! or subject, so it becomes thread rows.

use std::collections::BTreeMap;

use mailune_protocol::{
    AccountId, Address, Category, Envelope, Flags, MailboxId, MessageId, ThreadId, ThreadRow,
    TransportSecurity,
};
use serde::Deserialize;

/// Failure while reading a fixture. The text names the field, not a secret.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The document is not the JSON shape this parser accepts.
    #[error("fixture JSON is not valid: {0}")]
    Json(String),
}

/// Parses a JMAP `Email/get` result object into envelopes.
///
/// The caller passes the method result (`accountId`, `list`, …), not the
/// outer `methodResponses` wrapper.
///
/// # Errors
///
/// [`Error::Json`] when the document is not that object.
pub fn jmap_email_get(json: &str) -> Result<Vec<Envelope>, Error> {
    let doc: JmapGet = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    Ok(doc.list.into_iter().map(jmap_envelope).collect())
}

/// Parses a Gmail `history.list` body into thread rows.
///
/// History records do not carry `From` or a subject. The row uses an empty
/// sender and an empty subject, and the snippet and labels it does have.
///
/// # Errors
///
/// [`Error::Json`] when the document is not a history list.
pub fn gmail_history(json: &str, account: &AccountId) -> Result<Vec<ThreadRow>, Error> {
    let doc: GmailHistory =
        serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    let mut rows = Vec::new();
    for record in doc.history.into_iter().flatten() {
        for added in record.messages_added.into_iter().flatten() {
            rows.push(gmail_row(account, added.message));
        }
    }
    Ok(rows)
}

/// Parses a Microsoft Graph message delta page into envelopes.
///
/// `@odata.deltaLink` and `@odata.nextLink` are ignored: the page's `value`
/// array is the mail.
///
/// # Errors
///
/// [`Error::Json`] when `value` is missing or a message has no sender address.
pub fn graph_delta(json: &str) -> Result<Vec<Envelope>, Error> {
    let doc: GraphPage = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    doc.value
        .into_iter()
        .map(graph_envelope)
        .collect::<Result<Vec<_>, _>>()
}

fn jmap_envelope(email: JmapEmail) -> Envelope {
    let keywords = email.keywords.unwrap_or_default();
    Envelope {
        id: MessageId::new(email.id),
        thread: ThreadId::new(email.thread_id),
        from: email
            .from
            .into_iter()
            .flatten()
            .next()
            .map(jmap_address)
            .unwrap_or_else(empty_address),
        to: email.to.into_iter().flatten().map(jmap_address).collect(),
        cc: email.cc.into_iter().flatten().map(jmap_address).collect(),
        subject: email.subject.unwrap_or_default(),
        stamp: email.received_at.unwrap_or_default(),
        snippet: email.preview.unwrap_or_default(),
        flags: Flags {
            seen: keywords.get("$seen").copied().unwrap_or(false),
            flagged: keywords.get("$flagged").copied().unwrap_or(false),
            draft: keywords.get("$draft").copied().unwrap_or(false),
            answered: keywords.get("$answered").copied().unwrap_or(false),
            deleted: false,
            keywords: Vec::new(),
        },
        attachment_count: u32::from(email.has_attachment.unwrap_or(false)),
        transport: TransportSecurity::Tls,
    }
}

fn jmap_address(address: JmapAddress) -> Address {
    Address {
        name: address.name.filter(|name| !name.is_empty()),
        email: address.email,
    }
}

fn gmail_row(account: &AccountId, message: GmailMessage) -> ThreadRow {
    let labels = message.label_ids.unwrap_or_default();
    ThreadRow {
        id: ThreadId::new(message.thread_id),
        account: account.clone(),
        from: empty_address(),
        subject: String::new(),
        snippet: message.snippet.unwrap_or_default(),
        stamp: String::new(),
        message_count: 1,
        unread: has(&labels, "UNREAD"),
        flagged: has(&labels, "STARRED"),
        important: has(&labels, "IMPORTANT"),
        pinned: false,
        snoozed: false,
        draft: has(&labels, "DRAFT"),
        has_attachment: false,
        category: gmail_category(&labels),
        mailbox: MailboxId::new(gmail_mailbox(&labels)),
        labels: labels
            .into_iter()
            .filter(|label| !gmail_system_label(label))
            .collect(),
    }
}

fn has(labels: &[String], name: &str) -> bool {
    labels.iter().any(|label| label == name)
}

fn gmail_mailbox(labels: &[String]) -> &'static str {
    if has(labels, "TRASH") {
        "trash"
    } else if has(labels, "SPAM") {
        "spam"
    } else if has(labels, "SENT") {
        "sent"
    } else if has(labels, "DRAFT") {
        "drafts"
    } else if has(labels, "INBOX") {
        "inbox"
    } else {
        "all"
    }
}

fn gmail_system_label(label: &str) -> bool {
    matches!(
        label,
        "INBOX" | "UNREAD" | "STARRED" | "IMPORTANT" | "SENT" | "DRAFT" | "SPAM" | "TRASH"
    ) || label.starts_with("CATEGORY_")
}

fn gmail_category(labels: &[String]) -> Category {
    if has(labels, "CATEGORY_SOCIAL") {
        Category::Social
    } else if has(labels, "CATEGORY_PROMOTIONS") {
        Category::Promotions
    } else if has(labels, "CATEGORY_UPDATES") {
        Category::Updates
    } else {
        Category::Primary
    }
}

fn graph_envelope(message: GraphMessage) -> Result<Envelope, Error> {
    let from = message
        .from
        .and_then(|from| from.email_address)
        .ok_or_else(|| Error::Json("graph message has no from".into()))?;
    let flagged = message
        .flag
        .and_then(|flag| flag.flag_status)
        .is_some_and(|status| status.eq_ignore_ascii_case("flagged"));
    Ok(Envelope {
        id: MessageId::new(message.id),
        thread: ThreadId::new(message.conversation_id.unwrap_or_default()),
        from: graph_address(from)?,
        to: message
            .to_recipients
            .into_iter()
            .flatten()
            .filter_map(|recipient| recipient.email_address)
            .map(graph_address)
            .collect::<Result<Vec<_>, _>>()?,
        cc: Vec::new(),
        subject: message.subject.unwrap_or_default(),
        stamp: message.received_date_time.unwrap_or_default(),
        snippet: message.body_preview.unwrap_or_default(),
        flags: Flags {
            seen: message.is_read.unwrap_or(false),
            flagged,
            draft: false,
            answered: false,
            deleted: false,
            keywords: Vec::new(),
        },
        attachment_count: u32::from(message.has_attachments.unwrap_or(false)),
        transport: TransportSecurity::Tls,
    })
}

fn graph_address(address: GraphAddress) -> Result<Address, Error> {
    let email = address
        .address
        .filter(|email| !email.is_empty())
        .ok_or_else(|| Error::Json("graph address is empty".into()))?;
    Ok(Address {
        name: address.name.filter(|name| !name.is_empty()),
        email,
    })
}

fn empty_address() -> Address {
    Address {
        name: None,
        email: String::new(),
    }
}

#[derive(Deserialize)]
struct JmapGet {
    list: Vec<JmapEmail>,
}

#[derive(Deserialize)]
struct JmapEmail {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: String,
    subject: Option<String>,
    preview: Option<String>,
    #[serde(rename = "receivedAt")]
    received_at: Option<String>,
    from: Option<Vec<JmapAddress>>,
    to: Option<Vec<JmapAddress>>,
    cc: Option<Vec<JmapAddress>>,
    keywords: Option<BTreeMap<String, bool>>,
    #[serde(rename = "hasAttachment")]
    has_attachment: Option<bool>,
}

#[derive(Deserialize)]
struct JmapAddress {
    name: Option<String>,
    email: String,
}

#[derive(Deserialize)]
struct GmailHistory {
    history: Option<Vec<GmailRecord>>,
}

#[derive(Deserialize)]
struct GmailRecord {
    #[serde(rename = "messagesAdded")]
    messages_added: Option<Vec<GmailAdded>>,
}

#[derive(Deserialize)]
struct GmailAdded {
    message: GmailMessage,
}

#[derive(Deserialize)]
struct GmailMessage {
    #[serde(rename = "threadId")]
    thread_id: String,
    snippet: Option<String>,
    #[serde(rename = "labelIds")]
    label_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct GraphPage {
    value: Vec<GraphMessage>,
}

#[derive(Deserialize)]
struct GraphMessage {
    id: String,
    #[serde(rename = "conversationId")]
    conversation_id: Option<String>,
    subject: Option<String>,
    #[serde(rename = "bodyPreview")]
    body_preview: Option<String>,
    #[serde(rename = "receivedDateTime")]
    received_date_time: Option<String>,
    #[serde(rename = "isRead")]
    is_read: Option<bool>,
    flag: Option<GraphFlag>,
    #[serde(rename = "hasAttachments")]
    has_attachments: Option<bool>,
    from: Option<GraphRecipient>,
    #[serde(rename = "toRecipients")]
    to_recipients: Option<Vec<GraphRecipient>>,
}

#[derive(Deserialize)]
struct GraphFlag {
    #[serde(rename = "flagStatus")]
    flag_status: Option<String>,
}

#[derive(Deserialize)]
struct GraphRecipient {
    #[serde(rename = "emailAddress")]
    email_address: Option<GraphAddress>,
}

#[derive(Deserialize)]
struct GraphAddress {
    name: Option<String>,
    address: Option<String>,
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{AccountId, Category, MailboxId, MessageId, ThreadId};

    use super::{gmail_history, graph_delta, jmap_email_get};

    #[test]
    fn jmap_email_get_becomes_an_envelope() {
        let envelopes = jmap_email_get(include_str!("../fixtures/jmap-email-get.json")).unwrap();
        assert_eq!(envelopes.len(), 1);
        let message = &envelopes[0];
        assert_eq!(message.id, MessageId::new("m1"));
        assert_eq!(message.thread, ThreadId::new("t1"));
        assert_eq!(message.subject, "Hello");
        assert_eq!(message.from.email, "ada@example.com");
        assert_eq!(message.from.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(message.to[0].email, "me@example.com");
        assert!(message.flags.seen);
        assert!(message.flags.flagged);
        assert_eq!(message.attachment_count, 0);
    }

    #[test]
    fn gmail_history_becomes_a_thread_row() {
        let rows = gmail_history(
            include_str!("../fixtures/gmail-history.json"),
            &AccountId::new("ada"),
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.id, ThreadId::new("t1"));
        assert_eq!(row.account, AccountId::new("ada"));
        assert_eq!(row.snippet, "See you at the dock");
        assert!(row.subject.is_empty());
        assert!(row.from.email.is_empty());
        assert!(row.unread);
        assert!(row.flagged);
        assert_eq!(row.category, Category::Social);
        assert_eq!(row.mailbox, MailboxId::new("inbox"));
    }

    #[test]
    fn graph_delta_becomes_an_envelope() {
        let envelopes = graph_delta(include_str!("../fixtures/graph-delta.json")).unwrap();
        assert_eq!(envelopes.len(), 1);
        let message = &envelopes[0];
        assert_eq!(message.id, MessageId::new("AAMk"));
        assert_eq!(message.thread, ThreadId::new("AAQk"));
        assert_eq!(message.from.email, "ada@example.com");
        assert_eq!(message.to[0].email, "me@example.com");
        assert!(!message.flags.seen);
        assert!(message.flags.flagged);
        assert_eq!(message.attachment_count, 1);
        assert_eq!(message.snippet, "See you at the dock");
    }

    #[test]
    fn a_broken_document_is_an_error() {
        assert!(jmap_email_get("[]").is_err());
        assert!(gmail_history("{", &AccountId::new("ada")).is_err());
        assert!(graph_delta(r#"{"value":[]}"#).is_ok());
    }
}
