//! Graph resources (`mailFolder`, `message` with `$select`, OData pages)
//! mapped onto protocol types.

use mailune_core::parse_rfc3339;
use mailune_protocol::{
    Address, Envelope, Flags, MailboxId, MailboxRole, MessageId, ThreadId, TransportSecurity,
};
use serde::Deserialize;
use serde::de::IgnoredAny;

/// Message properties a delta query asks for. Anything else is not sent,
/// which keeps pages small and the body out of the sync path.
pub(crate) const SELECT: &str = "id,conversationId,subject,from,toRecipients,ccRecipients,receivedDateTime,sentDateTime,isRead,isDraft,flag,hasAttachments,bodyPreview,parentFolderId,categories";

/// A mail folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphFolder {
    /// Folder id.
    pub id: MailboxId,
    /// Display name.
    pub name: String,
    /// Parent folder, `None` for a top-level folder.
    pub parent: Option<MailboxId>,
    /// Role, when Graph reports a well-known name.
    pub role: Option<MailboxRole>,
    /// Messages in the folder.
    pub total: u32,
    /// Unread messages in the folder.
    pub unread: u32,
}

/// A message as a delta query returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphMessage {
    /// Headers and flags. Graph categories become keywords.
    pub envelope: Envelope,
    /// `receivedDateTime` in seconds since the epoch.
    pub received_at: i64,
    /// The folder the message is in.
    pub mailboxes: Vec<MailboxId>,
}

/// One OData page.
#[derive(Deserialize)]
pub(crate) struct Page<T> {
    #[serde(default = "Vec::new")]
    pub(crate) value: Vec<T>,
    #[serde(rename = "@odata.nextLink", default)]
    pub(crate) next_link: Option<String>,
    #[serde(rename = "@odata.deltaLink", default)]
    pub(crate) delta_link: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireFolder {
    id: String,
    display_name: String,
    #[serde(default)]
    parent_folder_id: Option<String>,
    #[serde(default)]
    pub(crate) child_folder_count: u32,
    #[serde(default)]
    total_item_count: u32,
    #[serde(default)]
    unread_item_count: u32,
    #[serde(default)]
    well_known_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireMessage {
    pub(crate) id: String,
    /// Present on an item that left the folder (deleted or moved out).
    #[serde(rename = "@removed", default)]
    pub(crate) removed: Option<IgnoredAny>,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    from: Option<Recipient>,
    #[serde(default)]
    to_recipients: Vec<Recipient>,
    #[serde(default)]
    cc_recipients: Vec<Recipient>,
    #[serde(default)]
    received_date_time: Option<String>,
    #[serde(default)]
    sent_date_time: Option<String>,
    #[serde(default)]
    is_read: Option<bool>,
    #[serde(default)]
    is_draft: Option<bool>,
    #[serde(default)]
    flag: Option<FollowupFlag>,
    #[serde(default)]
    has_attachments: Option<bool>,
    #[serde(default)]
    body_preview: Option<String>,
    #[serde(default)]
    parent_folder_id: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recipient {
    email_address: EmailAddress,
}

#[derive(Deserialize)]
struct EmailAddress {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    address: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FollowupFlag {
    #[serde(default)]
    flag_status: String,
}

impl From<WireFolder> for GraphFolder {
    fn from(wire: WireFolder) -> Self {
        let role = match wire.well_known_name.as_deref() {
            Some("inbox") => Some(MailboxRole::Inbox),
            Some("sentitems") => Some(MailboxRole::Sent),
            Some("drafts") => Some(MailboxRole::Drafts),
            Some("deleteditems") => Some(MailboxRole::Trash),
            Some("junkemail") => Some(MailboxRole::Spam),
            Some("archive") => Some(MailboxRole::Archive),
            _ => None,
        };
        Self {
            id: MailboxId::new(wire.id),
            name: wire.display_name,
            parent: wire.parent_folder_id.map(MailboxId::new),
            role,
            total: wire.total_item_count,
            unread: wire.unread_item_count,
        }
    }
}

impl Recipient {
    fn into_address(self) -> Address {
        Address {
            name: self.email_address.name.filter(|name| !name.is_empty()),
            email: self.email_address.address.unwrap_or_default(),
        }
    }
}

impl From<WireMessage> for GraphMessage {
    fn from(wire: WireMessage) -> Self {
        let received = wire.received_date_time.unwrap_or_default();
        let envelope = Envelope {
            id: MessageId::new(&wire.id),
            // A message without a conversation is its own thread.
            thread: ThreadId::new(wire.conversation_id.unwrap_or_else(|| wire.id.clone())),
            from: wire.from.map_or(
                Address {
                    name: None,
                    email: String::new(),
                },
                Recipient::into_address,
            ),
            to: wire
                .to_recipients
                .into_iter()
                .map(Recipient::into_address)
                .collect(),
            cc: wire
                .cc_recipients
                .into_iter()
                .map(Recipient::into_address)
                .collect(),
            subject: wire.subject.unwrap_or_default(),
            stamp: wire.sent_date_time.unwrap_or_else(|| received.clone()),
            snippet: wire.body_preview.unwrap_or_default(),
            flags: Flags {
                seen: wire.is_read.unwrap_or(false),
                flagged: wire.flag.is_some_and(|flag| flag.flag_status == "flagged"),
                draft: wire.is_draft.unwrap_or(false),
                answered: false,
                deleted: false,
                keywords: wire.categories,
            },
            // `hasAttachments` is a yes/no; the count needs a second call.
            attachment_count: u32::from(wire.has_attachments.unwrap_or(false)),
            transport: TransportSecurity::Tls,
        };
        Self {
            envelope,
            received_at: parse_rfc3339(&received).unwrap_or(0),
            mailboxes: wire
                .parent_folder_id
                .map(MailboxId::new)
                .into_iter()
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{MailboxId, MailboxRole, ThreadId};

    use super::{GraphFolder, GraphMessage, Page, WireFolder, WireMessage};

    #[test]
    fn a_message_maps_flags_categories_and_folder() {
        let page: Page<WireMessage> = serde_json::from_str(
            r#"{"value":[{"id":"m1","conversationId":"c1","subject":null,
                "from":{"emailAddress":{"name":"","address":"a@x.io"}},
                "receivedDateTime":"2023-11-14T22:13:20Z","isRead":false,
                "flag":{"flagStatus":"flagged"},"hasAttachments":true,
                "parentFolderId":"f1","categories":["Red"]},
                {"id":"m2","@removed":{"reason":"deleted"}}]}"#,
        )
        .unwrap();
        let mut items = page.value.into_iter();
        let first = GraphMessage::from(items.next().unwrap());
        assert_eq!(first.received_at, 1_700_000_000);
        assert_eq!(first.envelope.thread, ThreadId::new("c1"));
        assert!(first.envelope.flags.flagged && !first.envelope.flags.seen);
        assert_eq!(first.envelope.flags.keywords, ["Red"]);
        assert_eq!(first.envelope.attachment_count, 1);
        assert_eq!(first.envelope.from.name, None);
        assert_eq!(first.mailboxes, [MailboxId::new("f1")]);
        assert!(items.next().unwrap().removed.is_some());
    }

    #[test]
    fn a_well_known_folder_gets_its_role() {
        let wire: WireFolder = serde_json::from_str(
            r#"{"id":"f","displayName":"Posteingang","wellKnownName":"inbox","totalItemCount":3}"#,
        )
        .unwrap();
        let folder = GraphFolder::from(wire);
        assert_eq!(folder.role, Some(MailboxRole::Inbox));
        assert_eq!((folder.total, folder.unread), (3, 0));
    }
}
