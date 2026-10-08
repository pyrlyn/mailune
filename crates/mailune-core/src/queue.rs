//! In-memory operation queue.
//!
//! The mailbox moves as soon as the person asks. A second enqueue with the
//! same key does not apply again, so replay is safe. On a move conflict the
//! server mailbox replaces the optimistic one. Undo is refused once the
//! window has passed. Nothing here is stored in SQLite.

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
    /// [`Error::UnknownOp`] when `key` is not a move in this queue.
    pub fn resolve_move(&mut self, key: &IdempotencyKey, server: MailboxId) -> Result<(), Error> {
        let queued = self
            .ops
            .iter_mut()
            .find(|item| &item.key == key)
            .ok_or(Error::UnknownOp)?;
        let Op::Move { thread, .. } = &queued.op;
        let thread = thread.clone();
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
}

fn apply(location: &mut HashMap<ThreadId, MailboxId>, op: &Op) {
    let Op::Move { thread, to, .. } = op;
    location.insert(thread.clone(), to.clone());
}

fn revert(location: &mut HashMap<ThreadId, MailboxId>, op: &Op) {
    let Op::Move { thread, from, .. } = op;
    location.insert(thread.clone(), from.clone());
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
}
