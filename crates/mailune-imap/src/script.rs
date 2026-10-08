//! In-memory IMAP server for tests. No TCP.
//!
//! The Gmail capability `X-GM-EXT-1` is read from the provider row in
//! `mailune-mime`. That table has no Dovecot row, so the Dovecot fact is
//! copied here: `body-fld-octets` may be `-1`, which `imap-codec` rectifies
//! to 0 when `quirk_rectify_numbers` is on.

use mailune_mime::{Provider, quirks};

/// The one message FETCH returns. CRLF, as on the wire.
pub const FIXTURE: &str = "From: ana@example.com\r\nSubject: Hi\r\n\r\nHello\r\n";

/// Scripted server. Feed client bytes with [`Scripted::ingest`].
#[derive(Debug)]
pub struct Scripted {
    pending: Vec<u8>,
    authed: bool,
    selected: bool,
}

impl Scripted {
    /// A server that has not greeted yet.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            authed: false,
            selected: false,
        }
    }

    /// The greeting the client reads before it sends a command.
    pub fn greeting(&self) -> &'static str {
        "* OK IMAP4rev1 ready\r\n"
    }

    /// Append client bytes and return every response those bytes completed.
    ///
    /// A line without CRLF stays buffered. The caller owns the buffers, so
    /// this never opens a socket.
    pub fn ingest(&mut self, input: &[u8]) -> Vec<u8> {
        self.pending.extend_from_slice(input);
        let mut out = Vec::new();
        while let Some(split) = self.pending.windows(2).position(|window| window == b"\r\n") {
            let line: Vec<u8> = self.pending.drain(..=split + 1).collect();
            let text = String::from_utf8_lossy(&line[..line.len().saturating_sub(2)]);
            out.extend_from_slice(self.respond(text.trim()).as_bytes());
        }
        out
    }

    fn respond(&mut self, line: &str) -> String {
        let mut parts = line.split_whitespace();
        let tag = parts.next().unwrap_or("*");
        let verb = parts.next().unwrap_or("").to_ascii_uppercase();
        match verb.as_str() {
            "CAPABILITY" => {
                let gmail = gmail_capability();
                format!("* CAPABILITY IMAP4rev1 {gmail}\r\n{tag} OK CAPABILITY completed\r\n")
            }
            "LOGIN" => {
                self.authed = true;
                format!("{tag} OK LOGIN completed\r\n")
            }
            "SELECT" if self.authed => {
                self.selected = true;
                format!("* 1 EXISTS\r\n{tag} OK [READ-WRITE] SELECT completed\r\n")
            }
            "FETCH" if self.selected => fetch_response(tag),
            "SELECT" | "FETCH" => format!("{tag} NO not authenticated\r\n"),
            _ => format!("{tag} BAD unknown command\r\n"),
        }
    }
}

impl Default for Scripted {
    fn default() -> Self {
        Self::new()
    }
}

fn gmail_capability() -> &'static str {
    quirks(Provider::Gmail)
        .imap_attributes
        .iter()
        .copied()
        .find(|name| name.eq_ignore_ascii_case("X-GM-EXT-1"))
        .unwrap_or("X-GM-EXT-1")
}

fn fetch_response(tag: &str) -> String {
    let literal = FIXTURE.len();
    // Dovecot's `-1` is body-fld-octets. It is not a size the client should trust.
    format!(
        "* 1 FETCH (BODYSTRUCTURE (\"TEXT\" \"PLAIN\" NIL NIL NIL \"7BIT\" -1 1))\r\n* 1 FETCH (BODY[] {{{literal}}}\r\n{FIXTURE})\r\n{tag} OK FETCH completed\r\n"
    )
}

#[cfg(test)]
mod tests {
    use imap_codec::ResponseCodec;
    use imap_codec::decode::{Decoder, ResponseDecodeError};
    use imap_codec::imap_types::body::BodyStructure;
    use imap_codec::imap_types::fetch::MessageDataItem;
    use imap_codec::imap_types::response::{Data, Response};

    use super::{FIXTURE, Scripted};

    #[test]
    fn greeting_capability_login_select_and_fetch() {
        let mut server = Scripted::new();
        assert_eq!(server.greeting(), "* OK IMAP4rev1 ready\r\n");

        let caps = server.ingest(b"A1 CAPABILITY\r\n");
        let caps = String::from_utf8(caps).unwrap();
        assert!(caps.contains(gmail_extension()));
        assert!(caps.contains("A1 OK"));

        let partial = server.ingest(b"A2 LOG");
        assert!(partial.is_empty());
        let login = server.ingest(b"IN ana secret\r\n");
        assert!(String::from_utf8(login).unwrap().contains("A2 OK"));

        let selected = String::from_utf8(server.ingest(b"A3 SELECT INBOX\r\n")).unwrap();
        assert!(selected.contains("1 EXISTS"));
        assert!(selected.contains("A3 OK"));

        let fetched = server.ingest(b"A4 FETCH 1 (BODY[])\r\n");
        let fetched_text = String::from_utf8(fetched.clone()).unwrap();
        assert!(fetched_text.contains(FIXTURE));
        assert!(fetched_text.contains("-1"));
        assert!(body_octets_are_rectified(&fetched));
    }

    fn gmail_extension() -> &'static str {
        mailune_mime::quirks(mailune_mime::Provider::Gmail)
            .imap_attributes
            .iter()
            .copied()
            .find(|name| name.eq_ignore_ascii_case("X-GM-EXT-1"))
            .unwrap()
    }

    /// Walk the FETCH, skipping the literal the codec will not swallow, and
    /// check Dovecot's `-1` became 0.
    fn body_octets_are_rectified(bytes: &[u8]) -> bool {
        let codec = ResponseCodec::new();
        let mut rest = bytes;
        while !rest.is_empty() {
            match codec.decode(rest) {
                Ok((next, Response::Data(Data::Fetch { items, .. }))) => {
                    let found = items.as_ref().iter().any(|item| {
                        matches!(
                            item,
                            MessageDataItem::BodyStructure(BodyStructure::Single { body, .. })
                                if body.basic.size == 0
                        )
                    });
                    if found {
                        return true;
                    }
                    rest = next;
                }
                Ok((next, _)) => rest = next,
                Err(ResponseDecodeError::LiteralFound { length }) => {
                    let marker = format!("{{{length}}}\r\n");
                    let Some(at) = rest
                        .windows(marker.len())
                        .position(|window| window == marker.as_bytes())
                    else {
                        return false;
                    };
                    let start = at + marker.len();
                    let end = start + length as usize;
                    if end > rest.len() {
                        return false;
                    }
                    rest = &rest[end..];
                }
                Err(ResponseDecodeError::Incomplete) | Err(ResponseDecodeError::Failed) => {
                    return false;
                }
            }
        }
        false
    }
}
