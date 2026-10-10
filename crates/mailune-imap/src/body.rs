//! Lazy body fetch (P10): one section of one message, whole or a byte range, when the reader
//! opens it. Initial sync reads only metadata.
//!
//! Both requests use `.PEEK`, so opening a body never sets `\Seen` behind the user's back.
//! Requests are encoded and replies decoded with imap-codec, including `BINARY` (RFC 3516)
//! and its `~{n}` literals, which may hold NUL.

use std::io::{Read, Write};
use std::num::NonZeroU32;

use imap_codec::ResponseCodec;
use imap_codec::decode::Decoder;
use imap_codec::imap_types::command::CommandBody;
use imap_codec::imap_types::core::{NString, NString8, Vec1};
use imap_codec::imap_types::fetch::{
    MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName, Part, Section as ImapSection,
};
use imap_codec::imap_types::response::{Data, Response};
use imap_codec::imap_types::sequence::SequenceSet;

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
    /// [`Error::Argument`] for a zero UID, part number or count, [`Error::Rejected`] when the
    /// server refuses, [`Error::Session`] when the stream stops.
    pub fn fetch_body(
        &mut self,
        uid: u32,
        section: &Section,
        partial: Option<Partial>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let item = MessageDataItemName::BodyExt {
            section: imap_section(section)?,
            partial: range(partial)?,
            peek: true,
        };
        let replies = self.run_raw(uid_fetch(uid, item)?)?;
        let replies = replies.into_iter().map(|reply| reply.bytes);
        Ok(section_data(replies, uid).map(|bytes| clamp(bytes, partial)))
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
        let item = MessageDataItemName::Binary {
            section: part_numbers(part)?,
            partial: range(partial)?,
            peek: true,
        };
        let replies = self.run_raw(uid_fetch(uid, item)?)?;
        let replies = replies.iter().map(without_binary_origin);
        Ok(section_data(replies, uid).map(|bytes| clamp(bytes, partial)))
    }
}

fn uid_fetch(uid: u32, item: MessageDataItemName<'_>) -> Result<CommandBody<'_>, Error> {
    Ok(CommandBody::Fetch {
        sequence_set: SequenceSet::try_from(uid).map_err(|_| Error::Argument)?,
        macro_or_item_names: MacroOrMessageDataItemNames::MessageDataItemNames(vec![item]),
        uid: true,
    })
}

fn imap_section(section: &Section) -> Result<Option<ImapSection<'static>>, Error> {
    Ok(match section {
        Section::Full => None,
        Section::Header => Some(ImapSection::Header(None)),
        Section::Text => Some(ImapSection::Text(None)),
        Section::Part(path) => {
            let path = Vec1::try_from(part_numbers(path)?).map_err(|_| Error::Argument)?;
            Some(ImapSection::Part(Part(path)))
        }
    })
}

fn part_numbers(path: &[u32]) -> Result<Vec<NonZeroU32>, Error> {
    path.iter()
        .map(|&number| NonZeroU32::new(number).ok_or(Error::Argument))
        .collect()
}

fn range(partial: Option<Partial>) -> Result<Option<(u32, NonZeroU32)>, Error> {
    partial
        .map(|Partial { offset, count }| {
            NonZeroU32::new(count)
                .map(|count| (offset, count))
                .ok_or(Error::Argument)
        })
        .transpose()
}

/// The section bytes in the FETCH reply for `uid`. A NIL section and a missing reply are both
/// `None`; replies imap-codec cannot decode are skipped as untrusted noise.
fn section_data(replies: impl Iterator<Item = Vec<u8>>, uid: u32) -> Option<Vec<u8>> {
    let codec = ResponseCodec::new();
    for reply in replies {
        let Ok((_, Response::Data(Data::Fetch { items, .. }))) = codec.decode(&reply) else {
            continue;
        };
        let mut ours = false;
        let mut data = None;
        for item in items.as_ref() {
            match item {
                MessageDataItem::Uid(found) => ours = found.get() == uid,
                MessageDataItem::BodyExt { data: value, .. }
                | MessageDataItem::Binary {
                    value: NString8::NString(value),
                    ..
                } => data = nstring_bytes(value),
                MessageDataItem::Binary {
                    value: NString8::Literal8(literal),
                    ..
                } => data = Some(literal.data.to_vec()),
                _ => {}
            }
        }
        if ours {
            return data;
        }
    }
    None
}

fn nstring_bytes(value: &NString<'_>) -> Option<Vec<u8>> {
    value.0.as_ref().map(|value| value.as_ref().to_vec())
}

/// RFC 3516 and RFC 9051 say a partial `BINARY` reply names its origin (`BINARY[2]<4> ~{n}`),
/// and servers send it, but the ABNF of both leaves it out and imap-codec follows the ABNF.
/// Dropping the origin lets imap-codec decode the reply; it is the offset we asked for. Only
/// bytes outside literals are looked at, so message content never reads as protocol.
fn without_binary_origin(reply: &RawResponse) -> Vec<u8> {
    const NAME: &[u8] = b"BINARY[";
    let bytes = &reply.bytes;
    let inside = |at: usize| reply.literals.iter().any(|range| range.contains(&at));
    let name = (0..bytes.len()).find(|&at| {
        !inside(at)
            && bytes
                .get(at..at + NAME.len())
                .is_some_and(|window| window.eq_ignore_ascii_case(NAME))
    });
    let origin = name.and_then(|name| {
        let after_name = name + NAME.len();
        let section = bytes.get(after_name..)?;
        let close = after_name
            + section
                .iter()
                .position(|&byte| !(byte.is_ascii_digit() || byte == b'.'))?;
        let open = close + 1;
        let digits = bytes.get(open + 1..)?;
        let count = digits
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        let shut = open + 1 + count;
        let well_formed = bytes.get(close) == Some(&b']')
            && bytes.get(open) == Some(&b'<')
            && count > 0
            && bytes.get(shut) == Some(&b'>');
        well_formed.then_some(open..=shut)
    });
    let mut bytes = bytes.clone();
    if let Some(origin) = origin {
        bytes.drain(origin);
    }
    bytes
}

/// A server that sends more than was asked for is cut to the range.
fn clamp(mut bytes: Vec<u8>, partial: Option<Partial>) -> Vec<u8> {
    if let Some(partial) = partial {
        bytes.truncate(usize::try_from(partial.count).unwrap_or(usize::MAX));
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::{Partial, Section};
    use crate::session::RawResponse;
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
        assert!(matches!(
            session.fetch_binary(0, &[1], None),
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
        let literal = b"BINARY[2]<9> UID 7";
        let mut bytes = b"* 1 FETCH (UID 3 BINARY[1]<0> ~{18}\r\n".to_vec();
        let start = bytes.len();
        bytes.extend_from_slice(literal);
        let literals = std::iter::once(start..bytes.len()).collect();
        bytes.extend_from_slice(b")\r\n");
        let reply = RawResponse { bytes, literals };

        let decodable = super::without_binary_origin(&reply);
        assert!(decodable.starts_with(b"* 1 FETCH (UID 3 BINARY[1] ~{18}\r\n"));
        assert_eq!(
            super::section_data(std::iter::once(decodable.clone()), 3).as_deref(),
            Some(&literal[..])
        );
        assert_eq!(super::section_data(std::iter::once(decodable), 7), None);
    }

    #[test]
    fn binary_replies_without_an_origin_are_left_alone() {
        let reply = RawResponse {
            bytes: b"* 1 FETCH (UID 3 BINARY[1.2] \"a<1>\")\r\n".to_vec(),
            literals: Vec::new(),
        };
        assert_eq!(super::without_binary_origin(&reply), reply.bytes);
        assert_eq!(
            super::section_data(std::iter::once(reply.bytes), 3).as_deref(),
            Some(&b"a<1>"[..])
        );
    }
}
