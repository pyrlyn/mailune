//! WASM subset of the mail core.
//!
//! Protocol types, threading, and the query parser come from the native
//! crates. `mailune-mime` does not build for `wasm32-unknown-unknown`
//! (`getrandom` refuses that target without its `js` feature), so a plain
//! message is parsed here. A native test checks that parse against
//! `mailune-mime`. A scripted JMAP `Mailbox/query` is forwarded to
//! `mailune-jmap` with the store feature off, so this crate still builds for
//! wasm32. Nothing here opens a socket.

pub use mailune_core::{Container, Query, Threadable, parse_query, thread_messages};
pub use mailune_protocol::{Address, Envelope};

use mailune_protocol::{Flags, MessageId, ThreadId, TransportSecurity};

/// A plain message this subset can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMail {
    /// `Subject`.
    pub subject: String,
    /// `From`.
    pub from: Vec<Address>,
    /// `Message-ID` without angle brackets.
    pub message_id: Option<String>,
    /// `In-Reply-To`, without angle brackets.
    pub in_reply_to: Vec<String>,
    /// The text body after the header block.
    pub text: String,
}

/// Failure while reading a plain message or a JMAP query.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a UTF-8 message with a header block.
    #[error("the bytes are not a plain message")]
    Parse,
    /// The scripted JMAP body was not a query object.
    #[error(transparent)]
    Jmap(#[from] mailune_jmap::Error),
}

/// Mailbox ids from a scripted `Mailbox/query`, in document order.
///
/// # Errors
///
/// [`Error::Jmap`] when `json` is not a query object.
pub fn query_mailbox_ids(json: &str) -> Result<Vec<String>, Error> {
    mailune_jmap::query_mailbox_ids(json).map_err(Error::from)
}

/// Parses one `text/plain` message. Folded headers and multipart are out of this subset.
///
/// # Errors
///
/// [`Error::Parse`] when the bytes are not UTF-8 or have no header block.
pub fn parse_message(bytes: &[u8]) -> Result<ParsedMail, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Parse)?;
    let (head, body) = split_head(text).ok_or(Error::Parse)?;
    let head = head.replace("\r\n", "\n");
    let mut subject = String::new();
    let mut from = Vec::new();
    let mut message_id = None;
    let mut in_reply_to = Vec::new();
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("subject") && subject.is_empty() {
            subject = value.to_string();
        } else if name.eq_ignore_ascii_case("from") && from.is_empty() {
            from = parse_from(value);
        } else if name.eq_ignore_ascii_case("message-id") && message_id.is_none() {
            message_id = bare_id(value);
        } else if name.eq_ignore_ascii_case("in-reply-to") && in_reply_to.is_empty() {
            in_reply_to = value.split_whitespace().filter_map(bare_id).collect();
        }
    }
    Ok(ParsedMail {
        subject,
        from,
        message_id,
        in_reply_to,
        text: body.trim_end_matches(['\r', '\n']).to_string(),
    })
}

/// A protocol envelope for a parsed plain message.
pub fn envelope(mail: &ParsedMail) -> Envelope {
    let id = mail.message_id.clone().unwrap_or_default();
    let from = mail.from.first().cloned().unwrap_or(Address {
        name: None,
        email: String::new(),
    });
    Envelope {
        id: MessageId::new(id.clone()),
        thread: ThreadId::new(id),
        from,
        to: Vec::new(),
        cc: Vec::new(),
        subject: mail.subject.clone(),
        stamp: String::new(),
        snippet: mail.text.clone(),
        flags: Flags {
            seen: false,
            flagged: false,
            draft: false,
            answered: false,
            deleted: false,
            keywords: Vec::new(),
        },
        attachment_count: 0,
        transport: TransportSecurity::Tls,
    }
}

fn split_head(text: &str) -> Option<(&str, &str)> {
    if let Some(index) = text.find("\r\n\r\n") {
        return Some((&text[..index], &text[index + 4..]));
    }
    let (head, body) = text.split_once("\n\n")?;
    Some((head, body))
}

fn parse_from(value: &str) -> Vec<Address> {
    if let Some((name, rest)) = value.rsplit_once('<') {
        let email = rest.trim().trim_end_matches('>').trim();
        if email.is_empty() {
            return Vec::new();
        }
        let name = name.trim().trim_matches('"');
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        return vec![Address {
            name,
            email: email.to_string(),
        }];
    }
    let email = value.trim();
    if email.is_empty() {
        return Vec::new();
    }
    vec![Address {
        name: None,
        email: email.to_string(),
    }]
}

fn bare_id(value: &str) -> Option<String> {
    let id = value.trim().trim_matches(|ch| ch == '<' || ch == '>');
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{envelope, parse_message, parse_query, thread_messages};
    use mailune_core::Threadable;
    use mailune_mime::parse;
    use mailune_mime::{Body, PartRole};

    const RAW: &[u8] = b"Subject: Hello\r\nFrom: Ada Lovelace <ada@example.com>\r\nMessage-ID: <m1@example.com>\r\nIn-Reply-To: <m0@example.com>\r\n\r\nSee you at the dock\r\n";

    #[test]
    fn plain_mime_matches_the_native_parser() {
        let parsed = parse_message(RAW).unwrap();
        let native = parse(RAW).unwrap();
        assert_eq!(parsed.subject, native.subject);
        assert_eq!(parsed.from, native.from);
        assert_eq!(parsed.message_id, native.message_id);
        assert_eq!(parsed.in_reply_to, native.in_reply_to);
        let text = native
            .parts
            .iter()
            .find(|part| part.role == PartRole::Text)
            .and_then(|part| match &part.body {
                Body::Text(text) => Some(text.as_str()),
                Body::Bytes(_) => None,
            })
            .unwrap_or("");
        assert_eq!(parsed.text, text.trim_end_matches(['\r', '\n']));
        assert_eq!(envelope(&parsed).subject, native.subject);
    }

    #[test]
    fn threading_and_the_query_parser_match_the_native_core() {
        let parsed = parse_message(RAW).unwrap();
        let messages = [Threadable {
            message_id: parsed.message_id.clone().unwrap_or_default(),
            in_reply_to: parsed.in_reply_to.first().cloned(),
            references: Vec::new(),
            subject: parsed.subject.clone(),
            provider_thread: None,
        }];
        assert_eq!(
            thread_messages(&messages),
            mailune_core::thread_messages(&messages)
        );
        let query = "from:ada hello";
        assert_eq!(
            parse_query(query).unwrap(),
            mailune_core::parse_query(query).unwrap()
        );
    }

    #[test]
    fn a_scripted_mailbox_query_matches_the_native_parser() {
        let json = r#"{"accountId":"acc-1","queryState":"q1","ids":["inbox","archive"]}"#;
        assert_eq!(
            super::query_mailbox_ids(json).unwrap(),
            mailune_jmap::query_mailbox_ids(json).unwrap()
        );
    }
}
