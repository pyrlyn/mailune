//! UID STORE, UID MOVE, and APPEND against the scripted server.
//!
//! MOVE reports COPYUID and APPEND reports APPENDUID. Those are the
//! UIDPLUS mappings. The bytes never leave this process.

use std::io::{Read, Write};

use imap_codec::imap_types::fetch::MessageDataItem;
use imap_codec::imap_types::response::{Data, Response};

use crate::Error;
use crate::session::Connection;
use crate::sync::{each_response, flag_names};

/// Replace the flags on `uid` and return the flags the server stored.
pub fn store_flags<S: Read + Write>(
    session: &mut Connection<S>,
    uid: u32,
    flags: &[&str],
) -> Result<Vec<String>, Error> {
    let list = if flags.is_empty() {
        "()".to_string()
    } else {
        format!("({})", flags.join(" "))
    };
    let reply = session.transact(&format!("UID STORE {uid} FLAGS {list}"))?;
    stored_flags(reply.as_bytes())
}

/// UID MOVE. The returned uid is the destination from COPYUID.
pub fn move_uid<S: Read + Write>(
    session: &mut Connection<S>,
    uid: u32,
    mailbox: &str,
) -> Result<u32, Error> {
    let reply = session.transact(&format!("UID MOVE {uid} {mailbox}"))?;
    mapped_uid(&reply, "COPYUID")
}

/// APPEND `raw` and return the uid from APPENDUID.
pub fn append_message<S: Read + Write>(
    session: &mut Connection<S>,
    mailbox: &str,
    raw: &[u8],
) -> Result<u32, Error> {
    let reply = session.command_literal(&format!("APPEND {mailbox} ()"), raw)?;
    mapped_uid(&reply, "APPENDUID")
}

fn stored_flags(bytes: &[u8]) -> Result<Vec<String>, Error> {
    let mut flags = None;
    each_response(bytes, |response| {
        if let Response::Data(Data::Fetch { items, .. }) = response {
            for item in items.as_ref() {
                if let MessageDataItem::Flags(found) = item {
                    flags = Some(flag_names(found));
                }
            }
        }
    })?;
    flags.ok_or(Error::Response)
}

fn mapped_uid(reply: &str, code: &str) -> Result<u32, Error> {
    let upper = reply.to_ascii_uppercase();
    let start = upper.find(code).ok_or(Error::Response)? + code.len();
    let rest = reply.get(start..).ok_or(Error::Response)?;
    let inside = rest.split(']').next().unwrap_or(rest);
    let uid = inside
        .split_whitespace()
        .filter_map(|token| {
            let digits: String = token.chars().filter(char::is_ascii_digit).collect();
            if digits.is_empty() {
                None
            } else {
                digits.parse::<u32>().ok()
            }
        })
        .next_back();
    uid.ok_or(Error::Response)
}

#[cfg(test)]
mod tests {
    use super::{append_message, move_uid, store_flags};
    use crate::session::{Config, Connection};
    use crate::sync::search_uids;

    #[test]
    fn store_move_and_append_map_uids() {
        let mut session = Connection::open(config()).unwrap();
        session.login().unwrap();
        session.select("INBOX").unwrap();
        let flags = store_flags(&mut session, 1, &["\\Flagged"]).unwrap();
        assert_eq!(flags, vec!["\\Flagged".to_string()]);
        let raw = b"Subject: Note\r\n\r\nHi\r\n";
        let appended = append_message(&mut session, "INBOX", raw).unwrap();
        assert_eq!(appended, 2);
        let moved = move_uid(&mut session, appended, "Trash").unwrap();
        assert_eq!(moved, 3);
        let left = session.transact("UID SEARCH ALL").unwrap();
        assert_eq!(search_uids(left.as_bytes()).unwrap(), vec![1]);
    }

    fn config() -> Config {
        Config {
            tls_required: true,
            username: "ana".to_string(),
            password: "secret".to_string(),
        }
    }
}
