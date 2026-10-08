//! Property tests for the in-memory operation queue.
//!
//! proptest is a dev-dependency of this crate only. `mailune-core`'s
//! dev-dependencies count in the crate-boundary check, so the harness cannot
//! live there.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use mailune_core::{Error, IdempotencyKey, Op, Queue};
use mailune_protocol::{MailboxId, ThreadId};
use proptest::prelude::*;

const START: SystemTime = SystemTime::UNIX_EPOCH;
const WINDOW: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
enum Step {
    Move {
        key: u8,
        thread: u8,
        from: u8,
        to: u8,
    },
    Repeat,
    Undo {
        expired: bool,
    },
}

#[derive(Clone, Debug)]
struct Move {
    key: String,
    thread: String,
    from: String,
    to: String,
}

struct Model {
    location: HashMap<String, String>,
    pending: Vec<Move>,
    keys: HashMap<String, String>,
    last: Option<Move>,
}

fn arb_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(key, thread, from, to)| {
            Step::Move {
                key,
                thread,
                from,
                to,
            }
        }),
        Just(Step::Repeat),
        any::<bool>().prop_map(|expired| Step::Undo { expired }),
    ]
}

fn name(prefix: char, value: u8) -> String {
    format!("{prefix}{value}")
}

fn fingerprint(mv: &Move) -> String {
    format!("{}|{}|{}", mv.thread, mv.from, mv.to)
}

fn op_of(mv: &Move) -> Op {
    Op::Move {
        thread: ThreadId::new(mv.thread.clone()),
        from: MailboxId::new(mv.from.clone()),
        to: MailboxId::new(mv.to.clone()),
    }
}

fn enqueue(queue: &mut Queue, model: &mut Model, mv: Move) {
    let op = op_of(&mv);
    let key = IdempotencyKey::new(mv.key.clone());
    let print = fingerprint(&mv);
    if let Some(existing) = model.keys.get(&mv.key) {
        if existing == &print {
            queue.enqueue(key, op, START).unwrap();
            model.last = Some(mv);
            return;
        }
        assert!(matches!(
            queue.enqueue(key, op, START),
            Err(Error::KeyMismatch)
        ));
        return;
    }
    queue.enqueue(key, op, START).unwrap();
    model.location.insert(mv.thread.clone(), mv.to.clone());
    model.keys.insert(mv.key.clone(), print);
    model.pending.push(mv.clone());
    model.last = Some(mv);
}

fn undo(queue: &mut Queue, model: &mut Model, expired: bool) {
    let now = if expired {
        START + Duration::from_secs(6)
    } else {
        START
    };
    if model.pending.is_empty() {
        assert!(matches!(queue.undo(now), Err(Error::UnknownOp)));
        return;
    }
    if expired {
        assert!(matches!(queue.undo(now), Err(Error::UndoExpired)));
        return;
    }
    let undone = model.pending.pop().unwrap();
    model.keys.remove(&undone.key);
    model
        .location
        .insert(undone.thread.clone(), undone.from.clone());
    queue.undo(now).unwrap();
}

fn agree(queue: &Queue, model: &Model) {
    for (thread, mailbox) in &model.location {
        assert_eq!(
            queue
                .location(&ThreadId::new(thread.clone()))
                .map(MailboxId::as_str),
            Some(mailbox.as_str())
        );
    }
    let pending: Vec<&str> = queue.pending().iter().map(|key| key.as_str()).collect();
    let expected: Vec<&str> = model.pending.iter().map(|mv| mv.key.as_str()).collect();
    assert_eq!(pending, expected);
}

proptest! {
    #[test]
    fn random_moves_keep_idempotency_and_undo(steps in prop::collection::vec(arb_step(), 0..40)) {
        let mut queue = Queue::new(WINDOW);
        let mut model = Model {
            location: HashMap::new(),
            pending: Vec::new(),
            keys: HashMap::new(),
            last: None,
        };
        for step in steps {
            match step {
                Step::Move { key, thread, from, to } => enqueue(
                    &mut queue,
                    &mut model,
                    Move {
                        key: name('k', key),
                        thread: name('t', thread),
                        from: name('m', from),
                        to: name('m', to),
                    },
                ),
                Step::Repeat => {
                    if let Some(mv) = model.last.clone() {
                        enqueue(&mut queue, &mut model, mv);
                    }
                }
                Step::Undo { expired } => undo(&mut queue, &mut model, expired),
            }
            agree(&queue, &model);
        }
    }
}
