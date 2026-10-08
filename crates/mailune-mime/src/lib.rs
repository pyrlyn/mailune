//! MIME messages: parse bytes into parts, and build a message back.
//!
//! `mail-parser` owns the grammar. This crate owns the domain shape the
//! rest of Mailune sees. Nothing here opens a socket.

mod build;
mod link;
mod message;
mod parse;
mod quirks;
mod remote;

pub use build::{Attachment, Outbound, build};
pub use link::{LinkCheck, inspect_link};
pub use message::{Body, MimeMessage, Part, PartRole};
pub use parse::parse;
pub use quirks::{
    Auth, Endpoint, ImapUsername, Provider, Quirks, SpecialMailbox, Transport, all, by_imap_host,
    quirks,
};
pub use remote::{RemoteContent, inspect_url};

/// Failure from parsing or building one message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not an RFC 5322 message.
    #[error("the bytes are not an RFC 5322 message")]
    Parse,
    /// `mail-builder` could not write the message.
    #[error("the message could not be encoded")]
    Build(#[source] std::io::Error),
}
