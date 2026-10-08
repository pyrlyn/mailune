//! What the core tells the UI. A `Notice` is the conflict signal from the
//! architecture; a `Snapshot` is the fresh thread list the change feed pushes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::ids::{AccountId, MailboxId, ThreadId};

/// Triage tab. The mock stores `cat` as `0` Primary, `1` Social,
/// `2` Promotions, `3` Updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// `cat` 0.
    Primary,
    /// `cat` 1.
    Social,
    /// `cat` 2.
    Promotions,
    /// `cat` 3.
    Updates,
}

/// One row in the thread list. Initials, tint and selection are UI state
/// and are not stored here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ThreadRow {
    /// Conversation id.
    pub id: ThreadId,
    /// Owning account.
    pub account: AccountId,
    /// Sender shown on the row.
    pub from: Address,
    /// Subject.
    pub subject: String,
    /// One-line preview.
    pub snippet: String,
    /// Display stamp.
    pub stamp: String,
    /// How many messages the conversation holds.
    pub message_count: u32,
    /// The row is unread. Not the same bit as one message's `\Seen`.
    pub unread: bool,
    /// Starred.
    pub flagged: bool,
    /// Marked important.
    pub important: bool,
    /// Pinned to the top of the list.
    pub pinned: bool,
    /// Hidden until a snooze wakes it.
    pub snoozed: bool,
    /// The conversation is a draft.
    pub draft: bool,
    /// At least one message has an attachment.
    pub has_attachment: bool,
    /// Triage tab.
    pub category: Category,
    /// Mailbox the row was listed from.
    pub mailbox: MailboxId,
    /// Label texts. Colours stay in the UI.
    pub labels: Vec<String>,
}

/// A fact the UI renders. Surfaces do not invent these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    /// A conflict, rollback or other sentence the UI shows as a notice.
    Notice {
        /// Text the UI can show. Not a message body.
        message: String,
    },
    /// The current page of the thread list.
    Snapshot {
        /// Rows in list order.
        threads: Vec<ThreadRow>,
    },
}
