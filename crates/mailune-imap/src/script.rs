//! In-memory IMAP server for tests. No TCP.
//!
//! The Gmail capability `X-GM-EXT-1` is read from the provider row in
//! `mailune-mime`. That table has no Dovecot row, so the Dovecot fact is
//! copied here: `body-fld-octets` may be `-1`, which `imap-codec` rectifies
//! to 0 when `quirk_rectify_numbers` is on.
//!
//! Mailboxes hold message metadata only: enough for SELECT response codes,
//! UID SEARCH SINCE, and UID FETCH of UID, FLAGS, ENVELOPE and BODYSTRUCTURE.
//! Strings go out quoted, never as literals, so every untagged reply is one
//! line.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use mailune_mime::{Provider, quirks};

use crate::sync::{in_set, uid_set};

/// The one message FETCH returns. CRLF, as on the wire.
pub const FIXTURE: &str = "From: ana@example.com\r\nSubject: Hi\r\n\r\nHello\r\n";

/// Capabilities a new server advertises besides `IMAP4rev1` and Gmail's.
const DEFAULT_CAPABILITIES: [&str; 1] = ["CONDSTORE"];

/// One message in a scripted mailbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptMessage {
    /// UID.
    pub uid: u32,
    /// Flags, such as `\Seen`.
    pub flags: Vec<String>,
    /// Last modification sequence.
    pub modseq: u64,
    /// INTERNALDATE, the day SEARCH SINCE compares.
    pub internal: NaiveDate,
    /// Sender address.
    pub from: String,
    /// Subject.
    pub subject: String,
    /// Message-ID with angle brackets.
    pub message_id: String,
    /// File name of one attachment. Makes the message multipart/mixed.
    pub attachment: Option<String>,
}

/// One scripted mailbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptMailbox {
    /// UIDVALIDITY.
    pub uidvalidity: u32,
    /// Next UID to assign.
    pub uidnext: u32,
    /// Messages in UID order. The position plus one is the sequence number.
    pub messages: Vec<ScriptMessage>,
    /// Expunged UIDs and the modseq of their expunge, for VANISHED.
    pub vanished: Vec<(u32, u64)>,
}

impl ScriptMailbox {
    fn new(uidvalidity: u32) -> Self {
        Self {
            uidvalidity,
            uidnext: 1,
            messages: Vec::new(),
            vanished: Vec::new(),
        }
    }
}

/// Scripted server. Feed client bytes with [`Scripted::ingest`].
#[derive(Debug)]
pub struct Scripted {
    pending: Vec<u8>,
    authed: bool,
    selected: Option<String>,
    capabilities: Vec<String>,
    mailboxes: BTreeMap<String, ScriptMailbox>,
    modseq: u64,
    qresync: bool,
}

impl Scripted {
    /// A server that has not greeted yet, with one message in INBOX.
    pub fn new() -> Self {
        let mut server = Self {
            pending: Vec::new(),
            authed: false,
            selected: None,
            capabilities: DEFAULT_CAPABILITIES.map(String::from).to_vec(),
            mailboxes: BTreeMap::new(),
            modseq: 0,
            qresync: false,
        };
        server.create("INBOX", 1);
        let today = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap_or_default();
        server.deliver("INBOX", "ana@example.com", "Hi", today, None);
        server
    }

    /// Replaces the optional capabilities, such as `CONDSTORE` or `MOVE`.
    pub fn with_capabilities(mut self, capabilities: &[&str]) -> Self {
        self.capabilities = capabilities.iter().map(|name| name.to_string()).collect();
        self
    }

    /// Creates an empty mailbox, or empties one that exists.
    pub fn create(&mut self, name: &str, uidvalidity: u32) {
        self.mailboxes
            .insert(name.to_string(), ScriptMailbox::new(uidvalidity));
    }

    /// Adds a message and returns its UID, or `None` for an unknown mailbox.
    pub fn deliver(
        &mut self,
        mailbox: &str,
        from: &str,
        subject: &str,
        internal: NaiveDate,
        attachment: Option<&str>,
    ) -> Option<u32> {
        self.modseq += 1;
        let modseq = self.modseq;
        let target = self.mailboxes.get_mut(mailbox)?;
        let uid = target.uidnext;
        target.uidnext += 1;
        target.messages.push(ScriptMessage {
            uid,
            flags: Vec::new(),
            modseq,
            internal,
            from: from.to_string(),
            subject: subject.to_string(),
            message_id: format!("<{uid}.{}@script>", target.uidvalidity),
            attachment: attachment.map(String::from),
        });
        Some(uid)
    }

    /// Replaces the flags of one message, as another client would.
    pub fn set_flags(&mut self, mailbox: &str, uid: u32, flags: &[&str]) -> bool {
        self.modseq += 1;
        let modseq = self.modseq;
        let message = self
            .mailboxes
            .get_mut(mailbox)
            .and_then(|target| target.messages.iter_mut().find(|m| m.uid == uid));
        let Some(message) = message else {
            return false;
        };
        message.flags = flags.iter().map(|flag| flag.to_string()).collect();
        message.modseq = modseq;
        true
    }

    /// Removes one message, as another client's EXPUNGE would.
    pub fn expunge(&mut self, mailbox: &str, uid: u32) -> bool {
        self.modseq += 1;
        let modseq = self.modseq;
        let Some(target) = self.mailboxes.get_mut(mailbox) else {
            return false;
        };
        let before = target.messages.len();
        target.messages.retain(|message| message.uid != uid);
        let removed = target.messages.len() < before;
        if removed {
            target.vanished.push((uid, modseq));
        }
        removed
    }

    /// A mailbox, for tests that change it between sessions.
    pub fn mailbox_mut(&mut self, name: &str) -> Option<&mut ScriptMailbox> {
        self.mailboxes.get_mut(name)
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

    fn has(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|name| name.eq_ignore_ascii_case(capability))
    }

    fn respond(&mut self, line: &str) -> String {
        let mut parts = line.splitn(3, ' ');
        let tag = parts.next().unwrap_or("*");
        let verb = parts.next().unwrap_or("").to_ascii_uppercase();
        let args = parts.next().unwrap_or("");
        match verb.as_str() {
            "CAPABILITY" => {
                let gmail = gmail_capability();
                let extra = self.capabilities.join(" ");
                format!(
                    "* CAPABILITY IMAP4rev1 {gmail} {extra}\r\n{tag} OK CAPABILITY completed\r\n"
                )
            }
            "LOGIN" => {
                self.authed = true;
                format!("{tag} OK LOGIN completed\r\n")
            }
            "ENABLE" if self.authed => {
                self.qresync = self.has("QRESYNC") && args.to_ascii_uppercase().contains("QRESYNC");
                let enabled = if self.qresync { " QRESYNC" } else { "" };
                format!("* ENABLED{enabled}\r\n{tag} OK ENABLE completed\r\n")
            }
            "SELECT" if self.authed => self.select(tag, args),
            "FETCH" if self.selected.is_some() => fetch_response(tag),
            "UID" if self.selected.is_some() => self.uid(tag, args),
            "SELECT" | "FETCH" | "UID" => format!("{tag} NO not authenticated\r\n"),
            _ => format!("{tag} BAD unknown command\r\n"),
        }
    }

    fn select(&mut self, tag: &str, args: &str) -> String {
        let name = args
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches('"');
        let Some(mailbox) = self.mailboxes.get(name) else {
            return format!("{tag} NO no such mailbox\r\n");
        };
        let mut out = format!(
            "* {} EXISTS\r\n* OK [UIDVALIDITY {}] UIDs valid\r\n* OK [UIDNEXT {}] next\r\n",
            mailbox.messages.len(),
            mailbox.uidvalidity,
            mailbox.uidnext
        );
        if self.has("CONDSTORE") {
            out.push_str(&format!("* OK [HIGHESTMODSEQ {}] modseq\r\n", self.modseq));
        }
        self.selected = Some(name.to_string());
        out + &format!("{tag} OK [READ-WRITE] SELECT completed\r\n")
    }

    fn uid(&mut self, tag: &str, args: &str) -> String {
        let (verb, rest) = args.split_once(' ').unwrap_or((args, ""));
        let Some(mailbox) = self
            .selected
            .as_ref()
            .and_then(|name| self.mailboxes.get(name))
        else {
            return format!("{tag} NO no mailbox\r\n");
        };
        match verb.to_ascii_uppercase().as_str() {
            "SEARCH" => format!("{}{tag} OK SEARCH completed\r\n", search(mailbox, rest)),
            "FETCH" => format!(
                "{}{tag} OK FETCH completed\r\n",
                uid_fetch(mailbox, rest, self.qresync)
            ),
            _ => format!("{tag} BAD unknown UID command\r\n"),
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

/// `UID SEARCH ALL` or `UID SEARCH SINCE d-Mon-yyyy`.
fn search(mailbox: &ScriptMailbox, criteria: &str) -> String {
    let mut words = criteria.split_whitespace();
    let since = match (words.next().map(str::to_ascii_uppercase), words.next()) {
        (Some(key), Some(date)) if key == "SINCE" => {
            NaiveDate::parse_from_str(date.trim_matches('"'), "%d-%b-%Y").ok()
        }
        _ => None,
    };
    let uids: Vec<String> = mailbox
        .messages
        .iter()
        .filter(|message| since.is_none_or(|day| message.internal >= day))
        .map(|message| message.uid.to_string())
        .collect();
    if uids.is_empty() {
        "* SEARCH\r\n".to_string()
    } else {
        format!("* SEARCH {}\r\n", uids.join(" "))
    }
}

/// `UID FETCH <set> (<items>) [(CHANGEDSINCE n [VANISHED])]`. UID and FLAGS
/// always come back; MODSEQ comes back with CHANGEDSINCE.
fn uid_fetch(mailbox: &ScriptMailbox, args: &str, qresync: bool) -> String {
    let (set, items) = args.split_once(' ').unwrap_or((args, ""));
    let items = items.to_ascii_uppercase();
    let changed_since: Option<u64> = items.split_once("CHANGEDSINCE ").and_then(|(_, rest)| {
        rest.split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|digits| digits.parse().ok())
    });
    let max = mailbox.messages.last().map_or(0, |message| message.uid);
    let mut out = String::new();
    if let Some(since) = changed_since.filter(|_| qresync && items.contains("VANISHED")) {
        let gone: Vec<u32> = mailbox
            .vanished
            .iter()
            .filter(|(uid, modseq)| *modseq > since && in_set(set, *uid, u32::MAX))
            .map(|(uid, _)| *uid)
            .collect();
        if !gone.is_empty() {
            out.push_str(&format!("* VANISHED (EARLIER) {}\r\n", uid_set(&gone)));
        }
    }
    for (index, message) in mailbox.messages.iter().enumerate() {
        if !in_set(set, message.uid, max) || changed_since.is_some_and(|n| message.modseq <= n) {
            continue;
        }
        let mut fields = format!("UID {} FLAGS ({})", message.uid, message.flags.join(" "));
        if changed_since.is_some() {
            fields.push_str(&format!(" MODSEQ ({})", message.modseq));
        }
        if items.contains("ENVELOPE") {
            fields.push_str(&format!(" ENVELOPE {}", envelope(message)));
        }
        if items.contains("BODYSTRUCTURE") {
            fields.push_str(&format!(" BODYSTRUCTURE {}", body_structure(message)));
        }
        out.push_str(&format!("* {} FETCH ({fields})\r\n", index + 1));
    }
    out
}

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn envelope(message: &ScriptMessage) -> String {
    let (mailbox, host) = message.from.split_once('@').unwrap_or((&message.from, ""));
    let address = format!("((NIL NIL {} {}))", quoted(mailbox), quoted(host));
    format!(
        "(NIL {} {address} {address} {address} NIL NIL NIL NIL {})",
        quoted(&message.subject),
        quoted(&message.message_id)
    )
}

fn body_structure(message: &ScriptMessage) -> String {
    let text = "(\"TEXT\" \"PLAIN\" (\"CHARSET\" \"UTF-8\") NIL NIL \"7BIT\" 5 1)";
    match &message.attachment {
        None => text.to_string(),
        Some(name) => format!(
            "({text}(\"APPLICATION\" \"PDF\" (\"NAME\" {}) NIL NIL \"BASE64\" 120) \"MIXED\")",
            quoted(name)
        ),
    }
}

#[cfg(test)]
mod tests {
    use imap_codec::ResponseCodec;
    use imap_codec::decode::{Decoder, ResponseDecodeError};
    use imap_codec::imap_types::body::BodyStructure;
    use imap_codec::imap_types::fetch::MessageDataItem;
    use imap_codec::imap_types::response::{Data, Response};

    use super::{FIXTURE, Scripted};
    use crate::sync::in_set;

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
        assert!(selected.contains("[UIDVALIDITY 1]"));
        assert!(selected.contains("A3 OK"));

        let fetched = server.ingest(b"A4 FETCH 1 (BODY[])\r\n");
        let fetched_text = String::from_utf8(fetched.clone()).unwrap();
        assert!(fetched_text.contains(FIXTURE));
        assert!(fetched_text.contains("-1"));
        assert!(body_octets_are_rectified(&fetched));

        let missing = String::from_utf8(server.ingest(b"A5 SELECT Nope\r\n")).unwrap();
        assert!(missing.contains("A5 NO"));
    }

    #[test]
    fn sequence_sets_cover_ranges_lists_and_star() {
        assert!(in_set("1:3,7", 2, 9));
        assert!(in_set("1:3,7", 7, 9));
        assert!(!in_set("1:3,7", 5, 9));
        assert!(in_set("5:*", 9, 9));
        assert!(in_set("*", 9, 9));
        assert!(!in_set("x", 1, 9));
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
