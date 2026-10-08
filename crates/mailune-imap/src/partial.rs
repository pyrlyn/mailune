//! Lazy body bytes from the scripted server.
//!
//! `BODY.PEEK` does not set `\Seen`. `imap-codec` 1.0.0 folds a `BODY[]`
//! literal into [`MessageDataItem::BodyExt`] and has no `BINARY` fetch
//! item, so a binary span is read from the `{n}` marker the server wrote.
//! Nothing here connects.

use std::io::{Read, Write};

use imap_codec::imap_types::core::NString;
use imap_codec::imap_types::fetch::MessageDataItem;
use imap_codec::imap_types::response::{Data, Response};

use crate::Error;
use crate::session::Connection;
use crate::sync::each_response;

/// `BODY.PEEK[section]<offset.length>` from a selected mailbox.
pub fn peek_bytes<S: Read + Write>(
    session: &mut Connection<S>,
    uid: u32,
    section: &str,
    offset: usize,
    length: usize,
) -> Result<Vec<u8>, Error> {
    fetch_span(session, uid, "BODY.PEEK", section, offset, length)
}

/// `BINARY[section]<offset.length>` from a selected mailbox.
///
/// The scripted server stores 7bit text, so the octets are the same span
/// `BODY.PEEK` would return. A transfer encoding is not decoded here.
pub fn binary_bytes<S: Read + Write>(
    session: &mut Connection<S>,
    uid: u32,
    section: &str,
    offset: usize,
    length: usize,
) -> Result<Vec<u8>, Error> {
    fetch_span(session, uid, "BINARY", section, offset, length)
}

fn fetch_span<S: Read + Write>(
    session: &mut Connection<S>,
    uid: u32,
    item: &str,
    section: &str,
    offset: usize,
    length: usize,
) -> Result<Vec<u8>, Error> {
    let reply = session.transact(&format!(
        "UID FETCH {uid} ({item}[{section}]<{offset}.{length}>)"
    ))?;
    span_bytes(reply.as_bytes())
}

fn span_bytes(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    if let Some(found) = body_ext(bytes) {
        return Ok(found);
    }
    marker_bytes(bytes)
}

fn body_ext(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut found = None;
    each_response(bytes, |response| {
        if let Response::Data(Data::Fetch { items, .. }) = response {
            for item in items.as_ref() {
                if let MessageDataItem::BodyExt { data, .. } = item {
                    found = Some(nstring_owned(data));
                }
            }
        }
    })
    .ok()?;
    found
}

fn nstring_owned(value: &NString<'_>) -> Vec<u8> {
    value
        .0
        .as_ref()
        .map(|inner| inner.as_ref().to_vec())
        .unwrap_or_default()
}

fn marker_bytes(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Response)?;
    let start = text.find('{').ok_or(Error::Response)?;
    let end = text[start + 1..].find('}').ok_or(Error::Response)? + start + 1;
    let length: usize = text[start + 1..end].parse().map_err(|_| Error::Response)?;
    let data_at = end + 3;
    let raw = text.as_bytes();
    if data_at + length > raw.len() {
        return Err(Error::Response);
    }
    Ok(raw[data_at..data_at + length].to_vec())
}

#[cfg(test)]
mod tests {
    use super::{binary_bytes, peek_bytes};
    use crate::session::{Config, Connection};

    #[test]
    fn peek_and_binary_return_the_requested_span() {
        let mut session = Connection::open(config()).unwrap();
        session.login().unwrap();
        session.select("INBOX").unwrap();
        let head = peek_bytes(&mut session, 1, "", 0, 4).unwrap();
        assert_eq!(head, b"From");
        let word = peek_bytes(&mut session, 1, "TEXT", 0, 5).unwrap();
        assert_eq!(word, b"Hello");
        let same = binary_bytes(&mut session, 1, "TEXT", 0, 5).unwrap();
        assert_eq!(same, b"Hello");
        let tail = binary_bytes(&mut session, 1, "1", 1, 4).unwrap();
        assert_eq!(tail, b"ello");
    }

    fn config() -> Config {
        Config {
            tls_required: true,
            username: "ana".to_string(),
            password: "secret".to_string(),
        }
    }
}
