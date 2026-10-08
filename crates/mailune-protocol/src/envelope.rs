//! Message envelope: addresses, subject, stamp and flags.
//!
//! Body, quote and attachment bytes are MIME and stay out of this crate.
//! The UI's `security` string is either transport (`Encrypted (TLS)`,
//! `Not encrypted`) or the draft flag (`Draft`); the flag lives on [`Flags`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::flags::Flags;
use crate::ids::{MessageId, ThreadId};

/// How the message travelled. End-to-end crypto is a later crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransportSecurity {
    /// The mock's `Not encrypted`.
    Clear,
    /// The mock's `Encrypted (TLS)`.
    Tls,
}

/// Headers and flags for one message, enough for a list and a reader chrome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Envelope {
    /// This message.
    pub id: MessageId,
    /// Conversation it belongs to.
    pub thread: ThreadId,
    /// `From`.
    pub from: Address,
    /// `To`, split from the mock's comma-separated string.
    pub to: Vec<Address>,
    /// `Cc`.
    pub cc: Vec<Address>,
    /// Subject. The mock stores it on the thread; a message still has one.
    pub subject: String,
    /// The mock's `stamp` (a display string, not a parsed timestamp).
    pub stamp: String,
    /// One-line preview. The full body is not part of the envelope.
    pub snippet: String,
    /// Standard flags and keywords.
    pub flags: Flags,
    /// How many attachments the message has. Bytes live in the blob store.
    pub attachment_count: u32,
    /// Transport security shown in the reader.
    pub transport: TransportSecurity,
}
