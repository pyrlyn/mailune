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

    /// Random operations against a model mailbox (T4).
    ///
    /// The model is the person's view: every thread starts in the inbox, a
    /// move's `from` is where the thread is now, and undo drops the latest
    /// pending op. After every step the queue must agree with the model, a
    /// replayed op must change nothing, and a fresh op undone at once must
    /// leave no trace.
    mod props {
        use std::collections::HashMap;
        use std::time::{Duration, SystemTime};

        use mailune_protocol::{MailboxId, ThreadId};
        use proptest::prelude::*;

        use super::super::{IdempotencyKey, Op, Queue, When};
        use crate::Error;

        const WINDOW: Duration = Duration::from_secs(5);
        const THREADS: u8 = 3;
        const MAILBOXES: [&str; 3] = ["inbox", "archive", "trash"];
        const KEYS: u8 = 8;

        #[derive(Debug, Clone)]
        enum Step {
            Enqueue {
                key: u8,
                thread: u8,
                kind: u8,
                to: u8,
                wake: u8,
            },
            Replay(u8),
            Reuse(u8),
            Ack(u8),
            Resolve(u8, u8),
            Undo,
            Tick(u8),
        }

        fn step() -> impl Strategy<Value = Step> {
            prop_oneof![
                4 => (0..KEYS, 0..THREADS, 0..4u8, 0..3u8, 0..20u8).prop_map(
                    |(key, thread, kind, to, wake)| Step::Enqueue { key, thread, kind, to, wake }
                ),
                1 => any::<u8>().prop_map(Step::Replay),
                1 => any::<u8>().prop_map(Step::Reuse),
                1 => (0..KEYS).prop_map(Step::Ack),
                1 => (0..KEYS, 0..3u8).prop_map(|(key, to)| Step::Resolve(key, to)),
                2 => Just(Step::Undo),
                1 => (0..8u8).prop_map(Step::Tick),
            ]
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        enum Status {
            Pending,
            Done,
            Conflicted,
        }

        /// The reference: a list of accepted ops and where each thread is.
        #[derive(Default)]
        struct Model {
            ops: Vec<(String, Op, SystemTime, Status)>,
            at: HashMap<ThreadId, MailboxId>,
        }

        impl Model {
            fn location(&self, thread: &ThreadId) -> MailboxId {
                self.at
                    .get(thread)
                    .cloned()
                    .unwrap_or_else(|| MailboxId::new(MAILBOXES[0]))
            }

            fn find(&mut self, key: &str) -> Option<&mut (String, Op, SystemTime, Status)> {
                self.ops.iter_mut().find(|entry| entry.0 == key)
            }
        }

        /// What a test can see of a queue.
        #[derive(Debug, PartialEq)]
        struct View {
            pending: Vec<String>,
            places: Vec<MailboxId>,
            schedules: Vec<Option<(When, SystemTime)>>,
            due: Vec<String>,
        }

        fn thread(index: u8) -> ThreadId {
            ThreadId::new(format!("t{index}"))
        }

        fn view(queue: &Queue, now: SystemTime) -> View {
            let threads: Vec<ThreadId> = (0..THREADS).map(thread).collect();
            let keys =
                |list: Vec<&IdempotencyKey>| list.iter().map(|k| k.as_str().to_string()).collect();
            View {
                pending: keys(queue.pending()),
                places: threads
                    .iter()
                    .map(|t| {
                        queue
                            .location(t)
                            .cloned()
                            .unwrap_or_else(|| MailboxId::new(MAILBOXES[0]))
                    })
                    .collect(),
                schedules: threads.iter().map(|t| queue.schedule(t)).collect(),
                due: keys(queue.due(now)),
            }
        }

        fn model_view(model: &Model, now: SystemTime) -> View {
            let pending = |filter: &dyn Fn(&Op) -> bool| {
                model
                    .ops
                    .iter()
                    .filter(|entry| entry.3 == Status::Pending && filter(&entry.1))
                    .map(|entry| entry.0.clone())
                    .collect()
            };
            let schedule = |t: &ThreadId| {
                model.ops.iter().rev().find_map(|(_, op, _, status)| {
                    let (id, kind, at) = match op {
                        Op::Snooze { thread, until } => (thread, When::Snooze, *until),
                        Op::Reminder { thread, at } => (thread, When::Reminder, *at),
                        Op::ReplyLater { thread, at } => (thread, When::ReplyLater, *at),
                        Op::Move { .. } => return None,
                    };
                    (*status == Status::Pending && id == t).then_some((kind, at))
                })
            };
            let threads: Vec<ThreadId> = (0..THREADS).map(thread).collect();
            View {
                pending: pending(&|_| true),
                places: threads.iter().map(|t| model.location(t)).collect(),
                schedules: threads.iter().map(schedule).collect(),
                due: pending(&|op| match op {
                    Op::Snooze { until: at, .. }
                    | Op::Reminder { at, .. }
                    | Op::ReplyLater { at, .. } => *at <= now,
                    Op::Move { .. } => false,
                }),
            }
        }

        fn make_op(model: &Model, thread_index: u8, kind: u8, to: u8, wake: SystemTime) -> Op {
            let thread = thread(thread_index);
            match kind {
                0 => Op::Move {
                    from: model.location(&thread),
                    to: MailboxId::new(MAILBOXES[usize::from(to)]),
                    thread,
                },
                1 => Op::Snooze {
                    thread,
                    until: wake,
                },
                2 => Op::Reminder { thread, at: wake },
                _ => Op::ReplyLater { thread, at: wake },
            }
        }

        /// Applies `step` to both sides and checks the step's own invariant.
        fn run(
            queue: &mut Queue,
            model: &mut Model,
            now: &mut SystemTime,
            fresh: &mut u32,
            step: Step,
        ) {
            match step {
                Step::Enqueue {
                    key,
                    thread,
                    kind,
                    to,
                    wake,
                } => {
                    let key = format!("k{key}");
                    let op = make_op(
                        model,
                        thread,
                        kind,
                        to,
                        *now + Duration::from_secs(wake.into()),
                    );
                    let got = queue.enqueue(IdempotencyKey::new(key.clone()), op.clone(), *now);
                    match model.find(&key) {
                        Some(entry) if entry.1 == op => assert!(got.is_ok()),
                        Some(_) => assert!(matches!(got, Err(Error::KeyMismatch))),
                        None => {
                            got.unwrap();
                            if let Op::Move { thread, to, .. } = &op {
                                model.at.insert(thread.clone(), to.clone());
                            }
                            model.ops.push((key, op, *now, Status::Pending));
                        }
                    }
                }
                Step::Replay(_) | Step::Reuse(_) if model.ops.is_empty() => {}
                Step::Replay(pick) => {
                    // Idempotency: the same key and op a second time is a no-op.
                    let (key, op, _, _) = model.ops[usize::from(pick) % model.ops.len()].clone();
                    let before = view(queue, *now);
                    queue.enqueue(IdempotencyKey::new(key), op, *now).unwrap();
                    assert_eq!(view(queue, *now), before);
                }
                Step::Reuse(pick) => {
                    // A key names one op: a different op under it changes nothing.
                    let (key, op, _, _) = model.ops[usize::from(pick) % model.ops.len()].clone();
                    let other = match op {
                        Op::Move { thread, .. } => Op::Snooze {
                            thread,
                            until: SystemTime::UNIX_EPOCH,
                        },
                        Op::Snooze { thread, .. }
                        | Op::Reminder { thread, .. }
                        | Op::ReplyLater { thread, .. } => Op::Move {
                            from: MailboxId::new(MAILBOXES[0]),
                            to: MailboxId::new(MAILBOXES[1]),
                            thread,
                        },
                    };
                    let before = view(queue, *now);
                    assert!(matches!(
                        queue.enqueue(IdempotencyKey::new(key), other, *now),
                        Err(Error::KeyMismatch)
                    ));
                    assert_eq!(view(queue, *now), before);
                }
                Step::Ack(key) => {
                    let key = format!("k{key}");
                    let got = queue.ack(&IdempotencyKey::new(key.clone()));
                    match model.find(&key) {
                        Some(entry) => {
                            got.unwrap();
                            if entry.3 == Status::Pending {
                                entry.3 = Status::Done;
                            }
                        }
                        None => assert!(matches!(got, Err(Error::UnknownOp))),
                    }
                }
                Step::Resolve(key, to) => {
                    let key = format!("k{key}");
                    let server = MailboxId::new(MAILBOXES[usize::from(to)]);
                    let got = queue.resolve_move(&IdempotencyKey::new(key.clone()), server.clone());
                    match model.find(&key).map(|entry| (entry.1.clone(), entry)) {
                        Some((Op::Move { thread, .. }, entry)) => {
                            got.unwrap();
                            entry.3 = Status::Conflicted;
                            model.at.insert(thread, server);
                        }
                        Some(_) => assert!(matches!(got, Err(Error::NotAMove))),
                        None => assert!(matches!(got, Err(Error::UnknownOp))),
                    }
                }
                Step::Undo => {
                    let got = queue.undo(*now);
                    match model
                        .ops
                        .iter()
                        .rposition(|entry| entry.3 == Status::Pending)
                    {
                        None => assert!(matches!(got, Err(Error::UnknownOp))),
                        Some(index) if *now > model.ops[index].2 + WINDOW => {
                            assert!(matches!(got, Err(Error::UndoExpired)));
                        }
                        Some(index) => {
                            got.unwrap();
                            let (_, op, _, _) = model.ops.remove(index);
                            if let Op::Move { thread, from, .. } = op {
                                model.at.insert(thread, from);
                            }
                        }
                    }
                }
                Step::Tick(secs) => *now += Duration::from_secs(secs.into()),
            }
            // Undo invariant: a fresh op undone at once leaves no trace.
            *fresh += 1;
            let before = view(queue, *now);
            let op = make_op(
                model,
                (*fresh % u32::from(THREADS)) as u8,
                (*fresh % 4) as u8,
                1,
                *now,
            );
            queue
                .enqueue(IdempotencyKey::new(format!("fresh{fresh}")), op, *now)
                .unwrap();
            queue.undo(*now).unwrap();
            assert_eq!(view(queue, *now), before);
        }

        proptest! {
            // No regression files: tests do not write to the source tree.
            #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

            #[test]
            fn the_queue_matches_the_model_mailbox(steps in prop::collection::vec(step(), 1..60)) {
                let mut queue = Queue::new(WINDOW);
                let mut model = Model::default();
                let mut now = SystemTime::UNIX_EPOCH;
                let mut fresh = 0;
                for step in steps {
                    run(&mut queue, &mut model, &mut now, &mut fresh, step);
                    prop_assert_eq!(view(&queue, now), model_view(&model, now));
                }
            }
        }
    }
}
