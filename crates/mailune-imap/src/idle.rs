//! IMAP IDLE and reconnecting after a drop.
//!
//! IDLE turns the selected mailbox into a push channel: the server sends
//! untagged EXISTS, EXPUNGE and FETCH lines as they happen. Networks drop
//! idle connections, so [`IdleWatch`] notices a dead session, waits on the
//! injected [`Clock`] with exponential backoff, and reconnects. It never
//! sleeps a thread itself; a fake clock makes the waits instant in tests.

use std::io::{Read, Write};
use std::time::Duration;

use mailune_protocol::Clock;

use crate::incremental::{after, leading_number};
use crate::{Connection, Error};

/// One untagged update received while idling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleEvent {
    /// The mailbox now holds this many messages.
    Exists(u32),
    /// The message at this sequence number is gone.
    Expunge(u32),
    /// A message's flags changed.
    Flags {
        /// Sequence number.
        seq: u32,
        /// UID, when the server included it.
        uid: Option<u32>,
        /// The whole flag set now.
        flags: Vec<String>,
    },
    /// The server is closing the connection.
    Bye,
}

/// Reads one IDLE line. Lines the client does not act on give `None`.
pub fn parse_idle_line(line: &str) -> Option<IdleEvent> {
    let rest = line.strip_prefix("* ")?;
    if rest.to_ascii_uppercase().starts_with("BYE") {
        return Some(IdleEvent::Bye);
    }
    let (number, rest) = rest.split_once(' ')?;
    let seq: u32 = number.parse().ok()?;
    let keyword = rest.split_whitespace().next()?.to_ascii_uppercase();
    match keyword.as_str() {
        "EXISTS" => Some(IdleEvent::Exists(seq)),
        "EXPUNGE" => Some(IdleEvent::Expunge(seq)),
        "FETCH" => {
            let flags = after(rest, "FLAGS (")?.split_once(')')?.0;
            Some(IdleEvent::Flags {
                seq,
                uid: after(rest, "UID ")
                    .and_then(leading_number)
                    .and_then(|uid| u32::try_from(uid).ok()),
                flags: flags.split_whitespace().map(String::from).collect(),
            })
        }
        _ => None,
    }
}

impl<S: Read + Write> Connection<S> {
    /// Starts IDLE on the selected mailbox and returns its tag.
    ///
    /// # Errors
    ///
    /// [`Error::Rejected`] when the server does not answer with a continuation.
    pub fn idle(&mut self) -> Result<String, Error> {
        let tag = self.send("IDLE")?;
        let line = self.read_line()?;
        if line.starts_with('+') {
            Ok(tag)
        } else {
            Err(Error::Rejected)
        }
    }

    /// Updates that have arrived so far. Does not wait for more.
    ///
    /// # Errors
    ///
    /// [`Error::Session`] when the connection dropped.
    pub fn idle_poll(&mut self) -> Result<Vec<IdleEvent>, Error> {
        Ok(self
            .read_available()?
            .iter()
            .filter_map(|line| parse_idle_line(line))
            .collect())
    }

    /// Ends IDLE started under `tag`.
    ///
    /// # Errors
    ///
    /// A dropped stream or a refused DONE.
    pub fn idle_done(&mut self, tag: &str) -> Result<(), Error> {
        self.send_line("DONE")?;
        self.finish(tag).map(drop)
    }
}

/// Exponential reconnect delays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    /// Wait after the first failure.
    pub base: Duration,
    /// Longest wait.
    pub max: Duration,
}

impl Backoff {
    /// Wait before reconnect attempt `failures` (1 for the first).
    pub fn delay(&self, failures: u32) -> Duration {
        let factor = 1u32
            .checked_shl(failures.saturating_sub(1))
            .unwrap_or(u32::MAX);
        self.base.saturating_mul(factor).min(self.max)
    }
}

/// What one [`IdleWatch::step`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tick {
    /// The session is up. These updates arrived.
    Events(Vec<IdleEvent>),
    /// The session dropped and a new one is idling. Updates in the gap were
    /// missed, so the caller should run an incremental sync.
    Reconnected,
    /// The session is down and reconnecting failed after this many tries.
    Retrying(u32),
}

/// Keeps one mailbox in IDLE across drops.
pub struct IdleWatch<S, F> {
    connect: F,
    mailbox: String,
    backoff: Backoff,
    session: Option<Connection<S>>,
    failures: u32,
}

impl<S, F> IdleWatch<S, F>
where
    S: Read + Write,
    F: FnMut() -> Result<Connection<S>, Error>,
{
    /// Connects, signs in, selects `mailbox` and starts IDLE.
    ///
    /// # Errors
    ///
    /// The first connection's failure. Later drops are retried instead.
    pub fn start(mailbox: &str, backoff: Backoff, connect: F) -> Result<Self, Error> {
        let mut watch = Self {
            connect,
            mailbox: mailbox.to_string(),
            backoff,
            session: None,
            failures: 0,
        };
        watch.session = Some(watch.open()?);
        Ok(watch)
    }

    /// Polls the session, or backs off and reconnects when it is down.
    pub async fn step<C: Clock>(&mut self, clock: &C) -> Tick {
        if let Some(session) = self.session.as_mut() {
            match session.idle_poll() {
                Ok(events) if !events.contains(&IdleEvent::Bye) => return Tick::Events(events),
                _ => self.session = None,
            }
        }
        self.failures = self.failures.saturating_add(1);
        clock.sleep(self.backoff.delay(self.failures)).await;
        match self.open() {
            Ok(session) => {
                self.session = Some(session);
                self.failures = 0;
                Tick::Reconnected
            }
            Err(_) => Tick::Retrying(self.failures),
        }
    }

    fn open(&mut self) -> Result<Connection<S>, Error> {
        let mut session = (self.connect)()?;
        session.capability()?;
        session.login()?;
        session.select(&self.mailbox)?;
        session.idle()?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use mailune_protocol::Clock;
    use mailune_testkit::{FakeHost, poll_now};

    use super::{Backoff, IdleEvent, IdleWatch, Tick, parse_idle_line};
    use crate::sync::tests::{config, day};
    use crate::{Connection, Scripted, SharedServer};

    const BACKOFF: Backoff = Backoff {
        base: Duration::from_secs(1),
        max: Duration::from_secs(8),
    };

    #[test]
    fn updates_arrive_and_a_drop_backs_off_then_reconnects() {
        let server = SharedServer::new(Scripted::new());
        let dial = server.clone();
        let mut watch = IdleWatch::start("INBOX", BACKOFF, || {
            Connection::over(dial.connect(), config())
        })
        .unwrap();
        let clock = FakeHost::new();
        let step = |watch: &mut IdleWatch<_, _>| poll_now(watch.step(&clock)).unwrap();

        assert_eq!(step(&mut watch), Tick::Events(vec![]));
        server.with(|s| s.deliver("INBOX", "bo@acme.io", "Hey", day(10, 8), None));
        assert_eq!(step(&mut watch), Tick::Events(vec![IdleEvent::Exists(2)]));
        server.with(|s| s.set_flags("INBOX", 1, &["\\Seen"]));
        server.with(|s| s.expunge("INBOX", 2));
        assert_eq!(
            step(&mut watch),
            Tick::Events(vec![
                IdleEvent::Flags {
                    seq: 1,
                    uid: Some(1),
                    flags: vec!["\\Seen".into()]
                },
                IdleEvent::Expunge(2),
            ])
        );

        server.with(|s| {
            s.drop_connection();
            s.refuse_next(2);
        });
        assert_eq!(step(&mut watch), Tick::Retrying(1));
        assert_eq!(step(&mut watch), Tick::Retrying(2));
        assert_eq!(step(&mut watch), Tick::Reconnected);
        // Waited 1 + 2 + 4 seconds on the fake clock, none on the thread.
        assert_eq!(clock.now(), SystemTime::UNIX_EPOCH + Duration::from_secs(7));
        server.with(|s| s.deliver("INBOX", "cy@acme.io", "Back", day(10, 8), None));
        assert_eq!(step(&mut watch), Tick::Events(vec![IdleEvent::Exists(2)]));
    }

    #[test]
    fn idle_ends_with_done_and_lines_parse() {
        let server = SharedServer::new(Scripted::new());
        let mut session = Connection::over(server.connect(), config()).unwrap();
        session.login().unwrap();
        session.select("INBOX").unwrap();
        let tag = session.idle().unwrap();
        session.idle_done(&tag).unwrap();
        assert!(session.select("INBOX").is_ok());

        assert_eq!(parse_idle_line("* BYE shutting down"), Some(IdleEvent::Bye));
        assert_eq!(parse_idle_line("* OK still here"), None);
        assert_eq!(parse_idle_line("+ idling"), None);
        assert_eq!(
            parse_idle_line("* 4 FETCH (FLAGS ())"),
            Some(IdleEvent::Flags {
                seq: 4,
                uid: None,
                flags: vec![]
            })
        );
        assert_eq!(BACKOFF.delay(1), Duration::from_secs(1));
        assert_eq!(BACKOFF.delay(3), Duration::from_secs(4));
        assert_eq!(BACKOFF.delay(40), Duration::from_secs(8));
    }
}
