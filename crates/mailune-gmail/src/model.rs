//! Gmail API resources (labels, messages in `metadata` format, history
//! records) mapped onto protocol types.

use mailune_protocol::{
    Address, Envelope, Flags, MailboxId, MailboxRole, MessageId, ThreadId, TransportSecurity,
};
use serde::Deserialize;

/// A label, which this client treats as a mailbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmailLabel {
    /// Label id (`INBOX`, `Label_12`).
    pub id: MailboxId,
    /// Display name.
    pub name: String,
    /// Role for the system labels that have one.
    pub role: Option<MailboxRole>,
}

/// A message in `metadata` format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmailMessage {
    /// Headers and flags.
    pub envelope: Envelope,
    /// `internalDate` in seconds since the epoch.
    pub received_at: i64,
    /// Labels other than `UNREAD` and `STARRED`.
    pub mailboxes: Vec<MailboxId>,
    /// The message's `historyId`.
    pub history_id: u64,
}

/// What `history.list` reported since a historyId.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryDiff {
    /// Messages added or relabelled: fetch these again.
    pub changed: Vec<MessageId>,
    /// Messages deleted.
    pub deleted: Vec<MessageId>,
    /// historyId to resume from.
    pub history_id: String,
}

#[derive(Deserialize)]
pub(crate) struct LabelList {
    #[serde(default)]
    pub(crate) labels: Vec<WireLabel>,
}

#[derive(Deserialize)]
pub(crate) struct WireLabel {
    id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThreadList {
    #[serde(default)]
    pub(crate) threads: Vec<ThreadRef>,
}

#[derive(Deserialize)]
pub(crate) struct ThreadRef {
    pub(crate) id: String,
}

#[derive(Deserialize)]
pub(crate) struct WireThread {
    #[serde(default)]
    pub(crate) messages: Vec<WireMessage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireMessage {
    id: String,
    thread_id: String,
    #[serde(default)]
    label_ids: Vec<String>,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    history_id: Option<String>,
    #[serde(default)]
    internal_date: Option<String>,
    #[serde(default)]
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(default)]
    headers: Vec<Header>,
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    #[serde(default)]
    filename: String,
}

#[derive(Deserialize)]
struct Header {
    name: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryPage {
    #[serde(default)]
    history: Vec<HistoryRecord>,
    history_id: String,
    #[serde(default)]
    pub(crate) next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryRecord {
    #[serde(default)]
    messages_added: Vec<Touched>,
    #[serde(default)]
    messages_deleted: Vec<Touched>,
    #[serde(default)]
    labels_added: Vec<Touched>,
    #[serde(default)]
    labels_removed: Vec<Touched>,
}

#[derive(Deserialize)]
struct Touched {
    message: IdOnly,
}

#[derive(Deserialize)]
struct IdOnly {
    id: String,
}

impl From<WireLabel> for GmailLabel {
    fn from(wire: WireLabel) -> Self {
        let role = match wire.id.as_str() {
            "INBOX" => Some(MailboxRole::Inbox),
            "SENT" => Some(MailboxRole::Sent),
            "DRAFT" => Some(MailboxRole::Drafts),
            "TRASH" => Some(MailboxRole::Trash),
            "SPAM" => Some(MailboxRole::Spam),
            "STARRED" => Some(MailboxRole::Starred),
            "IMPORTANT" => Some(MailboxRole::Important),
            _ => None,
        };
        Self {
            id: MailboxId::new(wire.id),
            name: wire.name,
            role,
        }
    }
}

impl From<WireMessage> for GmailMessage {
    fn from(wire: WireMessage) -> Self {
        let has = |label: &str| wire.label_ids.iter().any(|have| have == label);
        let (headers, attachments) = match wire.payload {
            Some(payload) => {
                let attachments = payload
                    .parts
                    .iter()
                    .filter(|part| !part.filename.is_empty())
                    .count();
                (payload.headers, attachments)
            }
            None => (Vec::new(), 0),
        };
        let header = |name: &str| {
            headers
                .iter()
                .find(|header| header.name.eq_ignore_ascii_case(name))
                .map(|header| header.value.as_str())
                .unwrap_or_default()
        };
        let millis: i64 = wire
            .internal_date
            .as_deref()
            .and_then(|text| text.parse().ok())
            .unwrap_or(0);
        let envelope = Envelope {
            id: MessageId::new(&wire.id),
            thread: ThreadId::new(&wire.thread_id),
            from: address_list(header("From"))
                .into_iter()
                .next()
                .unwrap_or(Address {
                    name: None,
                    email: String::new(),
                }),
            to: address_list(header("To")),
            cc: address_list(header("Cc")),
            subject: header("Subject").to_string(),
            stamp: header("Date").to_string(),
            snippet: wire.snippet.clone(),
            flags: Flags {
                seen: !has("UNREAD"),
                flagged: has("STARRED"),
                draft: has("DRAFT"),
                answered: false,
                deleted: has("TRASH"),
                keywords: Vec::new(),
            },
            attachment_count: u32::try_from(attachments).unwrap_or(u32::MAX),
            transport: TransportSecurity::Tls,
        };
        Self {
            envelope,
            received_at: millis.div_euclid(1000),
            mailboxes: wire
                .label_ids
                .iter()
                .filter(|label| !matches!(label.as_str(), "UNREAD" | "STARRED"))
                .map(MailboxId::new)
                .collect(),
            history_id: wire
                .history_id
                .as_deref()
                .and_then(|text| text.parse().ok())
                .unwrap_or(0),
        }
    }
}

impl HistoryPage {
    /// Folds this page into `diff`. A message deleted later in the page is
    /// not fetched again.
    pub(crate) fn fold_into(self, diff: &mut HistoryDiff) {
        for record in self.history {
            let changed = record
                .messages_added
                .into_iter()
                .chain(record.labels_added)
                .chain(record.labels_removed);
            for touched in changed {
                let id = MessageId::new(touched.message.id);
                if !diff.changed.contains(&id) {
                    diff.changed.push(id);
                }
            }
            for touched in record.messages_deleted {
                let id = MessageId::new(touched.message.id);
                diff.changed.retain(|have| have != &id);
                if !diff.deleted.contains(&id) {
                    diff.deleted.push(id);
                }
            }
        }
        diff.history_id = self.history_id;
    }
}

/// Splits an RFC 5322 address-list header value into addresses. Commas
/// inside quotes or angle brackets do not split.
pub(crate) fn address_list(value: &str) -> Vec<Address> {
    let mut out = Vec::new();
    let mut start = 0;
    let (mut quoted, mut angle) = (false, false);
    for (index, ch) in value.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '<' if !quoted => angle = true,
            '>' if !quoted => angle = false,
            ',' if !quoted && !angle => {
                out.extend(one_address(&value[start..index]));
                start = index + 1;
            }
            _ => {}
        }
    }
    out.extend(one_address(&value[start..]));
    out
}

fn one_address(text: &str) -> Option<Address> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let (Some(open), Some(close)) = (text.rfind('<'), text.rfind('>'))
        && open < close
    {
        let name = text[..open].trim().trim_matches('"').trim();
        return Some(Address {
            name: (!name.is_empty()).then(|| name.to_string()),
            email: text[open + 1..close].trim().to_string(),
        });
    }
    Some(Address {
        name: None,
        email: text.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{Address, MessageId};

    use super::{HistoryDiff, HistoryPage, address_list};

    #[test]
    fn address_lists_split_outside_quotes_and_brackets() {
        let parsed =
            address_list(r#""Lovelace, Ada" <ada@example.com>, bob@example.com, <c@x.io>"#);
        assert_eq!(
            parsed,
            [
                Address {
                    name: Some("Lovelace, Ada".into()),
                    email: "ada@example.com".into()
                },
                Address {
                    name: None,
                    email: "bob@example.com".into()
                },
                Address {
                    name: None,
                    email: "c@x.io".into()
                },
            ]
        );
        assert!(address_list("  ").is_empty());
    }

    #[test]
    fn the_shared_history_fixture_folds_into_a_diff() {
        let json = include_str!("../../mailune-fixture/fixtures/gmail-history.json");
        let page: HistoryPage = serde_json::from_str(json).unwrap();
        let mut diff = HistoryDiff::default();
        page.fold_into(&mut diff);
        assert_eq!(diff.changed, [MessageId::new("m1")]);
        assert_eq!(diff.history_id, "100");
    }

    #[test]
    fn a_deleted_message_is_not_fetched_again() {
        let page: HistoryPage = serde_json::from_str(
            r#"{"historyId":"9","history":[
                {"id":"7","messagesAdded":[{"message":{"id":"a","threadId":"t"}}]},
                {"id":"8","labelsRemoved":[{"message":{"id":"b","threadId":"t"},"labelIds":["UNREAD"]}]},
                {"id":"9","messagesDeleted":[{"message":{"id":"a","threadId":"t"}}]}]}"#,
        )
        .unwrap();
        let mut diff = HistoryDiff::default();
        page.fold_into(&mut diff);
        assert_eq!(diff.changed, [MessageId::new("b")]);
        assert_eq!(diff.deleted, [MessageId::new("a")]);
    }
}
