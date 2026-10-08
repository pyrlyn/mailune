//! Build one message: plain text, an HTML alternative, one attachment,
//! and the reply headers `In-Reply-To` and `References`.
//!
//! The hostname feature of `mail-builder` stays off. A generated
//! `Message-ID` would read the machine name, and the caller already has an id.

use mail_builder::MessageBuilder;
use mail_builder::headers::address::Address as MailAddress;
use mail_builder::headers::message_id::MessageId;
use mailune_protocol::Address;

use crate::Error;

/// A file attached to an outbound message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// `type/subtype`.
    pub media_type: String,
    /// Filename stored on the part.
    pub filename: String,
    /// Raw bytes. They are not fetched.
    pub bytes: Vec<u8>,
}

/// What [`build`] writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outbound {
    /// `From`.
    pub from: Address,
    /// `To`.
    pub to: Vec<Address>,
    /// `Subject`.
    pub subject: String,
    /// Plain part of the alternative.
    pub text: String,
    /// HTML part of the alternative.
    pub html: String,
    /// The single attachment.
    pub attachment: Attachment,
    /// `Message-ID` without angle brackets.
    pub message_id: String,
    /// `In-Reply-To` without angle brackets.
    pub in_reply_to: String,
    /// `References`, oldest first, without angle brackets.
    pub references: Vec<String>,
}

/// Encode `message` to RFC 5322 bytes.
pub fn build(message: &Outbound) -> Result<Vec<u8>, Error> {
    MessageBuilder::new()
        .from(mail_addr(&message.from))
        .to(mail_list(&message.to))
        .subject(message.subject.as_str())
        .message_id(message.message_id.as_str())
        .in_reply_to(message.in_reply_to.as_str())
        .references(MessageId::new_list(
            message.references.iter().map(String::as_str),
        ))
        .text_body(message.text.as_str())
        .html_body(message.html.as_str())
        .attachment(
            message.attachment.media_type.as_str(),
            message.attachment.filename.as_str(),
            message.attachment.bytes.as_slice(),
        )
        .write_to_vec()
        .map_err(Error::Build)
}

fn mail_addr(addr: &Address) -> MailAddress<'_> {
    MailAddress::new_address(addr.name.as_deref(), addr.email.as_str())
}

fn mail_list(list: &[Address]) -> MailAddress<'_> {
    MailAddress::new_list(list.iter().map(mail_addr).collect())
}

#[cfg(test)]
mod tests {
    use mailune_protocol::Address;

    use super::{Attachment, Outbound, build};
    use crate::message::{Body, PartRole};
    use crate::parse;

    fn body_text(message: &crate::MimeMessage, role: PartRole) -> &str {
        message
            .parts
            .iter()
            .find(|part| part.role == role)
            .and_then(|part| match &part.body {
                Body::Text(text) => Some(text.as_str()),
                Body::Bytes(_) => None,
            })
            .unwrap_or("")
    }

    #[test]
    fn build_then_parse() {
        let outbound = Outbound {
            from: Address {
                name: Some("Ana".to_string()),
                email: "ana@acme.io".to_string(),
            },
            to: vec![Address {
                name: None,
                email: "bo@acme.io".to_string(),
            }],
            subject: "Re: Hello".to_string(),
            text: "plain reply".to_string(),
            html: "<p>html reply</p>".to_string(),
            attachment: Attachment {
                media_type: "application/pdf".to_string(),
                filename: "note.pdf".to_string(),
                bytes: b"hello".to_vec(),
            },
            message_id: "reply@acme.io".to_string(),
            in_reply_to: "m1@acme.io".to_string(),
            references: vec!["root@acme.io".to_string(), "m1@acme.io".to_string()],
        };

        let bytes = build(&outbound).unwrap();
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.subject, "Re: Hello");
        assert_eq!(parsed.message_id.as_deref(), Some("reply@acme.io"));
        assert_eq!(parsed.from[0].email, "ana@acme.io");
        assert_eq!(parsed.to[0].email, "bo@acme.io");
        assert_eq!(parsed.in_reply_to, ["m1@acme.io"]);
        assert_eq!(parsed.references, ["root@acme.io", "m1@acme.io"]);
        assert_eq!(body_text(&parsed, PartRole::Text), "plain reply");
        assert_eq!(body_text(&parsed, PartRole::Html), "<p>html reply</p>");
        let file = parsed
            .parts
            .iter()
            .find(|part| part.role == PartRole::Attachment)
            .unwrap();
        assert_eq!(file.filename.as_deref(), Some("note.pdf"));
        assert_eq!(file.body, Body::Bytes(b"hello".to_vec()));
    }
}
