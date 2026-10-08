//! Special-use mailboxes. The variants are the folder keys in the mail UI
//! mock (`inbox`, `starred`, …). A user-created folder has no role.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A mailbox the client treats specially. Names and icons stay in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MailboxRole {
    /// `inbox`.
    Inbox,
    /// `starred`. Virtual: flagged messages.
    Starred,
    /// `snoozed`.
    Snoozed,
    /// `important`.
    Important,
    /// `sent`.
    Sent,
    /// `drafts`.
    Drafts,
    /// `scheduled`. Send-later, not yet handed to the server.
    Scheduled,
    /// `outbox`. Waiting to be sent.
    Outbox,
    /// `archive`.
    Archive,
    /// `spam`.
    Spam,
    /// `trash`.
    Trash,
    /// `all`.
    All,
}
