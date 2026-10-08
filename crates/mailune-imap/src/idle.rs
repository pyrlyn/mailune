//! IMAP IDLE against the scripted server.
//!
//! A dropped session records a backoff on [`Clock`] and the next poll
//! reconnects only once that instant has arrived. Nothing here sleeps
//! or opens a socket.

use std::io::{Read, Write};

use imap_codec::imap_types::response::{Data, Response};

use crate::Error;
use crate::session::Connection;
use crate::sync::{each_response, flag_names};

/// Time source for IDLE backoff. Tests advance it; the watch never sleeps.
pub trait Clock {
    /// Monotonic milliseconds.
    fn now_ms(&self) -> u64;
}

/// A clock the test moves forward.
#[derive(Debug, Clone)]
pub struct ManualClock {
    now: u64,
}

impl ManualClock {
    /// Start at `now` milliseconds.
    pub fn new(now: u64) -> Self {
        Self { now }
    }

    /// Move the clock forward. A negative step is not representable.
    pub fn advance(&mut self, by: u64) {
        self.now = self.now.saturating_add(by);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.now
    }
}

/// One untagged update read during IDLE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleUpdate {
    /// `* n EXISTS`.
    Exists(u32),
    /// `* n EXPUNGE`.
    Expunge(u32),
    /// `* n FETCH (FLAGS ...)`.
    Flags {
        /// Sequence number.
        sequence: u32,
        /// Flags, with the leading backslash.
        flags: Vec<String>,
    },
}

/// IDLE plus reconnect backoff driven by an injected clock.
#[derive(Debug)]
pub struct IdleWatch<C> {
    clock: C,
    attempt: u32,
    retry_at: u64,
}

impl<C: Clock> IdleWatch<C> {
    /// A watch that may connect immediately.
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            attempt: 0,
            retry_at: 0,
        }
    }

    /// When the next reconnect is allowed. `0` means immediately.
    pub fn retry_at(&self) -> u64 {
        self.retry_at
    }

    /// The clock, so a test can advance it without sleeping.
    pub fn clock_mut(&mut self) -> &mut C {
        &mut self.clock
    }

    /// Read IDLE updates from a fresh session.
    ///
    /// `connect` runs only when the backoff has elapsed. A drop schedules
    /// the next attempt and still returns updates that arrived first.
    /// [`Error::Waiting`] means the clock is still earlier than [`Self::retry_at`].
    pub fn poll<S, F>(&mut self, connect: F) -> Result<Vec<IdleUpdate>, Error>
    where
        S: Read + Write,
        F: FnOnce() -> Result<Connection<S>, Error>,
    {
        if self.clock.now_ms() < self.retry_at {
            return Err(Error::Waiting);
        }
        let mut session = connect()?;
        let (raw, dropped) = session.read_idle()?;
        let updates = parse_idle(raw.as_bytes())?;
        if dropped {
            let shift = self.attempt.min(3);
            let delay = 1_000u64 << shift;
            self.attempt = self.attempt.saturating_add(1);
            self.retry_at = self.clock.now_ms().saturating_add(delay);
        } else {
            self.attempt = 0;
            self.retry_at = 0;
        }
        Ok(updates)
    }
}

fn parse_idle(bytes: &[u8]) -> Result<Vec<IdleUpdate>, Error> {
    let mut updates = Vec::new();
    each_response(bytes, |response| {
        if let Some(update) = one_update(response) {
            updates.push(update);
        }
    })?;
    Ok(updates)
}

fn one_update(response: Response<'_>) -> Option<IdleUpdate> {
    match response {
        Response::Data(Data::Exists(count)) => Some(IdleUpdate::Exists(count)),
        Response::Data(Data::Expunge(seq)) => Some(IdleUpdate::Expunge(seq.get())),
        Response::Data(Data::Fetch { seq, items }) => {
            let flags = items.as_ref().iter().find_map(|item| match item {
                imap_codec::imap_types::fetch::MessageDataItem::Flags(flags) => {
                    Some(flag_names(flags))
                }
                _ => None,
            })?;
            Some(IdleUpdate::Flags {
                sequence: seq.get(),
                flags,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    use super::{IdleUpdate, IdleWatch, ManualClock};
    use crate::Error;
    use crate::script::Scripted;
    use crate::session::{Config, Connection};

    #[test]
    fn idle_parses_exists_expunge_and_flags() {
        let server = Scripted::new()
            .push_idle("* 4 EXISTS\r\n* 1 EXPUNGE\r\n* 2 FETCH (FLAGS (\\Seen \\Flagged))\r\n");
        let mut watch = IdleWatch::new(ManualClock::new(0));
        let updates = watch.poll(|| open_server(server)).unwrap();
        assert_eq!(
            updates,
            vec![
                IdleUpdate::Exists(4),
                IdleUpdate::Expunge(1),
                IdleUpdate::Flags {
                    sequence: 2,
                    flags: vec!["\\Seen".to_string(), "\\Flagged".to_string()],
                },
            ]
        );
        assert_eq!(watch.retry_at(), 0);
    }

    #[test]
    fn a_dropped_session_backs_off_then_reconnects_on_the_clock() {
        let servers = RefCell::new(VecDeque::from([
            Scripted::new()
                .push_idle("* 4 EXISTS\r\n")
                .drop_after_idle(),
            Scripted::new().push_idle("* 1 EXPUNGE\r\n"),
        ]));
        let connects = Cell::new(0u32);
        let mut watch = IdleWatch::new(ManualClock::new(0));
        let first = watch.poll(|| open_next(&servers, &connects)).unwrap();
        assert_eq!(first, vec![IdleUpdate::Exists(4)]);
        assert_eq!(watch.retry_at(), 1_000);
        assert!(matches!(
            watch.poll(|| open_next(&servers, &connects)),
            Err(Error::Waiting)
        ));
        assert_eq!(connects.get(), 1);
        watch.clock_mut().advance(1_000);
        let second = watch.poll(|| open_next(&servers, &connects)).unwrap();
        assert_eq!(second, vec![IdleUpdate::Expunge(1)]);
        assert_eq!(connects.get(), 2);
        assert_eq!(watch.retry_at(), 0);
    }

    fn open_next(
        servers: &RefCell<VecDeque<Scripted>>,
        connects: &Cell<u32>,
    ) -> Result<Connection<crate::session::MemStream>, Error> {
        connects.set(connects.get() + 1);
        let server = servers.borrow_mut().pop_front().unwrap();
        open_server(server)
    }

    fn open_server(server: Scripted) -> Result<Connection<crate::session::MemStream>, Error> {
        let mut session = Connection::open_with(config(), server)?;
        session.login()?;
        session.select("INBOX")?;
        Ok(session)
    }

    fn config() -> Config {
        Config {
            tls_required: true,
            username: "ana".to_string(),
            password: "secret".to_string(),
        }
    }
}
