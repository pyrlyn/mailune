//! Write a parsed message as `.eml`, and a list as mbox.
//!
//! [`MimeMessage`](crate::MimeMessage) has no `Date`, so both files use the
//! Unix epoch. The mbox separator is that same instant in `ctime` form, and
//! it is not a received date. Body lines that look like a separator are
//! quoted the mboxrd way so a later reader does not split the message.

use mail_builder::MessageBuilder;
use mail_builder::headers::message_id::MessageId;

use crate::Error;
use crate::build::mail_list;
use crate::message::{Body, MimeMessage, Part, PartRole};

const MBOX_STAMP: &[u8] = b" Thu Jan  1 00:00:00 1970\n";
const MISSING_ID: &str = "export@mailune.invalid";

/// RFC 5322 bytes for one parsed message.
pub fn write_eml(message: &MimeMessage) -> Result<Vec<u8>, Error> {
    let mut builder = MessageBuilder::new()
        .date(0_i64)
        .subject(message.subject.as_str());
    builder = builder.message_id(message.message_id.as_deref().unwrap_or(MISSING_ID));
    if !message.from.is_empty() {
        builder = builder.from(mail_list(&message.from));
    }
    if !message.to.is_empty() {
        builder = builder.to(mail_list(&message.to));
    }
    if !message.cc.is_empty() {
        builder = builder.cc(mail_list(&message.cc));
    }
    if !message.in_reply_to.is_empty() {
        builder = builder.in_reply_to(MessageId::new_list(
            message.in_reply_to.iter().map(String::as_str),
        ));
    }
    if !message.references.is_empty() {
        builder = builder.references(MessageId::new_list(
            message.references.iter().map(String::as_str),
        ));
    }

    let text_at = first_body(message, PartRole::Text);
    let html_at = first_body(message, PartRole::Html);
    if let Some(index) = text_at {
        builder = builder.text_body(text_of(&message.parts[index]));
    }
    if let Some(index) = html_at {
        builder = builder.html_body(text_of(&message.parts[index]));
    }
    for (index, part) in message.parts.iter().enumerate() {
        if Some(index) == text_at || Some(index) == html_at {
            continue;
        }
        builder = attach(builder, part, index);
    }
    if text_at.is_none() && html_at.is_none() && message.parts.is_empty() {
        builder = builder.text_body("");
    }
    builder.write_to_vec().map_err(Error::Build)
}

/// One mbox file. An empty list is an empty file.
pub fn write_mbox(messages: &[MimeMessage]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for message in messages {
        let eml = write_eml(message)?;
        out.extend_from_slice(b"From ");
        out.extend_from_slice(mbox_sender(message).as_bytes());
        out.extend_from_slice(MBOX_STAMP);
        quote_from_lines(&eml, &mut out);
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        out.push(b'\n');
    }
    Ok(out)
}

fn first_body(message: &MimeMessage, role: PartRole) -> Option<usize> {
    message
        .parts
        .iter()
        .position(|part| part.role == role && matches!(part.body, Body::Text(_)))
}

fn text_of(part: &Part) -> &str {
    match &part.body {
        Body::Text(value) => value.as_str(),
        Body::Bytes(_) => "",
    }
}

fn attach<'a>(builder: MessageBuilder<'a>, part: &'a Part, index: usize) -> MessageBuilder<'a> {
    let media = part.media_type.as_str();
    match &part.body {
        Body::Text(value) => {
            if let Some(cid) = &part.content_id {
                builder.inline(media, cid.as_str(), value.as_str())
            } else {
                builder.attachment(media, filename(part, index), value.as_str())
            }
        }
        Body::Bytes(bytes) => {
            if let Some(cid) = &part.content_id {
                builder.inline(media, cid.as_str(), bytes.as_slice())
            } else {
                builder.attachment(media, filename(part, index), bytes.as_slice())
            }
        }
    }
}

fn filename(part: &Part, index: usize) -> String {
    part.filename
        .clone()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("part-{index}"))
}

/// A separator token. Addresses with spaces or line breaks would split the file.
fn mbox_sender(message: &MimeMessage) -> String {
    let Some(email) = message.from.first().map(|addr| addr.email.as_str()) else {
        return "MAILER-DAEMON".to_owned();
    };
    if email.is_empty()
        || email
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte == b'<' || byte == b'>')
    {
        "MAILER-DAEMON".to_owned()
    } else {
        email.to_owned()
    }
}

fn quote_from_lines(eml: &[u8], out: &mut Vec<u8>) {
    for line in eml.split_inclusive(|&byte| byte == b'\n') {
        if looks_like_separator(line) {
            out.push(b'>');
        }
        out.extend_from_slice(line);
    }
}

fn looks_like_separator(line: &[u8]) -> bool {
    let mut body = line.strip_suffix(b"\n").unwrap_or(line);
    body = body.strip_suffix(b"\r").unwrap_or(body);
    while let Some(stripped) = body.strip_prefix(b">") {
        body = stripped;
    }
    body.starts_with(b"From ")
}

#[cfg(test)]
mod tests {
    use mailune_protocol::Address;

    use super::*;
    use crate::message::{Body, Part, PartRole};
    use crate::parse;

    fn sample() -> MimeMessage {
        MimeMessage {
            subject: "Hi".to_owned(),
            from: vec![Address {
                name: Some("Ana".to_owned()),
                email: "ana@example.com".to_owned(),
            }],
            to: vec![Address {
                name: None,
                email: "bo@example.com".to_owned(),
            }],
            cc: Vec::new(),
            message_id: Some("m1@example.com".to_owned()),
            in_reply_to: Vec::new(),
            references: Vec::new(),
            parts: vec![Part {
                role: PartRole::Text,
                media_type: "text/plain".to_owned(),
                charset: Some("utf-8".to_owned()),
                filename: None,
                content_id: None,
                body: Body::Text("Hello\r\nFrom the desk\r\n".to_owned()),
            }],
        }
    }

    #[test]
    fn one_message_is_eml_and_a_list_is_mbox() {
        let message = sample();
        let eml = write_eml(&message).unwrap();
        let parsed = parse(&eml).unwrap();
        assert_eq!(parsed.subject, "Hi");
        assert_eq!(parsed.from[0].email, "ana@example.com");
        let text = match &parsed.parts[0].body {
            Body::Text(value) => value.as_str(),
            Body::Bytes(_) => panic!("text part"),
        };
        assert!(text.contains("Hello"));
        assert!(text.contains("From the desk"));

        let mbox = write_mbox(&[message, sample()]).unwrap();
        let text = String::from_utf8(mbox).unwrap();
        assert_eq!(text.matches("From ana@example.com ").count(), 2);
        assert!(text.contains(">From the desk"));
        assert!(write_mbox(&[]).unwrap().is_empty());
    }
}
