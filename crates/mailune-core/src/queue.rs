//! In-memory operation queue.
//!
//! The mailbox moves as soon as the person asks. Snooze, a reminder and
//! reply-later use the same queue and the same undo window. They are not
//! written as IMAP METADATA. A second enqueue with the same key does not
//! apply again, so replay is safe. On a move conflict the server mailbox
//! replaces the optimistic one. Undo is refused once the window has passed.
//! Nothing here is stored in SQLite.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use mailune_protocol::{MailboxId, ThreadId};

use crate::Error;

/// Identity of one operation. Replay sends the same key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Wraps the caller's key. The contents are not inspected.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrows the key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An optimistic change. `from` is where undo puts a move back.
///
/// Snooze, reminder and reply-later carry the wake time on the op. The
/// queue's own timestamp is when the person asked, which is what the undo
/// window measures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Move one thread from one mailbox to another.
    Move {
        /// Thread the person moved.
        thread: ThreadId,
        /// Mailbox before the optimistic move.
        from: MailboxId,
        /// Mailbox the person asked for.
        to: MailboxId,
    },
    /// Hide a thread until `until`.
    Snooze {
        /// Thread to hide.
        thread: ThreadId,
        /// When it should reappear.
        until: SystemTime,
    },
    /// Surface a thread again at `at`.
    Reminder {
        /// Thread to remind about.
        thread: ThreadId,
        /// When the reminder is due.
        at: SystemTime,
    },
    /// Bring a thread back so the person can answer.
    ReplyLater {
        /// Thread waiting for a reply.
        thread: ThreadId,
        /// When to bring it back.
        at: SystemTime,
    },
}

/// Which scheduled op is waiting. A move is not a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// [`Op::Snooze`].
    Snooze,
    /// [`Op::Reminder`].
    Reminder,
    /// [`Op::ReplyLater`].
    ReplyLater,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pending,
    Done,
    Conflicted,
}

struct Queued {
    key: IdempotencyKey,
    op: Op,
    at: SystemTime,
    status: Status,
}

/// Optimistic moves, keyed for replay.
pub struct Queue {
    window: Duration,
    ops: Vec<Queued>,
    location: HashMap<ThreadId, MailboxId>,
}

impl Queue {
    /// Empty queue. `undo_window` is how long the latest pending move can be undone.
    pub fn new(undo_window: Duration) -> Self {
        Self {
            window: undo_window,
            ops: Vec::new(),
            location: HashMap::new(),
        }
    }

    /// Applies `op` unless `key` was already accepted.
    ///
    /// # Errors
    ///
    /// [`Error::KeyMismatch`] when `key` already names a different operation.
    pub fn enqueue(&mut self, key: IdempotencyKey, op: Op, now: SystemTime) -> Result<(), Error> {
        if let Some(existing) = self.ops.iter().find(|item| item.key == key) {
            if existing.op == op {
                return Ok(());
            }
            return Err(Error::KeyMismatch);
        }
        apply(&mut self.location, &op);
        self.ops.push(Queued {
            key,
            op,
            at: now,
            status: Status::Pending,
        });
        Ok(())
    }

    /// Where the thread sits after optimistic applies and conflict rollbacks.
    pub fn location(&self, thread: &ThreadId) -> Option<&MailboxId> {
        self.location.get(thread)
    }

    /// The server's mailbox wins over the optimistic destination.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownOp`] when `key` is not in this queue.
    /// [`Error::NotAMove`] when `key` is a snooze, reminder, or reply-later.
    pub fn resolve_move(&mut self, key: &IdempotencyKey, server: MailboxId) -> Result<(), Error> {
        let queued = self
            .ops
            .iter_mut()
            .find(|item| &item.key == key)
            .ok_or(Error::UnknownOp)?;
        let thread = match &queued.op {
            Op::Move { thread, .. } => thread.clone(),
            Op::Snooze { .. } | Op::Reminder { .. } | Op::ReplyLater { .. } => {
                return Err(Error::NotAMove);
            }
        };
        queued.status = Status::Conflicted;
        self.location.insert(thread, server);
        Ok(())
    }

    /// Marks a pending op done so undo will not reverse a finished replay.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownOp`] when `key` is absent.
    pub fn ack(&mut self, key: &IdempotencyKey) -> Result<(), Error> {
        let queued = self
            .ops
            .iter_mut()
            .find(|item| &item.key == key)
            .ok_or(Error::UnknownOp)?;
        if queued.status == Status::Pending {
            queued.status = Status::Done;
        }
        Ok(())
    }

    /// Reverses the latest pending op when `now` is still inside the window.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownOp`] when nothing pending can be undone.
    /// [`Error::UndoExpired`] when that op is older than the window.
    pub fn undo(&mut self, now: SystemTime) -> Result<(), Error> {
        let index = self
            .ops
            .iter()
            .rposition(|item| item.status == Status::Pending)
            .ok_or(Error::UnknownOp)?;
        if !within_window(self.ops[index].at, now, self.window) {
            return Err(Error::UndoExpired);
        }
        let queued = self.ops.remove(index);
        revert(&mut self.location, &queued.op);
        Ok(())
    }

    /// Keys still waiting to be replayed, oldest first.
    pub fn pending(&self) -> Vec<&IdempotencyKey> {
        self.ops
            .iter()
            .filter(|item| item.status == Status::Pending)
            .map(|item| &item.key)
            .collect()
    }

    /// Pending ops with the time each was queued, oldest first.
    ///
    /// A store saves this set and replays it through [`Queue::enqueue`] on
    /// the next launch, so the decision rules stay in this type.
    pub fn pending_ops(&self) -> Vec<(&IdempotencyKey, &Op, SystemTime)> {
        self.ops
            .iter()
            .filter(|item| item.status == Status::Pending)
            .map(|item| (&item.key, &item.op, item.at))
            .collect()
    }

    /// Pending snooze, reminder, or reply-later for `thread`. The latest one
    /// wins, so undoing it reveals the previous pending schedule.
    pub fn schedule(&self, thread: &ThreadId) -> Option<(When, SystemTime)> {
        self.ops.iter().rev().find_map(|item| {
            if item.status != Status::Pending {
                return None;
            }
            let (id, at, kind) = schedule_of(&item.op)?;
            (id == thread).then_some((kind, at))
        })
    }

    /// Scheduled ops whose wake time is at or before `now`, oldest first.
    ///
    /// Moves are not scheduled. A time still in the future is not due.
    pub fn due(&self, now: SystemTime) -> Vec<&IdempotencyKey> {
        self.ops
            .iter()
            .filter(|item| item.status == Status::Pending && is_due(&item.op, now))
            .map(|item| &item.key)
            .collect()
    }
}

fn apply(location: &mut HashMap<ThreadId, MailboxId>, op: &Op) {
    // A schedule does not move the thread. Undo drops the op itself.
    if let Op::Move { thread, to, .. } = op {
        location.insert(thread.clone(), to.clone());
    }
}

fn revert(location: &mut HashMap<ThreadId, MailboxId>, op: &Op) {
    if let Op::Move { thread, from, .. } = op {
        location.insert(thread.clone(), from.clone());
    }
}

fn schedule_of(op: &Op) -> Option<(&ThreadId, SystemTime, When)> {
    match op {
        Op::Snooze { thread, until } => Some((thread, *until, When::Snooze)),
        Op::Reminder { thread, at } => Some((thread, *at, When::Reminder)),
        Op::ReplyLater { thread, at } => Some((thread, *at, When::ReplyLater)),
        Op::Move { .. } => None,
    }
}

fn is_due(op: &Op, now: SystemTime) -> bool {
    let Some((_, at, _)) = schedule_of(op) else {
        return false;
    };
    now.duration_since(at).is_ok()
}

fn within_window(at: SystemTime, now: SystemTime, window: Duration) -> bool {
    match now.duration_since(at) {
        Ok(elapsed) => elapsed <= window,
        // A backwards clock step is not an expiry. The window only moves forward.
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use mailune_protocol::{MailboxId, ThreadId};

    use super::{IdempotencyKey, Op, Queue};
    use crate::Error;

    fn ids() -> (ThreadId, MailboxId, MailboxId) {
        (
            ThreadId::new("t"),
            MailboxId::new("inbox"),
            MailboxId::new("archive"),
        )
    }

    fn move_op() -> Op {
        let (thread, from, to) = ids();
        Op::Move { thread, from, to }
    }

    #[test]
    fn the_same_key_applies_once() {
        let mut queue = Queue::new(Duration::from_secs(5));
        let now = SystemTime::UNIX_EPOCH;
        let key = IdempotencyKey::new("k");
        queue.enqueue(key.clone(), move_op(), now).unwrap();
        queue.enqueue(key, move_op(), now).unwrap();
        assert_eq!(queue.pending().len(), 1);
        let (thread, _, archive) = ids();
        assert_eq!(queue.location(&thread), Some(&archive));
    }

    #[test]
    fn a_reused_key_with_a_different_move_is_an_error() {
        let mut queue = Queue::new(Duration::from_secs(5));
        let now = SystemTime::UNIX_EPOCH;
        let key = IdempotencyKey::new("k");
        queue.enqueue(key.clone(), move_op(), now).unwrap();
        let (thread, from, _) = ids();
        let other = Op::Move {
            thread,
            from,
            to: MailboxId::new("trash"),
        };
        assert!(matches!(
            queue.enqueue(key, other, now),
            Err(Error::KeyMismatch)
        ));
    }

    #[test]
    fn a_move_conflict_keeps_the_server_mailbox() {
        let mut queue = Queue::new(Duration::from_secs(5));
        let now = SystemTime::UNIX_EPOCH;
        let key = IdempotencyKey::new("k");
        queue.enqueue(key.clone(), move_op(), now).unwrap();
        let (thread, inbox, _) = ids();
        queue.resolve_move(&key, inbox.clone()).unwrap();
        assert_eq!(queue.location(&thread), Some(&inbox));
        assert!(queue.pending().is_empty());
    }

    #[test]
    fn undo_restores_inside_the_window_and_fails_after_it() {
        let mut queue = Queue::new(Duration::from_secs(5));
        let now = SystemTime::UNIX_EPOCH;
        queue
            .enqueue(IdempotencyKey::new("k"), move_op(), now)
            .unwrap();
        let (thread, inbox, archive) = ids();
        assert_eq!(queue.location(&thread), Some(&archive));
        queue.undo(now + Duration::from_secs(5)).unwrap();
        assert_eq!(queue.location(&thread), Some(&inbox));

        queue
            .enqueue(IdempotencyKey::new("later"), move_op(), now)
            .unwrap();
        assert!(matches!(
            queue.undo(now + Duration::from_secs(6)),
            Err(Error::UndoExpired)
        ));
        assert_eq!(queue.location(&thread), Some(&archive));
    }

    #[test]
    fn snooze_reminder_and_reply_later_share_the_undo_window() {
        let mut queue = Queue::new(Duration::from_secs(5));
        let now = SystemTime::UNIX_EPOCH;
        let wake = now + Duration::from_secs(60);
        let (thread, _, _) = ids();
        queue
            .enqueue(
                IdempotencyKey::new("snooze"),
                Op::Snooze {
                    thread: thread.clone(),
                    until: wake,
                },
                now,
            )
            .unwrap();
        queue
            .enqueue(
                IdempotencyKey::new("remind"),
                Op::Reminder {
                    thread: thread.clone(),
                    at: wake,
                },
                now,
            )
            .unwrap();
        queue
            .enqueue(
                IdempotencyKey::new("later"),
                Op::ReplyLater {
                    thread: thread.clone(),
                    at: wake,
                },
                now,
            )
            .unwrap();
        assert_eq!(
            queue.schedule(&thread),
            Some((super::When::ReplyLater, wake))
        );
        assert!(queue.due(now).is_empty());
        let due = queue.due(wake);
        assert_eq!(due.len(), 3);
        assert!(matches!(
            queue.resolve_move(&IdempotencyKey::new("snooze"), MailboxId::new("inbox")),
            Err(Error::NotAMove)
        ));

        queue.undo(now).unwrap();
        assert_eq!(queue.schedule(&thread), Some((super::When::Reminder, wake)));
        assert!(matches!(
            queue.undo(now + Duration::from_secs(6)),
            Err(Error::UndoExpired)
        ));
        assert_eq!(queue.due(wake).len(), 2);
    }
}
