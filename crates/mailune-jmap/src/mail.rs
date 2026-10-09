//! RFC 8621 records: Mailbox, Email, Thread, and the `/changes` and
//! `/query` results, mapped onto protocol types.

use std::collections::BTreeMap;

use mailune_core::parse_rfc3339;
use mailune_protocol::{
    Address, Envelope, Flags, MailboxId, MailboxRole, MessageId, ThreadId, TransportSecurity,
};
use serde::Deserialize;

/// Email properties fetched for a list and a reader header.
pub(crate) const EMAIL_PROPERTIES: [&str; 11] = [
    "id",
    "threadId",
    "mailboxIds",
    "keywords",
    "from",
    "to",
    "cc",
    "subject",
    "preview",
    "receivedAt",
    "hasAttachment",
];

/// A mailbox from `Mailbox/get`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JmapMailbox {
    /// Server id.
    pub id: MailboxId,
    /// Display name.
    pub name: String,
    /// Parent mailbox.
    pub parent: Option<MailboxId>,
    /// Special-use role mapped onto the client's roles.
    pub role: Option<MailboxRole>,
}

/// An email from `Email/get`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JmapEmail {
    /// Headers and flags.
    pub envelope: Envelope,
    /// `receivedAt` in seconds since the epoch; 0 when the server sent none.
    pub received_at: i64,
    /// Mailboxes the email is in.
    pub mailboxes: Vec<MailboxId>,
}

/// A thread from `Thread/get`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JmapThread {
    /// Thread id.
    pub id: ThreadId,
    /// Emails in the thread, oldest first as the server orders them.
    pub emails: Vec<MessageId>,
}

/// An `Email/changes` result.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Changes {
    /// State the changes start from.
    pub old_state: String,
    /// State to ask from next time.
    pub new_state: String,
    /// More changes are waiting after `new_state`.
    pub has_more_changes: bool,
    /// New ids.
    pub created: Vec<MessageId>,
    /// Changed ids.
    pub updated: Vec<MessageId>,
    /// Removed ids.
    pub destroyed: Vec<MessageId>,
}

/// An `Email/query` result.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    /// Query state, for `Email/queryChanges`.
    pub query_state: String,
    /// Matching ids in sort order.
    pub ids: Vec<MessageId>,
    /// Total matches, when the server computed it.
    #[serde(default)]
    pub total: Option<u64>,
}

#[derive(Deserialize)]
pub(crate) struct GetResult<T> {
    pub(crate) state: String,
    pub(crate) list: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireMailbox {
    id: String,
    name: String,
    parent_id: Option<String>,
    role: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireEmail {
    id: String,
    thread_id: String,
    #[serde(default)]
    mailbox_ids: BTreeMap<String, bool>,
    #[serde(default)]
    keywords: BTreeMap<String, bool>,
    from: Option<Vec<WireAddress>>,
    to: Option<Vec<WireAddress>>,
    cc: Option<Vec<WireAddress>>,
    subject: Option<String>,
    preview: Option<String>,
    received_at: Option<String>,
    has_attachment: Option<bool>,
}

#[derive(Deserialize)]
struct WireAddress {
    name: Option<String>,
    email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireThread {
    id: String,
    email_ids: Vec<String>,
}

impl From<WireMailbox> for JmapMailbox {
    fn from(wire: WireMailbox) -> Self {
        Self {
            id: MailboxId::new(wire.id),
            name: wire.name,
            parent: wire.parent_id.map(MailboxId::new),
            role: wire.role.as_deref().and_then(role),
        }
    }
}

impl From<WireThread> for JmapThread {
    fn from(wire: WireThread) -> Self {
        Self {
            id: ThreadId::new(wire.id),
            emails: wire.email_ids.into_iter().map(MessageId::new).collect(),
        }
    }
}

impl From<WireEmail> for JmapEmail {
    fn from(wire: WireEmail) -> Self {
        let set = |name: &str| wire.keywords.get(name).copied().unwrap_or(false);
        let flags = Flags {
            seen: set("$seen"),
            flagged: set("$flagged"),
            draft: set("$draft"),
            answered: set("$answered"),
            deleted: false,
            keywords: wire
                .keywords
                .iter()
                .filter(|(name, on)| **on && !SYSTEM_KEYWORDS.contains(&name.as_str()))
                .map(|(name, _)| name.clone())
                .collect(),
        };
        let stamp = wire.received_at.unwrap_or_default();
        let envelope = Envelope {
            id: MessageId::new(wire.id),
            thread: ThreadId::new(wire.thread_id),
            from: wire
                .from
                .into_iter()
                .flatten()
                .next()
                .map(address)
                .unwrap_or_else(|| Address {
                    name: None,
                    email: String::new(),
                }),
            to: wire.to.into_iter().flatten().map(address).collect(),
            cc: wire.cc.into_iter().flatten().map(address).collect(),
            subject: wire.subject.unwrap_or_default(),
            snippet: wire.preview.unwrap_or_default(),
            flags,
            attachment_count: u32::from(wire.has_attachment.unwrap_or(false)),
            transport: TransportSecurity::Tls,
            stamp: stamp.clone(),
        };
        Self {
            received_at: parse_rfc3339(&stamp).unwrap_or(0),
            mailboxes: wire
                .mailbox_ids
                .into_iter()
                .filter(|(_, on)| *on)
                .map(|(id, _)| MailboxId::new(id))
                .collect(),
            envelope,
        }
    }
}

const SYSTEM_KEYWORDS: [&str; 4] = ["$seen", "$flagged", "$draft", "$answered"];

fn address(wire: WireAddress) -> Address {
    Address {
        name: wire.name.filter(|name| !name.is_empty()),
        email: wire.email,
    }
}

/// RFC 8621 §2 roles (IANA "IMAP Mailbox Name Attributes") onto the client's.
fn role(name: &str) -> Option<MailboxRole> {
    match name {
        "inbox" => Some(MailboxRole::Inbox),
        "archive" => Some(MailboxRole::Archive),
        "drafts" => Some(MailboxRole::Drafts),
        "sent" => Some(MailboxRole::Sent),
        "trash" => Some(MailboxRole::Trash),
        "junk" => Some(MailboxRole::Spam),
        "important" => Some(MailboxRole::Important),
        "flagged" => Some(MailboxRole::Starred),
        "all" => Some(MailboxRole::All),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{MailboxId, MailboxRole};

    use super::{GetResult, JmapEmail, JmapMailbox, WireEmail, WireMailbox};

    #[test]
    fn the_shared_fixture_maps_like_the_fixture_crate() {
        let json = include_str!("../../mailune-fixture/fixtures/jmap-email-get.json");
        let ours: GetResult<WireEmail> = serde_json::from_str(json).unwrap();
        let ours: Vec<JmapEmail> = ours.list.into_iter().map(JmapEmail::from).collect();
        let fixture = mailune_fixture::jmap_email_get(json).unwrap();
        let envelopes: Vec<_> = ours.iter().map(|email| email.envelope.clone()).collect();
        assert_eq!(envelopes, fixture);
        assert_eq!(ours[0].received_at, 1_772_366_400);
    }

    #[test]
    fn custom_keywords_survive_and_roles_map() {
        let email: WireEmail = serde_json::from_str(
            r#"{"id":"m","threadId":"t","mailboxIds":{"mb1":true,"mb2":false},
                "keywords":{"$seen":true,"work":true,"old":false}}"#,
        )
        .unwrap();
        let email = JmapEmail::from(email);
        assert!(email.envelope.flags.seen);
        assert_eq!(email.envelope.flags.keywords, ["work"]);
        assert_eq!(email.mailboxes, [MailboxId::new("mb1")]);
        let junk: WireMailbox =
            serde_json::from_str(r#"{"id":"j","name":"Junk","parentId":null,"role":"junk"}"#)
                .unwrap();
        assert_eq!(JmapMailbox::from(junk).role, Some(MailboxRole::Spam));
    }
}
