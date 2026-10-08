//! Initial sync of one mailbox.
//!
//! SELECT records UIDVALIDITY, so later syncs know when every cached UID
//! became void. UID SEARCH SINCE keeps the first sync to a day window, then
//! UID FETCH reads envelope, flags and BODYSTRUCTURE in batches so one huge
//! mailbox never becomes one huge reply. Bodies are not fetched here, and
//! nothing is written to the store: the caller gets a [`SyncBatch`].

use std::io::{Read, Write};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use imap_codec::ResponseCodec;
use imap_codec::decode::Decoder;
use imap_codec::imap_types::body::{BodyStructure, SpecificFields};
use imap_codec::imap_types::core::{IString, NString};
use imap_codec::imap_types::envelope::Address;
use imap_codec::imap_types::fetch::MessageDataItem;
use imap_codec::imap_types::flag::FlagFetch;
use imap_codec::imap_types::response::{Data, Response};

use crate::{Connection, Error};

const DAY: Duration = Duration::from_secs(86_400);

/// What SELECT reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selected {
    /// Messages in the mailbox.
    pub exists: u32,
    /// UIDVALIDITY. A change voids every cached UID.
    pub uidvalidity: u32,
    /// The UID the next message will get.
    pub uidnext: u32,
    /// HIGHESTMODSEQ, when the server supports CONDSTORE.
    pub highest_modseq: Option<u64>,
}

/// Envelope, flags and structure of one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageMeta {
    /// UID.
    pub uid: u32,
    /// Flags, such as `\Seen`.
    pub flags: Vec<String>,
    /// Subject, as sent (still RFC 2047 encoded when the sender encoded it).
    pub subject: Option<String>,
    /// Sender addresses, `mailbox@host`.
    pub from: Vec<String>,
    /// Message-ID.
    pub message_id: Option<String>,
    /// Top-level MIME type, such as `multipart/mixed`.
    pub content_type: String,
    /// Leaf parts in the structure.
    pub parts: usize,
}

/// How much the first sync reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// Days back from now. Older mail is left for a later backfill.
    pub days: u32,
    /// UIDs per FETCH.
    pub batch: usize,
}

/// The result of an initial sync. Nothing is stored yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBatch {
    /// Mailbox name.
    pub mailbox: String,
    /// The SELECT that started the sync.
    pub selected: Selected,
    /// Messages inside the window, in UID order.
    pub messages: Vec<MessageMeta>,
    /// FETCH commands sent.
    pub fetches: usize,
}

impl<S: Read + Write> Connection<S> {
    /// SELECT `mailbox`.
    ///
    /// # Errors
    ///
    /// [`Error::Rejected`] for an unknown mailbox, [`Error::Response`] when
    /// the reply has no UIDVALIDITY.
    pub fn select(&mut self, mailbox: &str) -> Result<Selected, Error> {
        let reply = self.run(&format!("SELECT {}", quote_mailbox(mailbox)))?;
        Ok(Selected {
            exists: untagged_number(&reply, "EXISTS").unwrap_or(0),
            uidvalidity: response_code(&reply, "UIDVALIDITY")
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(Error::Response)?,
            uidnext: response_code(&reply, "UIDNEXT")
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0),
            highest_modseq: response_code(&reply, "HIGHESTMODSEQ"),
        })
    }

    /// Reads the metadata of every message in `mailbox` that arrived within
    /// the window before `now`.
    ///
    /// # Errors
    ///
    /// A refused command or an unreadable FETCH reply.
    pub fn initial_sync(
        &mut self,
        mailbox: &str,
        window: Window,
        now: SystemTime,
    ) -> Result<SyncBatch, Error> {
        let selected = self.select(mailbox)?;
        let since = now
            .checked_sub(DAY * window.days)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let day = DateTime::<Utc>::from(since).date_naive();
        let found = self.run(&format!("UID SEARCH SINCE {}", day.format("%-d-%b-%Y")))?;
        let mut uids = search_uids(&found);
        uids.sort_unstable();
        let mut messages = Vec::with_capacity(uids.len());
        let mut fetches = 0;
        for chunk in uids.chunks(window.batch.max(1)) {
            let reply = self.run(&format!(
                "UID FETCH {} (UID FLAGS ENVELOPE BODYSTRUCTURE)",
                uid_set(chunk)
            ))?;
            fetches += 1;
            messages.extend(fetch_metadata(&reply)?);
        }
        messages.sort_by_key(|message| message.uid);
        Ok(SyncBatch {
            mailbox: mailbox.to_string(),
            selected,
            messages,
            fetches,
        })
    }
}

/// Quotes a mailbox name unless it is a plain atom.
pub(crate) fn quote_mailbox(name: &str) -> String {
    if !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'-' | b'_'))
    {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// `n` from a `* n KEYWORD` line.
pub(crate) fn untagged_number(reply: &str, keyword: &str) -> Option<u32> {
    reply.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        match (words.next(), words.next(), words.next()) {
            (Some("*"), Some(number), Some(word)) if word.eq_ignore_ascii_case(keyword) => {
                number.parse().ok()
            }
            _ => None,
        }
    })
}

/// The number in a `[CODE n]` response code.
pub(crate) fn response_code(reply: &str, code: &str) -> Option<u64> {
    let marker = format!("[{code} ");
    let start = reply.find(&marker)? + marker.len();
    let digits: String = reply[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// UIDs from `* SEARCH` lines.
pub(crate) fn search_uids(reply: &str) -> Vec<u32> {
    reply
        .lines()
        .filter_map(|line| line.strip_prefix("* SEARCH"))
        .flat_map(|rest| rest.split_whitespace().filter_map(|word| word.parse().ok()))
        .collect()
}

/// Compresses sorted UIDs into a sequence set such as `1:3,7`.
pub(crate) fn uid_set(uids: &[u32]) -> String {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for &uid in uids {
        match ranges.last_mut() {
            Some((_, high)) if high.checked_add(1) == Some(uid) => *high = uid,
            _ => ranges.push((uid, uid)),
        }
    }
    ranges
        .iter()
        .map(|(low, high)| {
            if low == high {
                low.to_string()
            } else {
                format!("{low}:{high}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn fetch_metadata(reply: &str) -> Result<Vec<MessageMeta>, Error> {
    let codec = ResponseCodec::new();
    let mut messages = Vec::new();
    for line in reply.lines().filter(|line| line.contains(" FETCH (")) {
        let wire = format!("{line}\r\n");
        let Ok((_, Response::Data(Data::Fetch { items, .. }))) = codec.decode(wire.as_bytes())
        else {
            return Err(Error::Response);
        };
        let mut meta = MessageMeta {
            uid: 0,
            flags: Vec::new(),
            subject: None,
            from: Vec::new(),
            message_id: None,
            content_type: String::new(),
            parts: 0,
        };
        for item in items.as_ref() {
            match item {
                MessageDataItem::Uid(uid) => meta.uid = uid.get(),
                MessageDataItem::Flags(flags) => {
                    meta.flags = flags
                        .iter()
                        .filter_map(|flag| match flag {
                            FlagFetch::Flag(flag) => Some(flag.to_string()),
                            FlagFetch::Recent => None,
                        })
                        .collect();
                }
                MessageDataItem::Envelope(envelope) => {
                    meta.subject = text(&envelope.subject);
                    meta.message_id = text(&envelope.message_id);
                    meta.from = envelope.from.iter().filter_map(address).collect();
                }
                MessageDataItem::BodyStructure(structure) => {
                    meta.content_type = content_type(structure);
                    meta.parts = leaves(structure);
                }
                _ => {}
            }
        }
        if meta.uid == 0 {
            return Err(Error::Response);
        }
        messages.push(meta);
    }
    Ok(messages)
}

fn text(value: &NString<'_>) -> Option<String> {
    value.0.as_ref().map(istring)
}

fn istring(value: &IString<'_>) -> String {
    String::from_utf8_lossy(value.as_ref()).into_owned()
}

fn address(address: &Address<'_>) -> Option<String> {
    let mailbox = text(&address.mailbox)?;
    match text(&address.host) {
        Some(host) if !host.is_empty() => Some(format!("{mailbox}@{host}")),
        _ => Some(mailbox),
    }
}

fn content_type(structure: &BodyStructure<'_>) -> String {
    match structure {
        BodyStructure::Single { body, .. } => match &body.specific {
            SpecificFields::Basic { r#type, subtype } => {
                format!("{}/{}", istring(r#type), istring(subtype)).to_ascii_lowercase()
            }
            SpecificFields::Message { .. } => "message/rfc822".to_string(),
            SpecificFields::Text { subtype, .. } => {
                format!("text/{}", istring(subtype)).to_ascii_lowercase()
            }
        },
        BodyStructure::Multi { subtype, .. } => {
            format!("multipart/{}", istring(subtype)).to_ascii_lowercase()
        }
    }
}

fn leaves(structure: &BodyStructure<'_>) -> usize {
    match structure {
        BodyStructure::Single { .. } => 1,
        BodyStructure::Multi { bodies, .. } => bodies.as_ref().iter().map(leaves).sum(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use chrono::NaiveDate;

    use super::{Window, quote_mailbox, uid_set};
    use crate::{Config, Connection, Error, MemStream, Scripted};

    pub(crate) fn config() -> Config {
        Config {
            tls_required: true,
            username: "ana".into(),
            password: "secret".into(),
        }
    }

    pub(crate) fn day(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    /// 2026-10-08T12:00:00Z.
    pub(crate) fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_791_460_800)
    }

    #[test]
    fn a_sync_records_uidvalidity_and_fetches_the_window_in_batches() {
        let mut server = Scripted::new();
        server.create("Archive", 77);
        server.deliver("Archive", "old@example.com", "Old", day(8, 1), None);
        for (index, date) in [day(10, 1), day(10, 5), day(10, 6), day(10, 8)]
            .into_iter()
            .enumerate()
        {
            let attachment = (index == 1).then_some("deck.pdf");
            let subject = format!("Note \"{index}\"");
            server.deliver("Archive", "ana@acme.io", &subject, date, attachment);
        }
        let mut session = Connection::over(MemStream::new(server), config()).unwrap();
        session.capability().unwrap();
        session.login().unwrap();

        let window = Window { days: 7, batch: 2 };
        let batch = session.initial_sync("Archive", window, now()).unwrap();
        assert_eq!(batch.selected.uidvalidity, 77);
        assert_eq!(batch.selected.exists, 5);
        assert_eq!(batch.selected.uidnext, 6);
        assert!(batch.selected.highest_modseq.is_some());
        // 2026-10-01 is the first day inside a 7-day window; August is not.
        let uids: Vec<u32> = batch.messages.iter().map(|m| m.uid).collect();
        assert_eq!(uids, [2, 3, 4, 5]);
        assert_eq!(batch.fetches, 2);

        let with_pdf = &batch.messages[1];
        assert_eq!(with_pdf.subject.as_deref(), Some("Note \"1\""));
        assert_eq!(with_pdf.from, ["ana@acme.io"]);
        assert_eq!(with_pdf.message_id.as_deref(), Some("<3.77@script>"));
        assert_eq!(with_pdf.content_type, "multipart/mixed");
        assert_eq!(with_pdf.parts, 2);
        assert_eq!(batch.messages[0].content_type, "text/plain");
        assert!(batch.messages[0].flags.is_empty());

        assert!(matches!(
            session.initial_sync("Missing", window, now()),
            Err(Error::Rejected)
        ));
    }

    #[test]
    fn sets_and_names_are_encoded_for_the_wire() {
        assert_eq!(uid_set(&[1, 2, 3, 7, 9, 10]), "1:3,7,9:10");
        assert_eq!(uid_set(&[]), "");
        assert_eq!(quote_mailbox("INBOX"), "INBOX");
        assert_eq!(quote_mailbox("Sent Items"), "\"Sent Items\"");
    }
}
