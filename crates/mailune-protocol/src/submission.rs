//! What the UI asks the core to do.
//!
//! Variants follow the mock's `Actions` callbacks, plus `Summarize` from the
//! architecture data flow. A password never appears here: sign-in secrets
//! stay in the host keychain and are not a value the core logs or replays.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::ids::{AccountId, MailboxId, ThreadId};

/// A typed request from a surface. The core applies it; the model only
/// proposes values of this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Submission {
    /// Reload the visible mailbox.
    Refresh,
    /// Open one conversation.
    OpenThread {
        /// Which conversation.
        thread: ThreadId,
    },
    /// Send a draft. Confirmation still happens in the app before this is applied.
    Send {
        /// Recipients.
        to: Vec<Address>,
        /// Subject line.
        subject: String,
        /// Plain body. Rich MIME is built by the MIME crate.
        body: String,
    },
    /// Store a draft without sending.
    SaveDraft {
        /// Recipients so far.
        to: Vec<Address>,
        /// Subject line.
        subject: String,
        /// Plain body.
        body: String,
    },
    /// Archive these conversations.
    Archive {
        /// Conversations to archive.
        threads: Vec<ThreadId>,
    },
    /// Move these conversations to Trash. Confirmation still happens in the app.
    Delete {
        /// Conversations to delete.
        threads: Vec<ThreadId>,
    },
    /// Mark these conversations as spam.
    Spam {
        /// Conversations to mark.
        threads: Vec<ThreadId>,
    },
    /// Hide a conversation until a preset (`later`, `tomorrow`, …).
    Snooze {
        /// Which conversation.
        thread: ThreadId,
        /// The mock passes the preset as a string the UI already chose.
        preset: String,
    },
    /// Move a conversation into a mailbox.
    MoveTo {
        /// Which conversation.
        thread: ThreadId,
        /// Destination mailbox.
        mailbox: MailboxId,
    },
    /// Toggle `\Flagged` on a conversation.
    ToggleStar {
        /// Which conversation.
        thread: ThreadId,
    },
    /// Toggle unread on a conversation.
    ToggleRead {
        /// Which conversation.
        thread: ThreadId,
    },
    /// Add or remove a label.
    ApplyLabel {
        /// Which conversation.
        thread: ThreadId,
        /// Label text from the UI.
        label: String,
    },
    /// Run a search query.
    Search {
        /// The query string, including tokens such as `from:`.
        query: String,
    },
    /// Show another account's mailboxes.
    SwitchAccount {
        /// Which account.
        account: AccountId,
    },
    /// Reverse the last reversible command.
    Undo,
    /// Download one attachment by the name the UI shows.
    Download {
        /// File name from the mock. A content id replaces this when blobs exist.
        name: String,
    },
    /// Start sign-in. The password is not in this value.
    SignIn {
        /// Account address.
        email: String,
    },
    /// Start account creation. The password is not in this value.
    SignUp {
        /// Display name.
        name: String,
        /// Account address.
        email: String,
    },
    /// Leave the current session.
    SignOut,
    /// Pick a plan by its key (`free`, `plus`, `team`).
    ChoosePlan {
        /// Plan key from the mock.
        plan: String,
    },
    /// Switch UI language. Empty means English, matching the mock.
    SetLanguage {
        /// Language code, or empty for English.
        code: String,
    },
    /// Ask for a summary of one conversation. Encrypted mail stays local.
    Summarize {
        /// Which conversation.
        thread: ThreadId,
    },
}
