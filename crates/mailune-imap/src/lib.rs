//! IMAP mailbox roles, sync over a session, and an in-memory scripted server.
//!
//! `imap-codec` 1.0.0 parses LIST and FETCH. SPECIAL-USE attributes win over
//! a mailbox name. The session drives any `Read + Write` stream; tests use the
//! scripted server over bytes the caller already holds. Nothing here connects.

mod idle;
mod incremental;
mod list;
mod mutate;
mod script;
mod session;
mod sync;

pub use idle::{Backoff, IdleEvent, IdleWatch, Tick, parse_idle_line};
pub use incremental::{Delta, FlagChange, Resync, SyncState};
pub use list::{ListedMailbox, mailbox_role, parse_list};
pub use mutate::{Appended, FlagOp, UidMap};
pub use script::{FIXTURE, ScriptMailbox, ScriptMessage, Scripted};
pub use session::{Config, Connection, MemStream, SharedServer};
pub use sync::{MessageMeta, Selected, SyncBatch, Window};

/// Failure while reading one IMAP response or driving a session.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a complete IMAP response.
    #[error("imap response could not be read")]
    Response,
    /// The in-memory stream ended before the tagged reply.
    #[error("the imap session stopped")]
    Session,
    /// The server answered NO or BAD.
    #[error("the server refused the command")]
    Rejected,
    /// A name, flag or UID list cannot be sent safely.
    #[error("the argument cannot be sent to the server")]
    Argument,
    /// The server lacks an extension this operation needs to be safe.
    #[error("the server does not support this operation")]
    Unsupported,
}
