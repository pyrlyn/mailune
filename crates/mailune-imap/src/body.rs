//! Lazy body fetch (P10): one section of one message, whole or a byte range, when the reader
//! opens it. Initial sync reads only metadata.
//!
//! Both requests use `.PEEK`, so opening a body never sets `\Seen` behind the user's back.
//! `BODY` replies are decoded with imap-codec. imap-codec 1.0 knows neither the `BINARY` item
//! nor `~{n}` literals (RFC 3516), and refuses literals holding NUL, which decoded binary parts
//! do; so a `BINARY` reply is read here, from the raw bytes the session kept.

use std::io::{Read, Write};

use imap_codec::ResponseCodec;
use imap_codec::decode::Decoder;
use imap_codec::imap_types::core::NString;
use imap_codec::imap_types::fetch::MessageDataItem;
use imap_codec::imap_types::response::{Data, Response};

use crate::session::RawResponse;
use crate::{Connection, Error};

/// Which part of a message to fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    /// The whole message, header included.
    Full,
    /// The message header.
    Header,
    /// The message body without the header.
    Text,
    /// One MIME part by its BODYSTRUCTURE path, such as `[2, 1]` for `2.1`.
    Part(Vec<u32>),
}

/// A byte range of a section: up to `count` octets starting at `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Partial {
    /// First octet.
    pub offset: u32,
    /// Octets wanted. Zero is refused.
    pub count: u32,
}

impl<S: Read + Write> Connection<S> {
    /// The bytes of `section` of message `uid` in the selected mailbox, still in its transfer
    /// encoding. `None` when the server returned nothing for it (the message is gone, or the
    /// part does not exist).
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] for a zero part number or count, [`Error::Rejected`] when the server
    /// refuses, [`Error::Session`] when the stream stops.
    pub fn fetch_body(
        &mut self,
        uid: u32,
        section: &Section,
        partial: Option<Partial>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let spec = section_spec(section)?;
        let range = range_spec(partial)?;
        let replies = self.run_raw(&format!("UID FETCH {uid} (BODY.PEEK[{spec}]{range})"))?;
        let codec = ResponseCodec::new();
        for reply in &replies {
            let Ok((_, Response::Data(Data::Fetch { items, .. }))) = codec.decode(&reply.bytes)
            else {
                continue;
            };
            let mut ours = false;
            let mut data = None;
            for item in items.as_ref() {
                match item {
                    MessageDataItem::Uid(found) => ours = found.get() == uid,
                    MessageDataItem::BodyExt {
                        data: NString(value),
                        ..
                    } => data = value.as_ref().map(|value| value.as_ref().to_vec()),
                    _ => {}
                }
            }
            if ours {
                return Ok(data.map(|bytes| clamp(bytes, partial)));
            }
        }
        Ok(None)
    }

    /// The decoded bytes of part `part` of message `uid` (RFC 3516): the server undoes base64
    /// or quoted-printable, so binary attachments come back as they were sent. An empty `part`
    /// is the whole message. `None` as for [`Connection::fetch_body`].
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] when the last CAPABILITY reply had no `BINARY`, otherwise as for
    /// [`Connection::fetch_body`].
    pub fn fetch_binary(
        &mut self,
        uid: u32,
        part: &[u32],
        partial: Option<Partial>,
    ) -> Result<Option<Vec<u8>>, Error> {
        if !self.has_capability("BINARY") {
            return Err(Error::Unsupported);
        }
        let spec = if part.is_empty() {
            String::new()
        } else {
            part_path(part)?
        };
        let range = range_spec(partial)?;
        let replies = self.run_raw(&format!("UID FETCH {uid} (BINARY.PEEK[{spec}]{range})"))?;
        for reply in &replies {
            if !is_fetch(reply) || !has_uid(reply, uid) {
                continue;
            }
            return binary_data(reply).map(|data| data.map(|bytes| clamp(bytes, partial)));
        }
        Ok(None)
    }
}

fn section_spec(section: &Section) -> Result<String, Error> {
    Ok(match section {
        Section::Full => String::new(),
        Section::Header => "HEADER".to_string(),
        Section::Text => "TEXT".to_string(),
        Section::Part(path) => part_path(path)?,
    })
}

fn part_path(path: &[u32]) -> Result<String, Error> {
    if path.is_empty() || path.contains(&0) {
        return Err(Error::Argument);
    }
    Ok(path
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("."))
}

fn range_spec(partial: Option<Partial>) -> Result<String, Error> {
    match partial {
        None => Ok(String::new()),
        Some(Partial { count: 0, .. }) => Err(Error::Argument),
        Some(Partial { offset, count }) => Ok(format!("<{offset}.{count}>")),
    }
}

/// A server that sends more than was asked for is cut to the range.
fn clamp(mut bytes: Vec<u8>, partial: Option<Partial>) -> Vec<u8> {
    if let Some(partial) = partial {
        bytes.truncate(usize::try_from(partial.count).unwrap_or(usize::MAX));
    }
    bytes
}

/// The first position of `needle` that is not inside a literal, so message bytes that happen
/// to read like protocol are never mistaken for it.
fn find_outside(reply: &RawResponse, needle: &[u8], from: usize) -> Option<usize> {
    let inside = |at: usize| reply.literals.iter().any(|range| range.contains(&at));
    (from..reply
        .bytes
        .len()
        .saturating_sub(needle.len())
        .saturating_add(1))
        .find(|&at| !inside(at) && reply.bytes.get(at..at + needle.len()) == Some(needle))
}

fn is_fetch(reply: &RawResponse) -> bool {
    reply.bytes.starts_with(b"* ") && find_outside(reply, b" FETCH (", 0).is_some()
}

fn has_uid(reply: &RawResponse, uid: u32) -> bool {
    let mut from = 0;
    while let Some(at) = find_outside(reply, b"UID ", from) {
        let boundary = matches!(reply.bytes.get(at.wrapping_sub(1)), Some(b'(' | b' '));
        let digits: Vec<u8> = reply.bytes[at + 4..]
            .iter()
            .copied()
            .take_while(u8::is_ascii_digit)
            .collect();
        if boundary
            && std::str::from_utf8(&digits)
                .ok()
                .and_then(|text| text.parse().ok())
                == Some(uid)
        {
            return true;
        }
        from = at + 1;
    }
    false
}

/// The value after `BINARY[...]<n> `: a literal, a literal8, a quoted string, or NIL.
fn binary_data(reply: &RawResponse) -> Result<Option<Vec<u8>>, Error> {
    let at = find_outside(reply, b"BINARY[", 0).ok_or(Error::Response)?;
    let close = find_outside(reply, b"]", at).ok_or(Error::Response)?;
    let mut cursor = close + 1;
    if reply.bytes.get(cursor) == Some(&b'<') {
        cursor = find_outside(reply, b">", cursor).ok_or(Error::Response)? + 1;
    }
    if reply.bytes.get(cursor) != Some(&b' ') {
        return Err(Error::Response);
    }
    cursor += 1;
    let rest = reply.bytes.get(cursor..).ok_or(Error::Response)?;
    if rest.starts_with(b"NIL") {
        return Ok(None);
    }
    if rest.starts_with(b"~{") || rest.starts_with(b"{") {
        let literal = reply
            .literals
            .iter()
            .find(|range| range.start > cursor)
            .ok_or(Error::Response)?;
        return Ok(Some(reply.bytes[literal.clone()].to_vec()));
    }
    if rest.first() == Some(&b'"') {
        let mut out = Vec::new();
        let mut bytes = rest.iter().skip(1);
        while let Some(&byte) = bytes.next() {
            match byte {
                b'"' => return Ok(Some(out)),
                b'\\' => out.push(*bytes.next().ok_or(Error::Response)?),
                _ => out.push(byte),
            }
        }
    }
    Err(Error::Response)
}

#[cfg(test)]
mod tests {
    use super::{Partial, Section};
    use crate::sync::tests::{config, day};
    use crate::{Connection, Error, MemStream, Scripted};

    /// Logged in with Archive selected: UID 1 is plain text, UID 2 has a PDF attachment.
    fn session(server: Scripted) -> Connection<MemStream> {
        let mut server = server;
        server.create("Archive", 9);
        server.deliver("Archive", "ana@acme.io", "Plain", day(10, 1), None);
        server.deliver(
            "Archive",
            "ana@acme.io",
            "Deck",
            day(10, 2),
            Some("deck.pdf"),
        );
        let mut session = Connection::over(MemStream::new(server), config()).unwrap();
        session.capability().unwrap();
        session.login().unwrap();
        session.select("Archive").unwrap();
        session
    }

    #[test]
    fn body_peek_returns_whole_sections_and_byte_ranges() {
        let mut session = session(Scripted::new());
        let full = session
            .fetch_body(1, &Section::Full, None)
            .unwrap()
            .unwrap();
        let text = String::from_utf8(full.clone()).unwrap();
        assert!(text.starts_with("From: ana@acme.io\r\nSubject: Plain\r\n"));
        assert!(text.ends_with("\r\n\r\nHello\r\n"));

        let header = session
            .fetch_body(1, &Section::Header, None)
            .unwrap()
            .unwrap();
        assert!(header.ends_with(b"\r\n\r\n") && full.starts_with(&header));
        let body = session
            .fetch_body(1, &Section::Text, None)
            .unwrap()
            .unwrap();
        assert_eq!(body, b"Hello\r\n");

        let range = Partial {
            offset: 6,
            count: 8,
        };
        let slice = session.fetch_body(1, &Section::Full, Some(range)).unwrap();
        assert_eq!(slice.as_deref(), Some(&full[6..14]));

        let encoded = session
            .fetch_body(2, &Section::Part(vec![2]), None)
            .unwrap();
        assert_eq!(encoded.as_deref(), Some(&b"JVBERi0AAQI=\r\n"[..]));
        let first = session
            .fetch_body(
                2,
                &Section::Part(vec![1]),
                Some(Partial {
                    offset: 1,
                    count: 3,
                }),
            )
            .unwrap();
        assert_eq!(first.as_deref(), Some(&b"ell"[..]));
    }

    #[test]
    fn binary_peek_returns_decoded_bytes_with_nul() {
        let mut session = session(Scripted::new());
        let pdf = session.fetch_binary(2, &[2], None).unwrap();
        assert_eq!(pdf.as_deref(), Some(&b"%PDF-\0\x01\x02"[..]));
        let tail = session
            .fetch_binary(
                2,
                &[2],
                Some(Partial {
                    offset: 4,
                    count: 100,
                }),
            )
            .unwrap();
        assert_eq!(tail.as_deref(), Some(&b"-\0\x01\x02"[..]));
        let text = session.fetch_binary(2, &[1], None).unwrap();
        assert_eq!(text.as_deref(), Some(&b"Hello\r\n"[..]));
    }

    #[test]
    fn missing_messages_and_parts_come_back_as_none() {
        let mut session = session(Scripted::new());
        assert_eq!(session.fetch_body(99, &Section::Full, None).unwrap(), None);
        assert_eq!(
            session
                .fetch_body(1, &Section::Part(vec![3]), None)
                .unwrap(),
            None
        );
        assert_eq!(session.fetch_binary(1, &[3], None).unwrap(), None);
    }

    #[test]
    fn bad_requests_are_refused_before_sending() {
        let mut session = session(Scripted::new());
        let zero = Some(Partial {
            offset: 0,
            count: 0,
        });
        assert!(matches!(
            session.fetch_body(1, &Section::Full, zero),
            Err(Error::Argument)
        ));
        assert!(matches!(
            session.fetch_body(1, &Section::Part(vec![1, 0]), None),
            Err(Error::Argument)
        ));
        assert!(matches!(
            session.fetch_body(1, &Section::Part(Vec::new()), None),
            Err(Error::Argument)
        ));
        // The session still works after a refused request.
        assert!(
            session
                .fetch_body(1, &Section::Text, None)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn binary_needs_the_capability() {
        let mut session = session(Scripted::new().with_capabilities(&["UIDPLUS"]));
        assert!(matches!(
            session.fetch_binary(2, &[2], None),
            Err(Error::Unsupported)
        ));
    }

    #[test]
    fn protocol_text_inside_a_literal_is_not_parsed() {
        let reply = crate::session::RawResponse {
            bytes: b"* 1 FETCH (BINARY[1] {15}\r\nUID 7 BINARY[2] UID 3)\r\n".to_vec(),
            literals: std::iter::once(27..42).collect(),
        };
        assert!(super::has_uid(&reply, 3));
        assert!(!super::has_uid(&reply, 7));
        assert_eq!(
            super::binary_data(&reply).unwrap().as_deref(),
            Some(&b"UID 7 BINARY[2]"[..])
        );
    }
}
