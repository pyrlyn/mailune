//! SMTP submission behind a [`Transport`].
//!
//! The message is already built. This crate decides whether those bytes go
//! out now or wait, then returns the copy that belongs in Sent. It does not
//! open a socket and it does not write a store.
//!
//! `lettre`'s SMTP transport opens a socket, and `mailune-mime` already builds
//! the message, so the handoff is a trait. Tests supply an in-memory transport.

use std::time::Duration;

/// Failure from a submission. The text does not include the message body.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// There is nothing to send.
    #[error("message is empty")]
    Empty,
    /// The envelope has no recipient.
    #[error("message has no recipient")]
    NoRecipient,
    /// The transport refused the bytes.
    #[error("transport refused the message")]
    Transport,
}

/// Bytes plus the envelope the transport needs. The body is not parsed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    mail_from: String,
    recipients: Vec<String>,
    bytes: Vec<u8>,
}

impl Message {
    /// A message the caller already built.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] when `bytes` is empty. [`Error::NoRecipient`] when
    /// `recipients` is empty.
    pub fn new(
        mail_from: impl Into<String>,
        recipients: Vec<String>,
        bytes: Vec<u8>,
    ) -> Result<Self, Error> {
        if bytes.is_empty() {
            return Err(Error::Empty);
        }
        if recipients.is_empty() {
            return Err(Error::NoRecipient);
        }
        Ok(Self {
            mail_from: mail_from.into(),
            recipients,
            bytes,
        })
    }

    /// Reverse-path.
    pub fn mail_from(&self) -> &str {
        &self.mail_from
    }

    /// Forward-path.
    pub fn recipients(&self) -> &[String] {
        &self.recipients
    }

    /// The built message.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// When the bytes may leave the device. A zero delay is "now": there is
/// nothing to wait for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    /// Hand the message to the transport immediately.
    Now,
    /// Hold the message. `delay` is how long the caller waits before asking again.
    Later {
        /// How long to wait. Zero is treated as [`Schedule::Now`].
        delay: Duration,
    },
}

/// What the caller does after [`submit`]. The Sent copy is a value. This
/// crate does not write it anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The transport accepted the message.
    Sent {
        /// Bytes to store in Sent.
        sent_copy: Vec<u8>,
    },
    /// The delay has not elapsed. The transport was not called.
    Delayed {
        /// Bytes to store in Sent once the delay elapses, or to keep scheduled.
        sent_copy: Vec<u8>,
        /// The delay the caller asked for.
        delay: Duration,
    },
}

/// Accepts a built message. A fake records the bytes. A real SMTP client
/// would live behind this trait and is not in this crate.
pub trait Transport {
    /// Deliver `message`.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] when the bytes cannot be accepted.
    fn deliver(&mut self, message: &Message) -> Result<(), Error>;
}

/// Hands `message` to `transport`, or holds it when `schedule` is a delay.
///
/// # Errors
///
/// [`Error::Transport`] when the transport refuses a message that is due now.
pub fn submit(
    transport: &mut impl Transport,
    message: &Message,
    schedule: Schedule,
) -> Result<Outcome, Error> {
    match schedule {
        Schedule::Later { delay } if !delay.is_zero() => Ok(Outcome::Delayed {
            sent_copy: message.bytes.clone(),
            delay,
        }),
        Schedule::Now | Schedule::Later { .. } => {
            transport.deliver(message)?;
            Ok(Outcome::Sent {
                sent_copy: message.bytes.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Error, Message, Outcome, Schedule, Transport, submit};

    struct Fake {
        got: Vec<Vec<u8>>,
        refuse: bool,
    }

    impl Transport for Fake {
        fn deliver(&mut self, message: &Message) -> Result<(), Error> {
            if self.refuse {
                return Err(Error::Transport);
            }
            assert_eq!(message.mail_from(), "me@example.com");
            assert_eq!(message.recipients(), ["ada@example.com"]);
            self.got.push(message.as_bytes().to_vec());
            Ok(())
        }
    }

    fn message() -> Message {
        Message::new(
            "me@example.com",
            vec!["ada@example.com".into()],
            b"Subject: Hi\r\n\r\nHello".to_vec(),
        )
        .unwrap()
    }

    #[test]
    fn a_due_message_is_handed_to_the_transport_and_sent_is_a_copy() {
        let message = message();
        let mut transport = Fake {
            got: Vec::new(),
            refuse: false,
        };
        let outcome = submit(&mut transport, &message, Schedule::Now).unwrap();
        let Outcome::Sent { sent_copy } = outcome else {
            panic!("sent");
        };
        assert_eq!(sent_copy, message.as_bytes());
        assert_eq!(transport.got, vec![sent_copy]);
    }

    #[test]
    fn a_delay_does_not_call_the_transport() {
        let message = message();
        let mut transport = Fake {
            got: Vec::new(),
            refuse: false,
        };
        let delay = Duration::from_secs(3600);
        let outcome = submit(&mut transport, &message, Schedule::Later { delay }).unwrap();
        assert_eq!(
            outcome,
            Outcome::Delayed {
                sent_copy: message.as_bytes().to_vec(),
                delay,
            }
        );
        assert!(transport.got.is_empty());
    }

    #[test]
    fn a_zero_delay_is_sent_now() {
        let mut transport = Fake {
            got: Vec::new(),
            refuse: false,
        };
        let outcome = submit(
            &mut transport,
            &message(),
            Schedule::Later {
                delay: Duration::ZERO,
            },
        )
        .unwrap();
        assert!(matches!(outcome, Outcome::Sent { .. }));
        assert_eq!(transport.got.len(), 1);
    }

    #[test]
    fn a_refused_transport_does_not_return_a_sent_copy() {
        let mut transport = Fake {
            got: Vec::new(),
            refuse: true,
        };
        let err = submit(&mut transport, &message(), Schedule::Now).unwrap_err();
        assert_eq!(err, Error::Transport);
        assert!(transport.got.is_empty());
    }

    #[test]
    fn an_empty_body_or_recipient_is_rejected() {
        assert_eq!(
            Message::new("me@example.com", vec!["ada@example.com".into()], Vec::new()).unwrap_err(),
            Error::Empty
        );
        assert_eq!(
            Message::new("me@example.com", Vec::new(), b"Hi".to_vec()).unwrap_err(),
            Error::NoRecipient
        );
    }
}
