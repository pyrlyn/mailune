//! Initial mailbox sync against a session that is already logged in.
//!
//! The batch is the messages the day window keeps. Nothing is written to a
//! store, and the bytes come from whatever stream the session already holds.

use imap_codec::ResponseCodec;
use imap_codec::decode::Decoder;
use imap_codec::imap_types::body::{BodyStructure, SpecificFields};
use imap_codec::imap_types::core::{IString, NString};
use imap_codec::imap_types::fetch::MessageDataItem;
use imap_codec::imap_types::flag::{Flag, FlagFetch};
use imap_codec::imap_types::response::{Code, Data, Response};
use std::io::{Read, Write};

use crate::Error;
use crate::script::{format_imap_date, parse_imap_date};
use crate::session::Connection;

/// Inclusive lower bound of an initial sync, as `YYYYMMDD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayWindow {
    /// Messages dated before this day are left out of the batch.
    pub since: u32,
}

/// One message the initial fetch returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedMessage {
    /// IMAP uid.
    pub uid: u32,
    /// Flags, with the leading backslash the server sent.
    pub flags: Vec<String>,
    /// Envelope from address, `local@domain`.
    pub from: String,
    /// Envelope subject.
    pub subject: String,
    /// Envelope date, as the server sent it.
    pub date: String,
    /// `TYPE/SUBTYPE` from BODYSTRUCTURE.
    pub structure: String,
}

/// UIDVALIDITY plus the messages inside the day window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBatch {
    /// UIDVALIDITY recorded from SELECT.
    pub uid_validity: u32,
    /// Envelope, flags, and structure, in uid order, inside the window.
    pub messages: Vec<SyncedMessage>,
}

/// SELECT, UID SEARCH SINCE, then UID FETCH of envelope, flags, and
/// BODYSTRUCTURE in batches of `batch_size`.
///
/// A `batch_size` of 0 still fetches one uid at a time. Messages the search
/// returns are dropped again when their envelope date is before `window`,
/// so a server that ignores SINCE cannot widen the window.
pub fn initial_sync<S: Read + Write>(
    session: &mut Connection<S>,
    mailbox: &str,
    window: DayWindow,
    batch_size: usize,
) -> Result<SyncBatch, Error> {
    let selected = session.select(mailbox)?;
    let since = format_imap_date(window.since).ok_or(Error::Response)?;
    let searched = session.transact(&format!("UID SEARCH SINCE {since}"))?;
    let uids = search_uids(searched.as_bytes())?;
    let batch_size = batch_size.max(1);
    let mut messages = Vec::new();
    for chunk in uids.chunks(batch_size) {
        let set = chunk
            .iter()
            .map(|uid| uid.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let fetched = session.transact(&format!(
            "UID FETCH {set} (UID FLAGS ENVELOPE BODYSTRUCTURE)"
        ))?;
        messages.extend(parse_fetch(fetched.as_bytes())?);
    }
    messages.retain(|message| in_window(message, window));
    Ok(SyncBatch {
        uid_validity: selected.uid_validity,
        messages,
    })
}

fn in_window(message: &SyncedMessage, window: DayWindow) -> bool {
    parse_imap_date(&message.date).is_some_and(|day| day >= window.since)
}

pub(crate) fn each_response(
    bytes: &[u8],
    mut on_response: impl FnMut(Response<'_>),
) -> Result<(), Error> {
    let codec = ResponseCodec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let (next, response) = codec.decode(rest).map_err(|_| Error::Response)?;
        if next.len() == rest.len() {
            return Err(Error::Response);
        }
        on_response(response);
        rest = next;
    }
    Ok(())
}

fn search_uids(bytes: &[u8]) -> Result<Vec<u32>, Error> {
    let mut uids = Vec::new();
    each_response(bytes, |response| {
        if let Response::Data(Data::Search(found)) = response {
            uids.extend(found.into_iter().map(|uid| uid.get()));
        }
    })?;
    Ok(uids)
}

fn parse_fetch(bytes: &[u8]) -> Result<Vec<SyncedMessage>, Error> {
    let mut messages = Vec::new();
    let mut failed = false;
    each_response(bytes, |response| {
        if let Response::Data(Data::Fetch { items, .. }) = response {
            match message_from_items(items.as_ref()) {
                Some(message) => messages.push(message),
                None => failed = true,
            }
        }
    })?;
    if failed {
        return Err(Error::Response);
    }
    Ok(messages)
}

fn message_from_items(items: &[MessageDataItem<'_>]) -> Option<SyncedMessage> {
    let mut uid = None;
    let mut flags = Vec::new();
    let mut from = String::new();
    let mut subject = String::new();
    let mut date = String::new();
    let mut structure = String::new();
    let mut saw_envelope = false;
    let mut saw_structure = false;
    for item in items {
        match item {
            MessageDataItem::Uid(value) => uid = Some(value.get()),
            MessageDataItem::Flags(found) => flags = flag_names(found),
            MessageDataItem::Envelope(envelope) => {
                saw_envelope = true;
                date = nstring(&envelope.date);
                subject = nstring(&envelope.subject);
                from = first_address(&envelope.from);
            }
            MessageDataItem::BodyStructure(body) => {
                saw_structure = true;
                structure = structure_name(body);
            }
            _ => {}
        }
    }
    if !saw_envelope || !saw_structure {
        return None;
    }
    Some(SyncedMessage {
        uid: uid?,
        flags,
        from,
        subject,
        date,
        structure,
    })
}

fn flag_names(flags: &[FlagFetch<'_>]) -> Vec<String> {
    flags
        .iter()
        .map(|flag| match flag {
            FlagFetch::Recent => "\\Recent".to_string(),
            FlagFetch::Flag(Flag::Answered) => "\\Answered".to_string(),
            FlagFetch::Flag(Flag::Deleted) => "\\Deleted".to_string(),
            FlagFetch::Flag(Flag::Draft) => "\\Draft".to_string(),
            FlagFetch::Flag(Flag::Flagged) => "\\Flagged".to_string(),
            FlagFetch::Flag(Flag::Seen) => "\\Seen".to_string(),
            FlagFetch::Flag(other) => other.to_string(),
        })
        .collect()
}

fn first_address(addresses: &[imap_codec::imap_types::envelope::Address<'_>]) -> String {
    let Some(address) = addresses.first() else {
        return String::new();
    };
    let mailbox = nstring(&address.mailbox);
    let host = nstring(&address.host);
    if mailbox.is_empty() || host.is_empty() {
        mailbox
    } else {
        format!("{mailbox}@{host}")
    }
}

fn nstring(value: &NString<'_>) -> String {
    let Some(inner) = &value.0 else {
        return String::new();
    };
    String::from_utf8_lossy(inner.as_ref()).into_owned()
}

fn structure_name(structure: &BodyStructure<'_>) -> String {
    match structure {
        BodyStructure::Single { body, .. } => match &body.specific {
            SpecificFields::Text { subtype, .. } => format!("TEXT/{}", istring(subtype)),
            SpecificFields::Basic { r#type, subtype } => {
                format!("{}/{}", istring(r#type), istring(subtype))
            }
            SpecificFields::Message { .. } => "MESSAGE/RFC822".to_string(),
        },
        BodyStructure::Multi { subtype, .. } => format!("MULTIPART/{}", istring(subtype)),
    }
}

fn istring(value: &IString<'_>) -> String {
    String::from_utf8_lossy(value.as_ref()).into_owned()
}

/// Pull UIDVALIDITY and EXISTS out of a SELECT reply.
pub(crate) fn parse_select(bytes: &[u8]) -> Result<(u32, u32), Error> {
    let mut uid_validity = None;
    let mut exists = 0;
    each_response(bytes, |response| match response {
        Response::Status(status) => {
            if let Some(Code::UidValidity(value)) = status.code() {
                uid_validity = Some(value.get());
            }
        }
        Response::Data(Data::Exists(count)) => exists = count,
        _ => {}
    })?;
    Ok((uid_validity.ok_or(Error::Response)?, exists))
}

#[cfg(test)]
mod tests {
    use super::{DayWindow, initial_sync};
    use crate::script::{MailboxMessage, Scripted};
    use crate::session::{Config, Connection};

    #[test]
    fn initial_sync_records_uidvalidity_and_applies_the_day_window_in_batches() {
        let server = Scripted::with_messages(
            42,
            vec![
                message(1, 20_261_001, "\\Seen", "old"),
                message(2, 20_261_007, "\\Flagged", "edge"),
                message(3, 20_261_008, "\\Seen", "later"),
            ],
        );
        let mut session = Connection::open_with(config(), server).unwrap();
        session.login().unwrap();
        let batch =
            initial_sync(&mut session, "INBOX", DayWindow { since: 20_261_007 }, 1).unwrap();
        assert_eq!(batch.uid_validity, 42);
        assert_eq!(batch.messages.len(), 2);
        assert_eq!(batch.messages[0].uid, 2);
        assert_eq!(batch.messages[0].subject, "edge");
        assert_eq!(batch.messages[0].from, "bo@example.com");
        assert_eq!(batch.messages[0].flags, vec!["\\Flagged".to_string()]);
        assert_eq!(batch.messages[0].structure, "TEXT/PLAIN");
        assert!(batch.messages[0].date.contains("7-Oct-2026"));
        assert_eq!(batch.messages[1].uid, 3);
        assert!(
            batch
                .messages
                .iter()
                .all(|message| message.subject != "old")
        );
        let fetches = session
            .trace()
            .iter()
            .filter(|line| line.to_ascii_uppercase().contains("FETCH"))
            .count();
        assert_eq!(fetches, 2);
    }

    fn message(uid: u32, day: u32, flag: &str, subject: &str) -> MailboxMessage {
        MailboxMessage {
            uid,
            day,
            flags: vec![flag.to_string()],
            from: "bo@example.com".to_string(),
            subject: subject.to_string(),
            raw: format!("Subject: {subject}\r\n\r\n{subject}\r\n"),
        }
    }

    fn config() -> Config {
        Config {
            tls_required: true,
            username: "ana".to_string(),
            password: "secret".to_string(),
        }
    }
}
