//! Incremental sync of one mailbox after an initial sync.
//!
//! With QRESYNC, `UID FETCH ... (CHANGEDSINCE m VANISHED)` returns changed
//! flags and expunged UIDs in one round trip. With CONDSTORE alone the flags
//! come from CHANGEDSINCE and expunges from a diff of UID sets. Without
//! either, every known UID's flags are fetched and diffed. A changed
//! UIDVALIDITY voids the cache, so the caller must run an initial sync again.

use std::collections::BTreeSet;
use std::io::{Read, Write};

use crate::sync::{fetch_metadata, in_set, search_uids};
use crate::{Connection, Error, MessageMeta, Selected, SyncBatch};

/// What the client remembers between syncs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncState {
    /// UIDVALIDITY the UIDs belong to.
    pub uidvalidity: u32,
    /// HIGHESTMODSEQ at the last sync, when the server has CONDSTORE.
    pub highest_modseq: Option<u64>,
    /// UIDs the client holds.
    pub uids: BTreeSet<u32>,
}

/// How the server let the client resynchronize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resync {
    /// CHANGEDSINCE with VANISHED.
    Qresync,
    /// CHANGEDSINCE, expunges from a UID diff.
    Condstore,
    /// Every flag and a UID diff.
    Full,
}

/// New flags of one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagChange {
    /// UID.
    pub uid: u32,
    /// The whole flag set now.
    pub flags: Vec<String>,
    /// MODSEQ of the change, when the server reported it.
    pub modseq: Option<u64>,
}

/// Changes since the last sync. Nothing is stored yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    /// UIDVALIDITY changed. Every cached UID is void; the lists are empty.
    pub reset: bool,
    /// The SELECT this delta came from.
    pub selected: Selected,
    /// How the changes were found.
    pub method: Resync,
    /// Flag changes of known messages.
    pub flags: Vec<FlagChange>,
    /// Known UIDs that are gone.
    pub vanished: Vec<u32>,
    /// Messages above the highest known UID.
    pub new: Vec<MessageMeta>,
}

impl SyncState {
    /// The state an initial sync leaves.
    pub fn from_batch(batch: &SyncBatch) -> Self {
        Self {
            uidvalidity: batch.selected.uidvalidity,
            highest_modseq: batch.selected.highest_modseq,
            uids: batch.messages.iter().map(|message| message.uid).collect(),
        }
    }

    /// Applies `delta`. A reset keeps only the new UIDVALIDITY.
    pub fn apply(&mut self, delta: &Delta) {
        self.uidvalidity = delta.selected.uidvalidity;
        self.highest_modseq = delta.selected.highest_modseq;
        if delta.reset {
            self.uids.clear();
            return;
        }
        for uid in &delta.vanished {
            self.uids.remove(uid);
        }
        self.uids
            .extend(delta.new.iter().map(|message| message.uid));
    }
}

impl<S: Read + Write> Connection<S> {
    /// Finds what changed in `mailbox` since `state`.
    ///
    /// # Errors
    ///
    /// A refused command or an unreadable reply.
    pub fn incremental_sync(&mut self, mailbox: &str, state: &SyncState) -> Result<Delta, Error> {
        let qresync = self.has_capability("QRESYNC");
        if qresync {
            // ENABLE must come before SELECT for VANISHED to be allowed.
            self.run("ENABLE QRESYNC")?;
        }
        let selected = self.select(mailbox)?;
        let mut delta = Delta {
            reset: selected.uidvalidity != state.uidvalidity,
            selected,
            method: Resync::Full,
            flags: Vec::new(),
            vanished: Vec::new(),
            new: Vec::new(),
        };
        if delta.reset {
            return Ok(delta);
        }
        let known_max = state.uids.last().copied().unwrap_or(0);
        if known_max > 0 {
            let since = state
                .highest_modseq
                .filter(|_| selected.highest_modseq.is_some());
            let range = format!("1:{known_max}");
            let reply = match since {
                Some(modseq) if qresync => {
                    delta.method = Resync::Qresync;
                    self.run(&format!(
                        "UID FETCH {range} (UID FLAGS) (CHANGEDSINCE {modseq} VANISHED)"
                    ))?
                }
                Some(modseq) => {
                    delta.method = Resync::Condstore;
                    self.run(&format!(
                        "UID FETCH {range} (UID FLAGS) (CHANGEDSINCE {modseq})"
                    ))?
                }
                None => self.run(&format!("UID FETCH {range} (UID FLAGS)"))?,
            };
            delta.flags = flag_changes(&reply)?
                .into_iter()
                .filter(|change| state.uids.contains(&change.uid))
                .collect();
            delta.vanished = match delta.method {
                Resync::Qresync => vanished(&reply, &state.uids),
                Resync::Condstore => {
                    let present = search_uids(&self.run(&format!("UID SEARCH UID {range}"))?);
                    missing(&state.uids, present)
                }
                Resync::Full => missing(&state.uids, delta.flags.iter().map(|c| c.uid)),
            };
        }
        let fresh = self.run(&format!(
            "UID FETCH {}:* (UID FLAGS ENVELOPE BODYSTRUCTURE)",
            known_max.saturating_add(1)
        ))?;
        // `n:*` above the last UID still names the last message, so filter.
        delta.new = fetch_metadata(&fresh)?
            .into_iter()
            .filter(|message| message.uid > known_max)
            .collect();
        Ok(delta)
    }
}

fn missing(known: &BTreeSet<u32>, present: impl IntoIterator<Item = u32>) -> Vec<u32> {
    let present: BTreeSet<u32> = present.into_iter().collect();
    known.difference(&present).copied().collect()
}

/// Known UIDs named by `* VANISHED` lines. Only known UIDs are kept, so a
/// huge range from the server is never expanded.
fn vanished(reply: &str, known: &BTreeSet<u32>) -> Vec<u32> {
    let sets: Vec<&str> = reply
        .lines()
        .filter_map(|line| line.strip_prefix("* VANISHED "))
        .map(|rest| rest.trim_start_matches("(EARLIER)").trim())
        .collect();
    known
        .iter()
        .copied()
        .filter(|uid| sets.iter().any(|set| in_set(set, *uid, 0)))
        .collect()
}

/// UID, FLAGS and MODSEQ from FETCH lines. Read by hand because
/// `imap-codec` 1.0.0 does not decode MODSEQ yet.
fn flag_changes(reply: &str) -> Result<Vec<FlagChange>, Error> {
    reply
        .lines()
        .filter(|line| line.starts_with("* ") && line.contains(" FETCH ("))
        .map(|line| {
            let uid = after(line, "UID ")
                .and_then(leading_number)
                .and_then(|uid| u32::try_from(uid).ok())
                .ok_or(Error::Response)?;
            let flags = after(line, "FLAGS (")
                .and_then(|rest| rest.split_once(')'))
                .map(|(inside, _)| inside.split_whitespace().map(String::from).collect())
                .ok_or(Error::Response)?;
            let modseq = after(line, "MODSEQ (").and_then(leading_number);
            Ok(FlagChange { uid, flags, modseq })
        })
        .collect()
}

pub(crate) fn after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.find(marker).map(|at| &line[at + marker.len()..])
}

pub(crate) fn leading_number(text: &str) -> Option<u64> {
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    text[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{Resync, SyncState, flag_changes, vanished};
    use crate::sync::tests::{config, day, now};
    use crate::{Connection, MemStream, Scripted, Window};

    const WINDOW: Window = Window {
        days: 30,
        batch: 10,
    };

    fn server(capabilities: &[&str]) -> Scripted {
        let mut server = Scripted::new().with_capabilities(capabilities);
        for subject in ["a", "b", "c", "d"] {
            server.deliver("INBOX", "ana@acme.io", subject, day(10, 2), None);
        }
        server
    }

    fn session(server: Scripted) -> Connection<MemStream> {
        let mut session = Connection::over(MemStream::new(server), config()).unwrap();
        session.capability().unwrap();
        session.login().unwrap();
        session
    }

    /// Initial sync, then another client changes the mailbox, then a new
    /// session resynchronizes from the saved state.
    fn resync(capabilities: &[&str]) -> (SyncState, super::Delta) {
        let mut first = session(server(capabilities));
        let batch = first.initial_sync("INBOX", WINDOW, now()).unwrap();
        let state = SyncState::from_batch(&batch);
        assert_eq!(state.uids.len(), 5);

        let mut changed = server(capabilities);
        changed.set_flags("INBOX", 2, &["\\Seen"]);
        changed.expunge("INBOX", 3);
        changed.deliver("INBOX", "bo@acme.io", "new", day(10, 8), Some("x.pdf"));
        let mut second = session(changed);
        let delta = second.incremental_sync("INBOX", &state).unwrap();
        (state, delta)
    }

    #[test]
    fn qresync_applies_changedsince_and_vanished() {
        let (mut state, delta) = resync(&["CONDSTORE", "QRESYNC", "ENABLE"]);
        assert_eq!(delta.method, Resync::Qresync);
        assert!(!delta.reset);
        assert_eq!(delta.flags.len(), 1);
        assert_eq!(delta.flags[0].uid, 2);
        assert_eq!(delta.flags[0].flags, ["\\Seen"]);
        assert!(delta.flags[0].modseq.is_some());
        assert_eq!(delta.vanished, [3]);
        assert_eq!(delta.new.len(), 1);
        assert_eq!(delta.new[0].uid, 6);
        assert_eq!(delta.new[0].content_type, "multipart/mixed");

        state.apply(&delta);
        assert_eq!(
            state.uids.iter().copied().collect::<Vec<_>>(),
            [1, 2, 4, 5, 6]
        );
        assert_eq!(state.highest_modseq, delta.selected.highest_modseq);
    }

    #[test]
    fn without_qresync_expunges_come_from_a_uid_diff() {
        let (_, condstore) = resync(&["CONDSTORE"]);
        assert_eq!(condstore.method, Resync::Condstore);
        assert_eq!(condstore.flags.len(), 1);
        assert_eq!(condstore.vanished, [3]);
        assert_eq!(condstore.new.len(), 1);

        let (_, plain) = resync(&[]);
        assert_eq!(plain.method, Resync::Full);
        assert_eq!(plain.flags.len(), 4);
        assert_eq!(plain.vanished, [3]);
        assert_eq!(plain.new.len(), 1);
    }

    #[test]
    fn a_new_uidvalidity_resets_the_cache() {
        let mut first = session(server(&["CONDSTORE"]));
        let batch = first.initial_sync("INBOX", WINDOW, now()).unwrap();
        let mut state = SyncState::from_batch(&batch);
        let mut rebuilt = server(&["CONDSTORE"]);
        rebuilt.mailbox_mut("INBOX").unwrap().uidvalidity = 9;
        let delta = session(rebuilt).incremental_sync("INBOX", &state).unwrap();
        assert!(delta.reset);
        assert!(delta.new.is_empty() && delta.flags.is_empty());
        state.apply(&delta);
        assert_eq!(state.uidvalidity, 9);
        assert!(state.uids.is_empty());
    }

    #[test]
    fn hostile_ranges_and_bad_lines_are_handled() {
        let known = [1, 2, 900].into_iter().collect();
        assert_eq!(
            vanished("* VANISHED (EARLIER) 2:4294967295\r\n", &known),
            [2, 900]
        );
        assert!(flag_changes("* 1 FETCH (FLAGS (\\Seen))\r\n").is_err());
        let changes = flag_changes("* 1 FETCH (UID 7 FLAGS () MODSEQ (12))\r\n").unwrap();
        assert_eq!(changes[0].uid, 7);
        assert!(changes[0].flags.is_empty());
        assert_eq!(changes[0].modseq, Some(12));
    }
}
