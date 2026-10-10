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
//! line, except body sections (`BODY.PEEK[...]`, `BINARY.PEEK[...]`), which
//! are built from the metadata and sent as literals. Built bodies are ASCII, so
//! replies stay strings.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use mailune_mime::{Provider, quirks};

use crate::sync::{in_set, uid_set};

/// The one message FETCH returns. CRLF, as on the wire.
pub const FIXTURE: &str = "From: ana@example.com\r\nSubject: Hi\r\n\r\nHello\r\n";

/// Capabilities a new server advertises besides `IMAP4rev1` and Gmail's.
const DEFAULT_CAPABILITIES: [&str; 4] = ["CONDSTORE", "UIDPLUS", "MOVE", "BINARY"];

/// Text of every scripted body part 1.
const BODY_TEXT: &str = "Hello\r\n";
/// An attachment's bytes: a PDF signature followed by NUL, so BINARY has
/// something a text literal could not carry.
const ATTACHMENT: &[u8] = b"%PDF-\0\x01\x02";
/// [`ATTACHMENT`] in base64, as BODY returns it.
const ATTACHMENT_BASE64: &str = "JVBERi0AAQI=\r\n";

/// Largest APPEND literal the scripted server accepts.
const LITERAL_LIMIT: usize = 1 << 20;

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
    idling: Option<String>,
    pushed: Vec<u8>,
    generation: u64,
    dropped: bool,
    refuse: u32,
    literal: Option<(String, usize)>,
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
            idling: None,
            pushed: Vec::new(),
            generation: 0,
            dropped: false,
            refuse: 0,
            literal: None,
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
        let exists = target.messages.len();
        self.push(mailbox, format!("* {exists} EXISTS\r\n"));
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
        let update = format!("UID {uid} FLAGS ({})", message.flags.join(" "));
        let seq = self
            .mailboxes
            .get(mailbox)
            .and_then(|target| target.messages.iter().position(|m| m.uid == uid));
        if let Some(seq) = seq {
            self.push(mailbox, format!("* {} FETCH ({update})\r\n", seq + 1));
        }
        true
    }

    /// Removes one message, as another client's EXPUNGE would.
    pub fn expunge(&mut self, mailbox: &str, uid: u32) -> bool {
        self.modseq += 1;
        let modseq = self.modseq;
        let Some(target) = self.mailboxes.get_mut(mailbox) else {
            return false;
        };
        let Some(seq) = target.messages.iter().position(|m| m.uid == uid) else {
            return false;
        };
        target.messages.remove(seq);
        target.vanished.push((uid, modseq));
        self.push(mailbox, format!("* {} EXPUNGE\r\n", seq + 1));
        true
    }

    /// Kills the current connection, as a network drop would.
    pub fn drop_connection(&mut self) {
        self.dropped = true;
    }

    /// Answers the next `count` connections with BYE instead of a greeting.
    pub fn refuse_next(&mut self, count: u32) {
        self.refuse = count;
    }

    /// Starts a new connection: session state is cleared, mailboxes stay.
    /// Returns the greeting, or a BYE while refusing.
    pub(crate) fn reconnect(&mut self) -> (u64, &'static str) {
        self.pending.clear();
        self.authed = false;
        self.selected = None;
        self.qresync = false;
        self.idling = None;
        self.pushed.clear();
        self.dropped = false;
        self.generation += 1;
        if self.refuse > 0 {
            self.refuse -= 1;
            self.dropped = true;
            return (self.generation, "* BYE try later\r\n");
        }
        (self.generation, self.greeting())
    }

    /// Whether connection `generation` is still up.
    pub(crate) fn alive(&self, generation: u64) -> bool {
        generation == self.generation && !self.dropped
    }

    /// Untagged updates queued for an idling client.
    pub(crate) fn take_pushed(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pushed)
    }

    fn push(&mut self, mailbox: &str, update: String) {
        if self.idling.is_some() && self.selected.as_deref() == Some(mailbox) {
            self.pushed.extend_from_slice(update.as_bytes());
        }
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
        loop {
            if let Some((head, size)) = self.literal.take() {
                // The literal, then the CRLF that ends the command line.
                let end = self
                    .pending
                    .get(size..)
                    .and_then(|rest| rest.windows(2).position(|window| window == b"\r\n"));
                let Some(end) = end else {
                    self.literal = Some((head, size));
                    break;
                };
                let data: Vec<u8> = self.pending.drain(..size).collect();
                self.pending.drain(..end + 2);
                out.extend_from_slice(self.append(&head, &data).as_bytes());
                continue;
            }
            let Some(split) = self.pending.windows(2).position(|window| window == b"\r\n") else {
                break;
            };
            let line: Vec<u8> = self.pending.drain(..=split + 1).collect();
            let text = String::from_utf8_lossy(&line[..line.len().saturating_sub(2)]);
            let text = text.trim();
            match literal_size(text) {
                Some((head, size, sync)) if size <= LITERAL_LIMIT => {
                    self.literal = Some((head.to_string(), size));
                    if sync {
                        out.extend_from_slice(b"+ Ready\r\n");
                    }
                }
                Some((head, ..)) => {
                    let tag = head.split(' ').next().unwrap_or("*");
                    out.extend_from_slice(format!("{tag} NO literal too large\r\n").as_bytes());
                }
                None => out.extend_from_slice(self.respond(text).as_bytes()),
            }
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
        if let Some(idle_tag) = self.idling.take() {
            return if line.eq_ignore_ascii_case("DONE") {
                format!("{idle_tag} OK IDLE terminated\r\n")
            } else {
                format!("{idle_tag} BAD expected DONE\r\n")
            };
        }
        match verb.as_str() {
            "IDLE" if self.selected.is_some() => {
                self.idling = Some(tag.to_string());
                "+ idling\r\n".to_string()
            }
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
        let (set, rest) = rest.split_once(' ').unwrap_or((rest, ""));
        match verb.to_ascii_uppercase().as_str() {
            "STORE" => return self.store(tag, set, rest),
            "COPY" => return self.copy(tag, set, rest, false),
            "MOVE" if self.has("MOVE") => return self.copy(tag, set, rest, true),
            "EXPUNGE" if self.has("UIDPLUS") => {
                let gone = self.remove(|message| {
                    in_set(set, message.uid, u32::MAX)
                        && message.flags.iter().any(|f| f == "\\Deleted")
                });
                return format!("{gone}{tag} OK EXPUNGE completed\r\n");
            }
            _ => {}
        }
        let rest = if rest.is_empty() {
            set.to_string()
        } else {
            format!("{set} {rest}")
        };
        let rest = rest.as_str();
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

    /// `UID STORE <set> [+-]FLAGS[.SILENT] (<flags>)`.
    fn store(&mut self, tag: &str, set: &str, args: &str) -> String {
        let (op, flags) = args.split_once(' ').unwrap_or((args, ""));
        let op = op.to_ascii_uppercase();
        let flags: Vec<String> = flags
            .trim_matches(['(', ')'])
            .split_whitespace()
            .map(String::from)
            .collect();
        self.modseq += 1;
        let modseq = self.modseq;
        let Some(mailbox) = self
            .selected
            .clone()
            .and_then(|n| self.mailboxes.get_mut(&n))
        else {
            return format!("{tag} NO no mailbox\r\n");
        };
        let mut out = String::new();
        for (index, message) in mailbox.messages.iter_mut().enumerate() {
            if !in_set(set, message.uid, u32::MAX) {
                continue;
            }
            if op.starts_with('+') {
                for flag in &flags {
                    if !message.flags.contains(flag) {
                        message.flags.push(flag.clone());
                    }
                }
            } else if op.starts_with('-') {
                message.flags.retain(|flag| !flags.contains(flag));
            } else {
                message.flags = flags.clone();
            }
            message.modseq = modseq;
            if !op.ends_with(".SILENT") {
                let now = message.flags.join(" ");
                out.push_str(&format!(
                    "* {} FETCH (UID {} FLAGS ({now}))\r\n",
                    index + 1,
                    message.uid
                ));
            }
        }
        out + &format!("{tag} OK STORE completed\r\n")
    }

    /// `UID COPY` or, with `remove`, `UID MOVE`.
    fn copy(&mut self, tag: &str, set: &str, target: &str, remove: bool) -> String {
        let target = target.trim().trim_matches('"');
        let Some(source) = self.selected.clone() else {
            return format!("{tag} NO no mailbox\r\n");
        };
        if !self.mailboxes.contains_key(target) {
            return format!("{tag} NO [TRYCREATE] no such mailbox\r\n");
        }
        let picked: Vec<ScriptMessage> = self.mailboxes.get(&source).map_or(Vec::new(), |m| {
            m.messages
                .iter()
                .filter(|message| in_set(set, message.uid, u32::MAX))
                .cloned()
                .collect()
        });
        let mut copied = Vec::new();
        for message in &picked {
            self.modseq += 1;
            let modseq = self.modseq;
            if let Some(dest) = self.mailboxes.get_mut(target) {
                let uid = dest.uidnext;
                dest.uidnext += 1;
                dest.messages.push(ScriptMessage {
                    uid,
                    modseq,
                    ..message.clone()
                });
                copied.push(uid);
            }
        }
        let validity = self.mailboxes.get(target).map_or(0, |m| m.uidvalidity);
        let source_uids: Vec<u32> = picked.iter().map(|message| message.uid).collect();
        let code = if self.has("UIDPLUS") && !copied.is_empty() {
            format!(
                "[COPYUID {validity} {} {}] ",
                uid_set(&source_uids),
                uid_set(&copied)
            )
        } else {
            String::new()
        };
        if remove {
            let gone = self.remove(|message| source_uids.contains(&message.uid));
            format!("* OK {code}moved\r\n{gone}{tag} OK MOVE completed\r\n")
        } else {
            format!("{tag} OK {code}COPY completed\r\n")
        }
    }

    /// Removes matching messages from the selected mailbox and returns the
    /// EXPUNGE lines, each with the sequence number at the time it went.
    fn remove(&mut self, doomed: impl Fn(&ScriptMessage) -> bool) -> String {
        self.modseq += 1;
        let modseq = self.modseq;
        let Some(mailbox) = self
            .selected
            .clone()
            .and_then(|n| self.mailboxes.get_mut(&n))
        else {
            return String::new();
        };
        let mut out = String::new();
        let mut index = 0;
        while index < mailbox.messages.len() {
            if doomed(&mailbox.messages[index]) {
                let message = mailbox.messages.remove(index);
                mailbox.vanished.push((message.uid, modseq));
                out.push_str(&format!("* {} EXPUNGE\r\n", index + 1));
            } else {
                index += 1;
            }
        }
        out
    }

    /// `APPEND <mailbox> [(<flags>)] {n}` followed by the message.
    fn append(&mut self, head: &str, data: &[u8]) -> String {
        let mut words = head.split_whitespace();
        let tag = words.next().unwrap_or("*").to_string();
        let mailbox = words.nth(1).unwrap_or("").trim_matches('"').to_string();
        let flags: Vec<String> = head
            .split_once('(')
            .and_then(|(_, rest)| rest.split_once(')'))
            .map(|(inside, _)| inside.split_whitespace().map(String::from).collect())
            .unwrap_or_default();
        if !self.authed {
            return format!("{tag} NO not authenticated\r\n");
        }
        let text = String::from_utf8_lossy(data);
        let header = |name: &str| {
            text.lines()
                .take_while(|line| !line.is_empty())
                .find_map(|line| line.strip_prefix(name))
                .map(str::trim)
                .unwrap_or("")
                .to_string()
        };
        let today = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap_or_default();
        let (from, subject) = (header("From:"), header("Subject:"));
        let Some(uid) = self.deliver(&mailbox, &from, &subject, today, None) else {
            return format!("{tag} NO [TRYCREATE] no such mailbox\r\n");
        };
        if let Some(message) = self
            .mailboxes
            .get_mut(&mailbox)
            .and_then(|m| m.messages.last_mut())
        {
            message.flags = flags;
        }
        let validity = self.mailboxes.get(&mailbox).map_or(0, |m| m.uidvalidity);
        if self.has("UIDPLUS") {
            format!("{tag} OK [APPENDUID {validity} {uid}] APPEND completed\r\n")
        } else {
            format!("{tag} OK APPEND completed\r\n")
        }
    }
}

/// `{n}` or `{n+}` at the end of a command line: the text before it, the
/// literal size, and whether the client waits for a continuation.
fn literal_size(line: &str) -> Option<(&str, usize, bool)> {
    let open = line.strip_suffix('}')?.rfind('{')?;
    let inner = &line[open + 1..line.len() - 1];
    let (digits, sync) = match inner.strip_suffix('+') {
        Some(digits) => (digits, false),
        None => (inner, true),
    };
    Some((&line[..open], digits.parse().ok()?, sync))
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
        if let Some(section) = body_section(message, &items) {
            fields.push_str(&section);
        }
        out.push_str(&format!("* {} FETCH ({fields})\r\n", index + 1));
    }
    out
}

/// ` BODY[s]<o> {n}` or ` BINARY[s]<o> ~{n}` and the bytes, for the first
/// `BODY.PEEK[...]` or `BINARY.PEEK[...]` item. An unknown section is NIL.
fn body_section(message: &ScriptMessage, items: &str) -> Option<String> {
    let (name, tilde, rest) = match items.split_once("BINARY.PEEK[") {
        Some((_, rest)) => ("BINARY", "~", rest),
        None => ("BODY", "", items.split_once("BODY.PEEK[")?.1),
    };
    let (section, rest) = rest.split_once(']')?;
    let range = rest
        .strip_prefix('<')
        .and_then(|rest| rest.split_once('>'))
        .and_then(|(range, _)| range.split_once('.'))
        .and_then(|(offset, count)| Some((offset.parse().ok()?, count.parse().ok()?)));
    let origin = range.map_or(String::new(), |(offset, _): (usize, usize)| {
        format!("<{offset}>")
    });
    let Some(bytes) = section_bytes(message, section, name == "BINARY") else {
        return Some(format!(" {name}[{section}]{origin} NIL"));
    };
    let bytes: Vec<u8> = match range {
        Some((offset, count)) => bytes.iter().skip(offset).take(count).copied().collect(),
        None => bytes,
    };
    Some(format!(
        " {name}[{section}]{origin} {tilde}{{{}}}\r\n{}",
        bytes.len(),
        String::from_utf8_lossy(&bytes)
    ))
}

/// One section of the message the metadata describes: part 1 is text, part 2
/// the attachment, base64 for BODY and decoded for BINARY.
fn section_bytes(message: &ScriptMessage, section: &str, binary: bool) -> Option<Vec<u8>> {
    let content_type = if message.attachment.is_some() {
        "multipart/mixed; boundary=\"b\""
    } else {
        "text/plain; charset=UTF-8"
    };
    let header = format!(
        "From: {}\r\nSubject: {}\r\nMessage-ID: {}\r\nMIME-Version: 1.0\r\nContent-Type: {content_type}\r\n\r\n",
        message.from, message.subject, message.message_id
    );
    let text = match &message.attachment {
        None => BODY_TEXT.to_string(),
        Some(name) => format!(
            "--b\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n{BODY_TEXT}--b\r\n\
             Content-Type: application/pdf; name=\"{name}\"\r\n\
             Content-Transfer-Encoding: base64\r\n\r\n{ATTACHMENT_BASE64}--b--\r\n"
        ),
    };
    match (section, &message.attachment) {
        ("", _) => Some(format!("{header}{text}").into_bytes()),
        ("HEADER", _) => Some(header.into_bytes()),
        ("TEXT", _) => Some(text.into_bytes()),
        ("1", _) => Some(BODY_TEXT.as_bytes().to_vec()),
        ("2", Some(_)) if binary => Some(ATTACHMENT.to_vec()),
        ("2", Some(_)) => Some(ATTACHMENT_BASE64.as_bytes().to_vec()),
        _ => None,
    }
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
