//! IMAP mailbox roles. The socket stays outside this crate.
//!
//! `imap-codec` 1.0.0 parses LIST. SPECIAL-USE attributes win over a
//! mailbox name. Nothing here connects.

mod list;

pub use list::{ListedMailbox, mailbox_role, parse_list};

/// Failure while reading one IMAP response.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a complete IMAP response.
    #[error("imap response could not be read")]
    Response,
}
