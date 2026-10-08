//! In-memory IMAP server for tests. No TCP.
//!
//! The Gmail capability `X-GM-EXT-1` is read from the provider row in
//! `mailune-mime`. That table has no Dovecot row, so the Dovecot fact is
//! copied here: `body-fld-octets` may be `-1`, which `imap-codec` rectifies
//! to 0 when `quirk_rectify_numbers` is on.

use mailune_mime::{Provider, quirks};

/// The one message FETCH returns. CRLF, as on the wire.
pub const FIXTURE: &str = "From: ana@example.com\r\nSubject: Hi\r\n\r\nHello\r\n";

/// One message the scripted mailbox can SEARCH and UID FETCH.
#[derive(Debug, Clone)]
pub struct MailboxMessage {
    /// UID. `0` is not a valid IMAP uid and is ignored by SEARCH.
    pub uid: u32,
    /// Internal date as `YYYYMMDD`. The day window compares this number.
    pub day: u32,
    /// Flags as the server prints them, including the leading backslash.
    pub flags: Vec<String>,
    /// Bare address, `local@domain`.
    pub from: String,
    /// Envelope subject.
    pub subject: String,
    /// RFC 5322 bytes, counted by BODYSTRUCTURE.
    pub raw: String,
    /// CONDSTORE modification sequence. A later value is a newer change.
    pub modseq: u64,
}

/// Scripted server. Feed client bytes with [`Scripted::ingest`].
#[derive(Debug)]
pub struct Scripted {
    pending: Vec<u8>,
    authed: bool,
    selected: bool,
    uid_validity: u32,
    messages: Vec<MailboxMessage>,
    /// Commands other than LOGIN. LOGIN carries the password, so it stays out.
    trace: Vec<String>,
    /// CONDSTORE and QRESYNC are advertised only when this is set.
    /// `imap-codec` 1.0.0 marks that extension unfinished, so VANISHED is
    /// written by this server and read by the sync client, not by the codec.
    qresync: bool,
    /// Uids removed from the mailbox, with the modseq of the removal.
    vanished: Vec<(u32, u64)>,
}

impl Scripted {
    /// A server that has not greeted yet, with the single fixture message.
    pub fn new() -> Self {
        Self::with_messages(1, vec![fixture_message()])
    }

    /// A mailbox with `uid_validity` and these messages. Nothing is selected yet.
    pub fn with_messages(uid_validity: u32, mut messages: Vec<MailboxMessage>) -> Self {
        messages.retain(|message| message.uid > 0);
        messages.sort_by_key(|message| message.uid);
        Self {
            pending: Vec::new(),
            authed: false,
            selected: false,
            uid_validity,
            messages,
            trace: Vec::new(),
            qresync: false,
            vanished: Vec::new(),
        }
    }

    /// Advertise CONDSTORE and QRESYNC, and answer CHANGEDSINCE with VANISHED.
    pub fn qresync(mut self) -> Self {
        self.qresync = true;
        self
    }

    /// Remember a uid that left the mailbox at `modseq`.
    pub fn record_vanished(mut self, uid: u32, modseq: u64) -> Self {
        if uid > 0 {
            self.vanished.push((uid, modseq));
        }
        self
    }

    /// Commands this server has answered, apart from LOGIN.
    pub fn trace(&self) -> &[String] {
        &self.trace
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
        if !line.to_ascii_uppercase().contains(" LOGIN ")
            && !line.to_ascii_uppercase().starts_with("LOGIN ")
        {
            self.trace.push(line.to_string());
        }
        let mut parts = line.split_whitespace();
        let tag = parts.next().unwrap_or("*");
        let mut verb = parts.next().unwrap_or("").to_ascii_uppercase();
        let mut uid_command = false;
        if verb == "UID" {
            uid_command = true;
            verb = parts.next().unwrap_or("").to_ascii_uppercase();
        }
        let rest: Vec<&str> = parts.collect();
        match verb.as_str() {
            "CAPABILITY" => {
                let gmail = gmail_capability();
                let extra = if self.qresync {
                    " CONDSTORE QRESYNC"
                } else {
                    ""
                };
                format!(
                    "* CAPABILITY IMAP4rev1 {gmail}{extra}\r\n{tag} OK CAPABILITY completed\r\n"
                )
            }
            "LOGIN" => {
                self.authed = true;
                format!("{tag} OK LOGIN completed\r\n")
            }
            "SELECT" if self.authed => self.select(tag),
            "SEARCH" if self.selected && uid_command => self.search(tag, &rest),
            "FETCH" if self.selected && uid_command => self.uid_fetch(tag, line),
            "FETCH" if self.selected => fetch_response(tag),
            "SELECT" | "FETCH" | "SEARCH" => format!("{tag} NO not authenticated\r\n"),
            _ => format!("{tag} BAD unknown command\r\n"),
        }
    }

    fn select(&mut self, tag: &str) -> String {
        self.selected = true;
        let exists = self.messages.len();
        format!(
            "* OK [UIDVALIDITY {validity}] UIDs valid\r\n* {exists} EXISTS\r\n{tag} OK [READ-WRITE] SELECT completed\r\n",
            validity = self.uid_validity,
        )
    }

    fn search(&self, tag: &str, rest: &[&str]) -> String {
        let since = rest
            .windows(2)
            .find(|pair| pair[0].eq_ignore_ascii_case("SINCE"))
            .and_then(|pair| parse_imap_date(pair[1]));
        let mut hits = Vec::new();
        for message in &self.messages {
            let kept = since.is_none_or(|day| message.day >= day);
            if kept {
                hits.push(message.uid.to_string());
            }
        }
        let list = hits.join(" ");
        let prefix = if list.is_empty() {
            String::new()
        } else {
            format!(" {list}")
        };
        format!("* SEARCH{prefix}\r\n{tag} OK SEARCH completed\r\n")
    }

    fn uid_fetch(&self, tag: &str, line: &str) -> String {
        let set = line.split_whitespace().nth(3).unwrap_or("");
        let since = changed_since(line);
        let uids = if set == "1:*" || set == "*" {
            self.messages.iter().map(|message| message.uid).collect()
        } else {
            parse_set(set)
        };
        let mut out = String::new();
        for uid in uids {
            let Some((seq, message)) = self
                .messages
                .iter()
                .enumerate()
                .find(|(_, message)| message.uid == uid)
            else {
                continue;
            };
            if since.is_some_and(|floor| message.modseq <= floor) {
                continue;
            }
            out.push_str(&fetch_meta(seq + 1, message));
        }
        if since.is_some() && self.qresync {
            let gone: Vec<String> = self
                .vanished
                .iter()
                .filter(|(_, seq)| since.is_some_and(|floor| *seq > floor))
                .map(|(uid, _)| uid.to_string())
                .collect();
            if !gone.is_empty() {
                out.push_str(&format!("* VANISHED {}\r\n", gone.join(",")));
            }
            out.push_str(&format!(
                "* OK [HIGHESTMODSEQ {}] highest\r\n",
                self.highest_modseq()
            ));
        }
        out.push_str(&format!("{tag} OK FETCH completed\r\n"));
        out
    }

    fn highest_modseq(&self) -> u64 {
        let live = self.messages.iter().map(|message| message.modseq).max();
        let gone = self.vanished.iter().map(|(_, seq)| *seq).max();
        live.max(gone).unwrap_or(0)
    }
}

impl Default for Scripted {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_message() -> MailboxMessage {
    MailboxMessage {
        uid: 1,
        day: 20_261_008,
        flags: vec!["\\Seen".to_string()],
        from: "ana@example.com".to_string(),
        subject: "Hi".to_string(),
        raw: FIXTURE.to_string(),
        modseq: 1,
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

fn fetch_meta(sequence: usize, message: &MailboxMessage) -> String {
    let flags = if message.flags.is_empty() {
        "()".to_string()
    } else {
        format!("({})", message.flags.join(" "))
    };
    let date = format_imap_date(message.day).unwrap_or_else(|| "1-Jan-1970".to_string());
    let subject = quote_imap(&message.subject);
    let from = address_list(&message.from);
    let lines = message
        .raw
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .max(1);
    let size = message.raw.len();
    format!(
        "* {sequence} FETCH (UID {uid} FLAGS {flags} ENVELOPE (\"{date} 00:00:00 +0000\" {subject} {from} NIL NIL NIL NIL NIL NIL NIL) BODYSTRUCTURE (\"TEXT\" \"PLAIN\" NIL NIL NIL \"7BIT\" {size} {lines}))\r\n",
        uid = message.uid,
    )
}

fn address_list(email: &str) -> String {
    let Some((mailbox, host)) = email.split_once('@') else {
        return "NIL".to_string();
    };
    format!("((NIL NIL {} {}))", quote_imap(mailbox), quote_imap(host))
}

fn quote_imap(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// `7-Oct-2026` or the same token with a time after it.
pub(crate) fn parse_imap_date(text: &str) -> Option<u32> {
    let token = text.split_whitespace().next()?;
    let mut parts = token.split('-');
    let day: u32 = parts.next()?.parse().ok()?;
    let month_name = parts.next()?;
    let year: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || day == 0 || day > 31 {
        return None;
    }
    let month = month_index(month_name)?;
    Some(year * 10_000 + month * 100 + day)
}

/// `YYYYMMDD` to the IMAP date token, without a leading zero on the day.
pub(crate) fn format_imap_date(ymd: u32) -> Option<String> {
    let year = ymd / 10_000;
    let month = (ymd / 100) % 100;
    let day = ymd % 100;
    if day == 0 || day > 31 {
        return None;
    }
    let name = MONTHS.get(month.checked_sub(1)? as usize)?;
    Some(format!("{day}-{name}-{year}"))
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn month_index(name: &str) -> Option<u32> {
    MONTHS
        .iter()
        .position(|month| month.eq_ignore_ascii_case(name))
        .map(|index| index as u32 + 1)
}

fn changed_since(line: &str) -> Option<u64> {
    let upper = line.to_ascii_uppercase();
    let start = upper.find("CHANGEDSINCE")? + "CHANGEDSINCE".len();
    let rest = line.get(start..)?.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

pub(crate) fn parse_set(token: &str) -> Vec<u32> {
    let mut uids = Vec::new();
    for part in token.split(',') {
        if let Some((start, end)) = part.split_once(':') {
            let Ok(start) = start.parse::<u32>() else {
                continue;
            };
            let Ok(end) = end.parse::<u32>() else {
                continue;
            };
            let (lo, hi) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            uids.extend(lo..=hi);
        } else if let Ok(uid) = part.parse::<u32>() {
            uids.push(uid);
        }
    }
    uids
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
