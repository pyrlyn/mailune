//! MIME messages: parse bytes into parts, and build a message back.
//!
//! `mail-parser` owns the grammar. This crate owns the domain shape the
//! rest of Mailune sees. Nothing here opens a socket.

mod auth;
mod autocrypt;
mod build;
mod calendar;
mod chunk;
mod contacts;
mod export;
mod extract;
mod html;
mod link;
mod message;
mod parse;
mod quirks;
mod remote;
mod screen;
mod unsubscribe;

pub use auth::{
    AuthBadge, AuthResult, DkimDns, DkimVerdict, MethodBadge, authentication_badge, verify_dkim,
};
pub use autocrypt::{AutocryptKey, PreferEncrypt, gossip_keys, sender_autocrypt};
pub use build::{Attachment, Outbound, build};
pub use calendar::{Invite, InviteKind, parse_invite};
pub use chunk::{TextChunk, chunk_plain};
pub use contacts::{RankedContact, Sighting, rank_contacts};
pub use export::{write_eml, write_mbox};
pub use extract::{Fact, FactKind, extract_facts};
pub use html::{SanitizedHtml, sanitize_html};
pub use link::{LinkCheck, inspect_link};
pub use message::{Body, MimeMessage, Part, PartRole};
pub use parse::parse;
pub use quirks::{
    Auth, Endpoint, ImapUsername, Provider, Quirks, SpecialMailbox, Transport, all, by_imap_host,
    quirks,
};
pub use remote::{RemoteContent, inspect_url};
pub use screen::{SenderScreen, screen_sender};
pub use unsubscribe::{Unsubscribe, parse_list_unsubscribe};

/// Failure from parsing or building one message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not an RFC 5322 message.
    #[error("the bytes are not an RFC 5322 message")]
    Parse,
    /// `mail-builder` could not write the message.
    #[error("the message could not be encoded")]
    Build(#[source] std::io::Error),
    /// The HTML fragment could not be turned into a plain-text alternative.
    #[error("html could not be turned into text")]
    Html,
    /// The Authentication-Results header had no server id.
    #[error("authentication-results could not be read")]
    AuthenticationResults,
    /// The bytes were not a REQUEST or REPLY invite.
    #[error("the calendar invite could not be read")]
    Calendar,
}
