//! A parsed message: headers plus the parts a reader can show.
//!
//! Addresses are the contract type. Body bytes stay here, not on the envelope.

use mailune_protocol::Address;

/// One RFC 5322 message after MIME decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeMessage {
    /// `Subject`, or empty when the header was absent.
    pub subject: String,
    /// `From`.
    pub from: Vec<Address>,
    /// `To`.
    pub to: Vec<Address>,
    /// `Cc`.
    pub cc: Vec<Address>,
    /// `Message-ID` without the angle brackets.
    pub message_id: Option<String>,
    /// `In-Reply-To`, one id per entry, without angle brackets.
    pub in_reply_to: Vec<String>,
    /// `References`, one id per entry, without angle brackets.
    pub references: Vec<String>,
    /// Leaf parts. Multipart containers are not included.
    pub parts: Vec<Part>,
}

/// One MIME entity that carries bytes or text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Where the parser placed this entity.
    pub role: PartRole,
    /// `type/subtype`, lowercased.
    pub media_type: String,
    /// Encoding Standard name for the charset label, when the part declared one.
    ///
    /// `iso-8859-1` is stored as `windows-1252` because that is the encoding
    /// the standard uses for that label.
    pub charset: Option<String>,
    /// `Content-Disposition` filename, or the `name` parameter.
    pub filename: Option<String>,
    /// `Content-ID` without the angle brackets.
    pub content_id: Option<String>,
    /// Decoded body.
    pub body: Body,
}

/// What the part is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartRole {
    /// `text/plain` body.
    Text,
    /// `text/html` body.
    Html,
    /// A file the reader can save.
    Attachment,
    /// A related part (an inline image).
    Inline,
    /// A leaf the parser did not classify as a body or an attachment.
    Other,
}

/// Decoded part bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// Unicode text. The charset has already been applied.
    Text(String),
    /// An attachment or any other binary part.
    Bytes(Vec<u8>),
}
