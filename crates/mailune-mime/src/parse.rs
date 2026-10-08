//! Bytes to a [`MimeMessage`](crate::MimeMessage).
//!
//! Charset labels are renamed through `encoding_rs` so the stored name is
//! the one the Encoding Standard decodes. `mail-parser` with `full_encoding`
//! uses the same crate for the multibyte tail (Shift_JIS and the rest);
//! without that feature those decoders return lossy UTF-8.

use encoding_rs::Encoding;
use mail_parser::{Addr, HeaderValue, Message, MessageParser, MimeHeaders, PartType};

use crate::Error;
use crate::message::{Body, MimeMessage, Part, PartRole};
use mailune_protocol::Address;

/// Parse one message. The input is not fetched from anywhere; it is the bytes
/// the caller already holds.
pub fn parse(bytes: &[u8]) -> Result<MimeMessage, Error> {
    let message = MessageParser::default().parse(bytes).ok_or(Error::Parse)?;
    if message.parts.is_empty() {
        return Err(Error::Parse);
    }
    Ok(MimeMessage {
        subject: message.subject().unwrap_or("").to_string(),
        from: addresses(message.from()),
        to: addresses(message.to()),
        cc: addresses(message.cc()),
        message_id: message.message_id().map(str::to_string),
        in_reply_to: id_list(message.in_reply_to()),
        references: id_list(message.references()),
        parts: parts(&message),
    })
}

fn addresses(value: Option<&mail_parser::Address<'_>>) -> Vec<Address> {
    let Some(value) = value else {
        return Vec::new();
    };
    match value {
        mail_parser::Address::List(list) => list.iter().filter_map(one_address).collect(),
        mail_parser::Address::Group(groups) => groups
            .iter()
            .flat_map(|group| group.addresses.iter().filter_map(one_address))
            .collect(),
    }
}

fn one_address(addr: &Addr<'_>) -> Option<Address> {
    let email = addr.address.as_deref()?.trim();
    if email.is_empty() {
        return None;
    }
    let name = addr
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty());
    Some(Address {
        name: name.map(str::to_string),
        email: email.to_string(),
    })
}

fn id_list(value: &HeaderValue<'_>) -> Vec<String> {
    match value {
        HeaderValue::Text(text) => split_ids(text),
        HeaderValue::TextList(list) => list.iter().flat_map(|text| split_ids(text)).collect(),
        HeaderValue::Empty => Vec::new(),
        _ => Vec::new(),
    }
}

fn split_ids(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|id| id.trim_matches(|c| c == '<' || c == '>').to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

fn parts(message: &Message<'_>) -> Vec<Part> {
    message
        .parts
        .iter()
        .enumerate()
        .filter_map(|(index, part)| {
            let body = body_of(part)?;
            let id = u32::try_from(index).ok()?;
            Some(Part {
                role: role(id, message, part),
                media_type: media_type(part),
                charset: charset_name(part),
                filename: part.attachment_name().map(str::to_string),
                content_id: content_id(part),
                body,
            })
        })
        .collect()
}

fn role(id: u32, message: &Message<'_>, part: &mail_parser::MessagePart<'_>) -> PartRole {
    let in_text = message.text_body.contains(&id);
    let in_html = message.html_body.contains(&id);
    // A single text body is copied into the other list so a client can still
    // render it. The media type is the one the part actually has.
    if in_text && in_html {
        if media_type(part) == "text/html" {
            PartRole::Html
        } else {
            PartRole::Text
        }
    } else if in_html {
        PartRole::Html
    } else if in_text {
        PartRole::Text
    } else if matches!(part.body, PartType::InlineBinary(_)) {
        PartRole::Inline
    } else if message.attachments.contains(&id) {
        PartRole::Attachment
    } else {
        PartRole::Other
    }
}

fn media_type(part: &mail_parser::MessagePart<'_>) -> String {
    let Some(ct) = part.content_type() else {
        return "application/octet-stream".to_string();
    };
    let ctype = ct.ctype().to_ascii_lowercase();
    match ct.subtype() {
        Some(sub) => format!("{ctype}/{}", sub.to_ascii_lowercase()),
        None => ctype,
    }
}

fn charset_name(part: &mail_parser::MessagePart<'_>) -> Option<String> {
    let label = part.content_type()?.attribute("charset")?;
    Some(match Encoding::for_label(label.as_bytes()) {
        Some(encoding) => encoding.name().to_string(),
        None => label.to_string(),
    })
}

fn content_id(part: &mail_parser::MessagePart<'_>) -> Option<String> {
    let raw = part.content_id()?.trim();
    let bare = raw.trim_matches(|c| c == '<' || c == '>');
    if bare.is_empty() {
        None
    } else {
        Some(bare.to_string())
    }
}

fn body_of(part: &mail_parser::MessagePart<'_>) -> Option<Body> {
    match &part.body {
        PartType::Text(text) | PartType::Html(text) => Some(Body::Text(text.to_string())),
        PartType::Binary(bytes) | PartType::InlineBinary(bytes) => {
            Some(Body::Bytes(bytes.to_vec()))
        }
        PartType::Message(inner) => Some(Body::Bytes(inner.raw_message().to_vec())),
        PartType::Multipart(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::message::{Body, PartRole};

    fn text(message: &crate::MimeMessage, role: PartRole) -> &str {
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
    fn empty_bytes_fail() {
        assert!(parse(b"").is_err());
    }

    #[test]
    fn fixture_round_trips_charset_and_attachment() {
        let raw = "\
From: Ana Acme <ana@acme.io>\r\n\
To: Bo <bo@acme.io>\r\n\
Cc: Cy <cy@acme.io>\r\n\
Subject: Hello\r\n\
Message-ID: <m1@acme.io>\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"bnd\"\r\n\
\r\n\
--bnd\r\n\
Content-Type: text/plain; charset=iso-8859-1\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
caf=E9\r\n\
--bnd\r\n\
Content-Type: application/pdf; name=\"note.pdf\"\r\n\
Content-Disposition: attachment; filename=\"note.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
aGVsbG8=\r\n\
--bnd--\r\n";

        let message = parse(raw.as_bytes()).unwrap();
        assert_eq!(message.subject, "Hello");
        assert_eq!(message.message_id.as_deref(), Some("m1@acme.io"));
        assert_eq!(message.from[0].email, "ana@acme.io");
        assert_eq!(message.from[0].name.as_deref(), Some("Ana Acme"));
        assert_eq!(message.to[0].email, "bo@acme.io");
        assert_eq!(message.cc[0].email, "cy@acme.io");

        let plain = message
            .parts
            .iter()
            .find(|part| part.role == PartRole::Text)
            .unwrap();
        assert_eq!(plain.media_type, "text/plain");
        assert_eq!(plain.charset.as_deref(), Some("windows-1252"));
        assert_eq!(text(&message, PartRole::Text), "café");

        let file = message
            .parts
            .iter()
            .find(|part| part.role == PartRole::Attachment)
            .unwrap();
        assert_eq!(file.filename.as_deref(), Some("note.pdf"));
        assert_eq!(file.media_type, "application/pdf");
        assert_eq!(file.body, Body::Bytes(b"hello".to_vec()));
    }

    #[test]
    fn shift_jis_body_decodes() {
        let raw = "\
From: Ana <ana@acme.io>\r\n\
Subject: sjis\r\n\
MIME-Version: 1.0\r\n\
Content-Type: text/plain; charset=shift_jis\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
=83=6E=83=8D=81=5B=81=45=83=8F=81=5B=83=8B=83=68\r\n";

        let message = parse(raw.as_bytes()).unwrap();
        let plain = message
            .parts
            .iter()
            .find(|part| part.role == PartRole::Text)
            .unwrap();
        assert_eq!(plain.charset.as_deref(), Some("Shift_JIS"));
        assert_eq!(
            text(&message, PartRole::Text).trim_end(),
            "ハロー・ワールド"
        );
    }
}
