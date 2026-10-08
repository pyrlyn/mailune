//! JMAP (RFC 8620, RFC 8621) mail adapter.
//!
//! Requests go out through the injected [`mailune_protocol::Http`]
//! transport; this crate never opens a socket. Parsed records are protocol
//! types plus the arrival time and mailbox set, which is what the store's
//! repository takes. Wiring the two together is `mailune-app`'s job.

mod client;
mod mail;
mod wire;

pub use client::{JmapClient, Session, SyncBatch};
pub use mail::{Changes, JmapEmail, JmapMailbox, JmapThread, QueryResult};

/// Failure returned by the JMAP adapter. No variant carries the token.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The transport did not return a response.
    #[error(transparent)]
    Transport(#[from] mailune_protocol::Error),
    /// The server answered with a non-success status.
    #[error("JMAP server answered {status}")]
    Status {
        /// HTTP status code.
        status: u16,
    },
    /// The body is not the JSON shape this adapter expects.
    #[error("JMAP response is not valid: {0}")]
    Json(String),
    /// The server returned a method-level error such as `cannotCalculateChanges`.
    #[error("JMAP method error: {kind}")]
    Method {
        /// The error `type` from the server.
        kind: String,
    },
    /// The session does not offer a mail account.
    #[error("JMAP session has no mail account")]
    NoMailAccount,
    /// A call was made before the session was fetched.
    #[error("JMAP session not loaded")]
    NoSession,
}
