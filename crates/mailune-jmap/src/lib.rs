//! JMAP (RFC 8620, RFC 8621) mail adapter.
//!
//! Requests go out through the injected [`mailune_protocol::Http`]
//! transport; this crate never opens a socket. Parsed records are protocol
//! types plus the arrival time and mailbox set, which is what the store's
//! repository takes. Wiring the two together is `mailune-app`'s job.

mod client;
mod extras;
mod mail;
mod mutate;
mod push;
mod wire;

pub use client::{JmapClient, Session, SyncBatch};
pub use extras::{MaskedEmail, SieveScript};
pub use mail::{Changes, JmapEmail, JmapMailbox, JmapThread, QueryResult};
pub use push::{
    PushEvent, StateChange, WebSocket, WsMessage, event_source, push_disable, push_enable,
    ws_message,
};

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
    /// The server refused a create or update (`notCreated`, `notUpdated`).
    #[error("JMAP server refused the change: {kind}")]
    Rejected {
        /// The SetError `type`.
        kind: String,
    },
    /// The session does not offer a mail account.
    #[error("JMAP session has no mail account")]
    NoMailAccount,
    /// A call was made before the session was fetched.
    #[error("JMAP session not loaded")]
    NoSession,
    /// The session offers no EventSource URL.
    #[error("JMAP session offers no push")]
    NoPush,
    /// The push stream failed or is not an event stream.
    #[error("JMAP push stream: {0}")]
    Stream(String),
}
