//! MIME messages: parse bytes into parts, and build a message back.
//!
//! `mail-parser` owns the grammar. This crate owns the domain shape the
//! rest of Mailune sees. Nothing here opens a socket.

mod message;
mod parse;

pub use message::{Body, MimeMessage, Part, PartRole};
pub use parse::parse;

/// Failure from parsing or building one message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not an RFC 5322 message.
    #[error("the bytes are not an RFC 5322 message")]
    Parse,
}
