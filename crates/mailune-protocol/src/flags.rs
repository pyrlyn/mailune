//! Per-message flags. Keywords carry labels and other server-specific
//! tokens that are not the five standard bits.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Flags stored with a message. `seen` is the inverse of the list's unread
/// mark only for a one-message thread; a thread row keeps its own `unread`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Flags {
    /// `\Seen`.
    pub seen: bool,
    /// `\Flagged`. The UI calls this a star.
    pub flagged: bool,
    /// `\Draft`.
    pub draft: bool,
    /// `\Answered`.
    pub answered: bool,
    /// `\Deleted`. Distinct from the Trash mailbox.
    pub deleted: bool,
    /// Server keywords and labels (`snoozed`, a user label).
    pub keywords: Vec<String>,
}
