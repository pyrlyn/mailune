//! IMAP mailbox roles, an in-memory scripted server, and sync.
//!
//! `imap-codec` 1.0.0 parses LIST, FETCH, and IDLE updates. SPECIAL-USE
//! attributes win over a mailbox name. The scripted server speaks those
//! commands over bytes the caller already holds. Nothing here connects.

mod idle;
mod list;
mod mutate;
mod partial;
mod script;
mod session;
mod sync;

pub use idle::{Clock, IdleUpdate, IdleWatch, ManualClock};
pub use list::{ListedMailbox, mailbox_role, parse_list};
pub use mutate::{append_message, move_uid, store_flags};
pub use partial::{binary_bytes, peek_bytes};
pub use script::{FIXTURE, MailboxMessage, Scripted};
pub use session::{Config, Connection, MemStream, SelectedMailbox};
pub use sync::{DayWindow, SyncBatch, SyncedMessage, UidDelta, incremental_sync, initial_sync};

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
    /// The scripted stream has nothing more to read, and the session is still open.
    #[error("the imap session is idle")]
    Idle,
    /// The injected clock has not reached the reconnect time.
    #[error("the idle session is waiting to reconnect")]
    Waiting,
}
