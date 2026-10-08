//! IMAP mailbox roles and an in-memory scripted server.
//!
//! `imap-codec` 1.0.0 parses LIST. SPECIAL-USE attributes win over a
//! mailbox name. The scripted server speaks greeting, CAPABILITY, LOGIN,
//! SELECT, and one FETCH over bytes the caller already holds. Nothing
//! here connects.

mod list;
mod script;

pub use list::{ListedMailbox, mailbox_role, parse_list};
pub use script::{FIXTURE, Scripted};

/// Failure while reading one IMAP response.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a complete IMAP response.
    #[error("imap response could not be read")]
    Response,
}
