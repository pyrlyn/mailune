//! Persisted operation queue.
//!
//! The rules (idempotency, undo, conflicts) stay in `mailune_core::Queue`.
//! This file only saves its pending set and reads it back in queue order so
//! the caller can replay it through `Queue::enqueue` after a restart.

use std::time::{Duration, SystemTime};

use diesel::prelude::*;
use mailune_core::{IdempotencyKey, Op};
use mailune_protocol::{MailboxId, ThreadId};

use crate::open::database_error;
use crate::schema::ops;
use crate::{Error, Store};

/// One pending op as the queue holds it: key, change, and when it was asked for.
pub type PendingOp = (IdempotencyKey, Op, SystemTime);

#[derive(Insertable)]
#[diesel(table_name = ops)]
struct OpRow<'a> {
    op_key: &'a str,
    kind: &'static str,
    thread_id: &'a str,
    from_mailbox: Option<&'a str>,
    to_mailbox: Option<&'a str>,
    wake_ns: Option<i64>,
    queued_ns: i64,
}

impl Store {
    /// Replaces the stored queue with `pending`, in the order given.
    ///
    /// Pass `Queue::pending_ops()`. Acknowledged and undone ops are simply
    /// absent from the next save.
    ///
    /// # Errors
    ///
    /// [`Error::OpTime`] for a time before the epoch or past the year 2262,
    /// [`Error::Database`] when the write fails.
    pub fn save_ops(
        &mut self,
        pending: &[(&IdempotencyKey, &Op, SystemTime)],
    ) -> Result<(), Error> {
        let mut rows = Vec::with_capacity(pending.len());
        for (key, op, at) in pending {
            rows.push(op_row(key, op, *at)?);
        }
        self.conn
            .transaction(|conn| {
                diesel::delete(ops::table).execute(conn)?;
                for row in &rows {
                    diesel::insert_into(ops::table).values(row).execute(conn)?;
                }
                Ok(())
            })
            .map_err(database_error)
    }

    /// Stored ops, oldest first, ready to replay.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails or a row has an unknown kind.
    pub fn load_ops(&mut self) -> Result<Vec<PendingOp>, Error> {
        type Row = (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            i64,
        );
        let rows: Vec<Row> = ops::table
            .order(ops::seq.asc())
            .select((
                ops::op_key,
                ops::kind,
                ops::thread_id,
                ops::from_mailbox,
                ops::to_mailbox,
                ops::wake_ns,
                ops::queued_ns,
            ))
            .load(&mut self.conn)
            .map_err(database_error)?;
        rows.into_iter()
            .map(|(key, kind, thread, from, to, wake, queued)| {
                let thread = ThreadId::new(thread);
                let wake = || wake.map(time_from).ok_or_else(|| bad_row(&kind));
                let op = match kind.as_str() {
                    "move" => Op::Move {
                        thread,
                        from: MailboxId::new(from.ok_or_else(|| bad_row(&kind))?),
                        to: MailboxId::new(to.ok_or_else(|| bad_row(&kind))?),
                    },
                    "snooze" => Op::Snooze {
                        thread,
                        until: wake()?,
                    },
                    "reminder" => Op::Reminder {
                        thread,
                        at: wake()?,
                    },
                    "reply_later" => Op::ReplyLater {
                        thread,
                        at: wake()?,
                    },
                    _ => return Err(bad_row(&kind)),
                };
                Ok((IdempotencyKey::new(key), op, time_from(queued)))
            })
            .collect()
    }
}

fn op_row<'a>(key: &'a IdempotencyKey, op: &'a Op, at: SystemTime) -> Result<OpRow<'a>, Error> {
    let queued_ns = nanos(at)?;
    let row = |kind, thread: &'a ThreadId, wake: Option<SystemTime>| -> Result<OpRow<'a>, Error> {
        Ok(OpRow {
            op_key: key.as_str(),
            kind,
            thread_id: thread.as_str(),
            from_mailbox: None,
            to_mailbox: None,
            wake_ns: wake.map(nanos).transpose()?,
            queued_ns,
        })
    };
    match op {
        Op::Move { thread, from, to } => Ok(OpRow {
            from_mailbox: Some(from.as_str()),
            to_mailbox: Some(to.as_str()),
            ..row("move", thread, None)?
        }),
        Op::Snooze { thread, until } => row("snooze", thread, Some(*until)),
        Op::Reminder { thread, at } => row("reminder", thread, Some(*at)),
        Op::ReplyLater { thread, at } => row("reply_later", thread, Some(*at)),
    }
}

fn nanos(at: SystemTime) -> Result<i64, Error> {
    let since = at
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| Error::OpTime)?;
    i64::try_from(since.as_nanos()).map_err(|_| Error::OpTime)
}

fn time_from(nanos: i64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_nanos(u64::try_from(nanos).unwrap_or(0))
}

fn bad_row(kind: &str) -> Error {
    Error::Database(format!(
        "stored op of kind {kind:?} is incomplete or unknown"
    ))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use mailune_core::{IdempotencyKey, Op, Queue, When};
    use mailune_protocol::{MailboxId, ThreadId};

    use crate::{Error, Store};

    fn queue_with_ops(now: SystemTime) -> Queue {
        let mut queue = Queue::new(Duration::from_secs(5));
        let thread = ThreadId::new("t1");
        let moves = [("k1", "inbox", "archive"), ("k2", "archive", "trash")];
        for (key, from, to) in moves {
            queue
                .enqueue(
                    IdempotencyKey::new(key),
                    Op::Move {
                        thread: thread.clone(),
                        from: MailboxId::new(from),
                        to: MailboxId::new(to),
                    },
                    now,
                )
                .unwrap();
        }
        queue
            .enqueue(
                IdempotencyKey::new("k3"),
                Op::Snooze {
                    thread: ThreadId::new("t2"),
                    // Sub-microsecond part: the round trip must keep it.
                    until: now + Duration::new(3600, 123),
                },
                now + Duration::from_secs(1),
            )
            .unwrap();
        queue.ack(&IdempotencyKey::new("k1")).unwrap();
        queue
    }

    #[test]
    fn pending_ops_survive_a_reopen_and_replay_onto_the_queue() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let before = queue_with_ops(now);
        let mut store = Store::open(&path, None).unwrap();
        store.save_ops(&before.pending_ops()).unwrap();
        drop(store);

        let loaded = Store::open(&path, None).unwrap().load_ops().unwrap();
        let mut after = Queue::new(Duration::from_secs(5));
        for (key, op, at) in &loaded {
            after.enqueue(key.clone(), op.clone(), *at).unwrap();
        }
        let keys = |queue: &Queue| {
            queue
                .pending()
                .into_iter()
                .map(|key| key.as_str().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(&after), ["k2", "k3"]);
        assert_eq!(keys(&after), keys(&before));
        assert_eq!(
            after.location(&ThreadId::new("t1")),
            Some(&MailboxId::new("trash"))
        );
        assert_eq!(
            after.schedule(&ThreadId::new("t2")),
            Some((When::Snooze, now + Duration::new(3600, 123)))
        );
        // Replaying the same rows again is a no-op: the keys are idempotent.
        for (key, op, at) in loaded {
            after.enqueue(key, op, at).unwrap();
        }
        assert_eq!(keys(&after), ["k2", "k3"]);
        // The undo window is measured from the stored queue time.
        after.undo(now + Duration::from_secs(6)).unwrap();
        assert_eq!(keys(&after), ["k2"]);
    }

    #[test]
    fn a_save_replaces_the_previous_set() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        let mut queue = queue_with_ops(now);
        store.save_ops(&queue.pending_ops()).unwrap();
        queue.ack(&IdempotencyKey::new("k2")).unwrap();
        queue.ack(&IdempotencyKey::new("k3")).unwrap();
        store.save_ops(&queue.pending_ops()).unwrap();
        assert!(store.load_ops().unwrap().is_empty());
    }

    #[test]
    fn a_time_before_the_epoch_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        let key = IdempotencyKey::new("k");
        let op = Op::Reminder {
            thread: ThreadId::new("t"),
            at: SystemTime::UNIX_EPOCH,
        };
        let early = SystemTime::UNIX_EPOCH - Duration::from_secs(1);
        assert!(matches!(
            store.save_ops(&[(&key, &op, early)]),
            Err(Error::OpTime)
        ));
    }
}
