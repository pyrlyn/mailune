//! IMAP mutations: STORE, COPY, MOVE and APPEND.
//!
//! UIDPLUS answers with the new UIDs (COPYUID, APPENDUID), so the store can
//! map a moved or appended message without searching for it. A server
//! without MOVE gets COPY, `\Deleted` and UID EXPUNGE; that fallback needs
//! UIDPLUS, because a plain EXPUNGE would also remove messages another client
//! marked deleted.

use std::io::{Read, Write};

use crate::sync::{quote_mailbox, uid_set};
use crate::{Connection, Error};

/// How STORE changes flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagOp {
    /// Add these flags.
    Add,
    /// Remove these flags.
    Remove,
    /// Replace every flag.
    Replace,
}

/// Where copied or moved messages landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UidMap {
    /// UIDVALIDITY of the destination.
    pub uidvalidity: u32,
    /// `(source UID, destination UID)` pairs.
    pub pairs: Vec<(u32, u32)>,
}

/// An appended message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appended {
    /// UIDVALIDITY of the mailbox.
    pub uidvalidity: u32,
    /// UID of the new message.
    pub uid: u32,
}

impl<S: Read + Write> Connection<S> {
    /// UID STORE on the selected mailbox. Silent: the caller already knows
    /// the result.
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] for an empty UID list or a flag that is not an
    /// atom; a refused command.
    pub fn store_flags(&mut self, uids: &[u32], op: FlagOp, flags: &[&str]) -> Result<(), Error> {
        let set = nonempty_set(uids)?;
        let flags = flag_list(flags)?;
        let op = match op {
            FlagOp::Add => "+FLAGS.SILENT",
            FlagOp::Remove => "-FLAGS.SILENT",
            FlagOp::Replace => "FLAGS.SILENT",
        };
        self.run(&format!("UID STORE {set} {op} ({flags})"))
            .map(drop)
    }

    /// UID COPY to `target`. The map is `None` without UIDPLUS.
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] for an empty list or an unsendable name; a refused
    /// command.
    pub fn copy_messages(&mut self, uids: &[u32], target: &str) -> Result<Option<UidMap>, Error> {
        let set = nonempty_set(uids)?;
        let reply = self.run(&format!("UID COPY {set} {}", quote_mailbox(target)?))?;
        Ok(copy_uid(&reply, uids.len()))
    }

    /// Moves messages to `target`: MOVE when the server has it, otherwise
    /// COPY, `\Deleted` and UID EXPUNGE.
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] when the server has neither MOVE nor UIDPLUS,
    /// checked before anything changes; otherwise as [`Self::copy_messages`].
    pub fn move_messages(&mut self, uids: &[u32], target: &str) -> Result<Option<UidMap>, Error> {
        let set = nonempty_set(uids)?;
        let target = quote_mailbox(target)?;
        if self.has_capability("MOVE") {
            let reply = self.run(&format!("UID MOVE {set} {target}"))?;
            return Ok(copy_uid(&reply, uids.len()));
        }
        if !self.has_capability("UIDPLUS") {
            return Err(Error::Unsupported);
        }
        let reply = self.run(&format!("UID COPY {set} {target}"))?;
        self.run(&format!("UID STORE {set} +FLAGS.SILENT (\\Deleted)"))?;
        self.run(&format!("UID EXPUNGE {set}"))?;
        Ok(copy_uid(&reply, uids.len()))
    }

    /// APPENDs `message` to `mailbox` with `flags`. `None` without UIDPLUS.
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] for an unsendable name or flag; [`Error::Rejected`]
    /// when the server refuses the literal or the command.
    pub fn append(
        &mut self,
        mailbox: &str,
        flags: &[&str],
        message: &[u8],
    ) -> Result<Option<Appended>, Error> {
        let mailbox = quote_mailbox(mailbox)?;
        let flags = flag_list(flags)?;
        let tag = self.send(&format!("APPEND {mailbox} ({flags}) {{{}}}", message.len()))?;
        if !self.read_line()?.starts_with('+') {
            // The tagged NO is already consumed with the line above.
            return Err(Error::Rejected);
        }
        self.send_bytes(message)?;
        self.send_line("")?;
        let reply = self.finish(&tag)?;
        let code = reply.find("[APPENDUID ").map(|at| &reply[at + 11..]);
        Ok(code.and_then(|code| {
            let mut words = code.split([' ', ']']);
            Some(Appended {
                uidvalidity: words.next()?.parse().ok()?,
                uid: words.next()?.parse().ok()?,
            })
        }))
    }
}

fn nonempty_set(uids: &[u32]) -> Result<String, Error> {
    let mut sorted = uids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.is_empty() || sorted[0] == 0 {
        return Err(Error::Argument);
    }
    Ok(uid_set(&sorted))
}

/// Flags as a space-separated list. Only atoms, with an optional leading
/// backslash, so no flag can end the command early.
fn flag_list(flags: &[&str]) -> Result<String, Error> {
    let atom = |flag: &str| {
        let name = flag.strip_prefix('\\').unwrap_or(flag);
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && !b"(){%*\"\\]".contains(&byte))
    };
    if flags.iter().all(|flag| atom(flag)) {
        Ok(flags.join(" "))
    } else {
        Err(Error::Argument)
    }
}

/// The COPYUID response code as UID pairs. Ranges come from the server, so
/// expansion stops at the number of UIDs the client sent.
fn copy_uid(reply: &str, sent: usize) -> Option<UidMap> {
    let at = reply.find("[COPYUID ")?;
    let mut words = reply[at + 9..].split([' ', ']']);
    let uidvalidity = words.next()?.parse().ok()?;
    let source = expand(words.next()?, sent)?;
    let target = expand(words.next()?, sent)?;
    (source.len() == target.len()).then(|| UidMap {
        uidvalidity,
        pairs: source.into_iter().zip(target).collect(),
    })
}

fn expand(set: &str, limit: usize) -> Option<Vec<u32>> {
    let mut uids = Vec::new();
    for range in set.split(',') {
        let (low, high) = range.split_once(':').unwrap_or((range, range));
        let (low, high): (u32, u32) = (low.parse().ok()?, high.parse().ok()?);
        let span = usize::try_from(low.abs_diff(high)).ok()?;
        if uids.len().saturating_add(span) >= limit {
            return None;
        }
        // RFC 4315 lists COPYUID ranges in matching order, low to high.
        uids.extend(low.min(high)..=low.max(high));
    }
    Some(uids)
}

#[cfg(test)]
mod tests {
    use super::{Appended, FlagOp, copy_uid, flag_list};
    use crate::sync::tests::{config, day};
    use crate::{Connection, Error, MemStream, Scripted, SharedServer};

    fn session(server: &SharedServer) -> Connection<MemStream> {
        let mut session = Connection::over(server.connect(), config()).unwrap();
        session.capability().unwrap();
        session.login().unwrap();
        session.select("INBOX").unwrap();
        session
    }

    fn server(capabilities: &[&str]) -> SharedServer {
        let mut server = Scripted::new().with_capabilities(capabilities);
        server.create("Archive", 42);
        for subject in ["a", "b", "c"] {
            server.deliver("INBOX", "ana@acme.io", subject, day(10, 2), None);
        }
        SharedServer::new(server)
    }

    fn uids(server: &SharedServer, mailbox: &str) -> Vec<u32> {
        server
            .with(|s| {
                s.mailbox_mut(mailbox)
                    .unwrap()
                    .messages
                    .iter()
                    .map(|m| m.uid)
                    .collect()
            })
            .unwrap()
    }

    #[test]
    fn store_move_and_append_map_new_uids() {
        let server = server(&["UIDPLUS", "MOVE"]);
        let mut session = session(&server);
        session
            .store_flags(&[2, 1], FlagOp::Add, &["\\Seen", "$Work"])
            .unwrap();
        session
            .store_flags(&[1], FlagOp::Remove, &["$Work"])
            .unwrap();
        let flags = server
            .with(|s| s.mailbox_mut("INBOX").unwrap().messages[0].flags.clone())
            .unwrap();
        assert_eq!(flags, ["\\Seen"]);

        let moved = session.move_messages(&[2, 3], "Archive").unwrap().unwrap();
        assert_eq!(moved.uidvalidity, 42);
        assert_eq!(moved.pairs, [(2, 1), (3, 2)]);
        assert_eq!(uids(&server, "INBOX"), [1, 4]);

        let copied = session.copy_messages(&[4], "Archive").unwrap().unwrap();
        assert_eq!(copied.pairs, [(4, 3)]);
        assert_eq!(uids(&server, "INBOX"), [1, 4]);

        let message = b"From: me@acme.io\r\nSubject: Draft\r\n\r\nBody\r\n";
        let appended = session.append("Archive", &["\\Draft"], message).unwrap();
        assert_eq!(
            appended,
            Some(Appended {
                uidvalidity: 42,
                uid: 4
            })
        );
        let draft = server
            .with(|s| s.mailbox_mut("Archive").unwrap().messages[3].clone())
            .unwrap();
        assert_eq!(draft.subject, "Draft");
        assert_eq!(draft.flags, ["\\Draft"]);
    }

    #[test]
    fn without_move_copy_delete_and_uid_expunge_run() {
        let server = server(&["UIDPLUS"]);
        server.with(|s| s.set_flags("INBOX", 4, &["\\Deleted"]));
        let mut session = session(&server);
        let moved = session.move_messages(&[2], "Archive").unwrap().unwrap();
        assert_eq!(moved.pairs, [(2, 1)]);
        // UID EXPUNGE left the other client's deleted message alone.
        assert_eq!(uids(&server, "INBOX"), [1, 3, 4]);

        let plain = self::server(&[]);
        let mut session = self::session(&plain);
        assert!(matches!(
            session.move_messages(&[2], "Archive"),
            Err(Error::Unsupported)
        ));
        assert_eq!(uids(&plain, "Archive"), Vec::<u32>::new());
        assert_eq!(session.copy_messages(&[2], "Archive").unwrap(), None);
        assert_eq!(session.append("Archive", &[], b"x\r\n").unwrap(), None);
    }

    #[test]
    fn unsafe_arguments_and_hostile_codes_are_refused() {
        let server = server(&["UIDPLUS", "MOVE"]);
        let mut session = session(&server);
        assert!(matches!(
            session.store_flags(&[1], FlagOp::Add, &["\\Seen)\r\nA9 LOGOUT"]),
            Err(Error::Argument)
        ));
        assert!(matches!(
            session.move_messages(&[], "Archive"),
            Err(Error::Argument)
        ));
        assert!(matches!(
            session.copy_messages(&[1], "Arch\r\nive"),
            Err(Error::Argument)
        ));
        assert!(matches!(
            session.move_messages(&[1], "Missing"),
            Err(Error::Rejected)
        ));
        assert!(matches!(
            session.append("Missing", &[], b"x\r\n"),
            Err(Error::Rejected)
        ));
        assert!(flag_list(&["\\Seen", "$Label"]).is_ok());
        assert_eq!(
            copy_uid("A1 OK [COPYUID 7 1:4294967295 1:4294967295] done", 2),
            None
        );
        assert_eq!(copy_uid("A1 OK [COPYUID 7 1,2 5] done", 2), None);
    }
}
