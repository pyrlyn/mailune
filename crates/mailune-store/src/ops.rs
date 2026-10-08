//! Load and save the pending operation queue.
//!
//! The state machine stays in `mailune-core`. Rows are a snapshot of
//! [`mailune_core::Pending`], oldest first, so a caller can replay them with
//! `enqueue`. Ids are single lines; a newline cannot be stored unambiguously.

use std::time::{Duration, SystemTime};

use diesel::prelude::*;
use mailune_core::{IdempotencyKey, Op, Pending};
use mailune_protocol::{MailboxId, ThreadId};

use crate::schema::ops;
use crate::{Error, Store};

#[derive(Insertable)]
#[diesel(table_name = ops)]
struct NewOp {
    key: String,
    kind: String,
    payload: String,
    at_ms: i64,
    position: i32,
}

impl Store {
    /// Replaces the saved pending set with `ops`, oldest first.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when a write fails or an id contains a newline.
    pub fn save_pending(&mut self, pending: &[Pending]) -> Result<(), Error> {
        let rows = pending
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let (kind, payload) = encode(&item.op)?;
                Ok(NewOp {
                    key: item.key.as_str().to_string(),
                    kind,
                    payload,
                    at_ms: millis(item.at)?,
                    position: i32::try_from(index).map_err(|_| Error::Query)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.conn.transaction::<(), Error, _>(|conn| {
            diesel::delete(ops::table).execute(conn)?;
            for row in &rows {
                diesel::insert_into(ops::table).values(row).execute(conn)?;
            }
            Ok(())
        })
    }

    /// Pending operations in the order they were saved.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when a row cannot be read or decoded.
    pub fn load_pending(&mut self) -> Result<Vec<Pending>, Error> {
        let rows = ops::table
            .select((ops::key, ops::kind, ops::payload, ops::at_ms))
            .order(ops::position.asc())
            .load::<(String, String, String, i64)>(&mut self.conn)?;
        rows.iter()
            .map(|(key, kind, payload, at_ms)| decode(kind, payload, key, *at_ms))
            .collect()
    }
}

fn encode(op: &Op) -> Result<(String, String), Error> {
    match op {
        Op::Move { thread, from, to } => {
            plain(thread.as_str())?;
            plain(from.as_str())?;
            plain(to.as_str())?;
            Ok((
                "move".into(),
                format!("{}\n{}\n{}", thread.as_str(), from.as_str(), to.as_str()),
            ))
        }
        Op::Snooze { thread, until } => timed("snooze", thread.as_str(), *until),
        Op::Reminder { thread, at } => timed("reminder", thread.as_str(), *at),
        Op::ReplyLater { thread, at } => timed("reply_later", thread.as_str(), *at),
    }
}

fn timed(kind: &str, thread: &str, when: SystemTime) -> Result<(String, String), Error> {
    plain(thread)?;
    Ok((kind.to_string(), format!("{thread}\n{}", millis(when)?)))
}

fn plain(value: &str) -> Result<(), Error> {
    if value.contains('\n') {
        Err(Error::Query)
    } else {
        Ok(())
    }
}

fn decode(kind: &str, payload: &str, key: &str, at_ms: i64) -> Result<Pending, Error> {
    let at = from_millis(at_ms)?;
    let op = match kind {
        "move" => {
            let mut lines = payload.split('\n');
            let thread = lines.next().ok_or(Error::Query)?;
            let from = lines.next().ok_or(Error::Query)?;
            let to = lines.next().ok_or(Error::Query)?;
            if lines.next().is_some() {
                return Err(Error::Query);
            }
            Op::Move {
                thread: ThreadId::new(thread),
                from: MailboxId::new(from),
                to: MailboxId::new(to),
            }
        }
        "snooze" => {
            let (thread, until) = wake(payload)?;
            Op::Snooze { thread, until }
        }
        "reminder" => {
            let (thread, at) = wake(payload)?;
            Op::Reminder { thread, at }
        }
        "reply_later" => {
            let (thread, at) = wake(payload)?;
            Op::ReplyLater { thread, at }
        }
        _ => return Err(Error::Query),
    };
    Ok(Pending {
        key: IdempotencyKey::new(key),
        op,
        at,
    })
}

fn wake(payload: &str) -> Result<(ThreadId, SystemTime), Error> {
    let mut lines = payload.split('\n');
    let thread = lines.next().ok_or(Error::Query)?;
    let ms = lines.next().ok_or(Error::Query)?;
    if lines.next().is_some() {
        return Err(Error::Query);
    }
    let ms = ms.parse().map_err(|_| Error::Query)?;
    Ok((ThreadId::new(thread), from_millis(ms)?))
}

fn millis(time: SystemTime) -> Result<i64, Error> {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| Error::Query)?;
    i64::try_from(duration.as_millis()).map_err(|_| Error::Query)
}

fn from_millis(ms: i64) -> Result<SystemTime, Error> {
    let ms = u64::try_from(ms).map_err(|_| Error::Query)?;
    Ok(SystemTime::UNIX_EPOCH + Duration::from_millis(ms))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use mailune_core::{IdempotencyKey, Op, Queue};
    use mailune_protocol::{MailboxId, ThreadId};

    use crate::{Store, scratch_dir};

    #[test]
    fn pending_ops_replay_onto_the_queue() {
        let now = SystemTime::UNIX_EPOCH;
        let wake = now + Duration::from_secs(60);
        let thread = ThreadId::new("t");
        let mut queue = Queue::new(Duration::from_secs(5));
        queue
            .enqueue(
                IdempotencyKey::new("move"),
                Op::Move {
                    thread: thread.clone(),
                    from: MailboxId::new("inbox"),
                    to: MailboxId::new("archive"),
                },
                now,
            )
            .unwrap();
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

        let dir = scratch_dir();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        store.save_pending(&queue.pending_ops()).unwrap();
        let loaded = store.load_pending().unwrap();
        let mut replay = Queue::new(Duration::from_secs(5));
        for item in loaded {
            replay.enqueue(item.key, item.op, item.at).unwrap();
        }
        assert_eq!(replay.pending_ops(), queue.pending_ops());
        assert_eq!(replay.location(&thread), queue.location(&thread));
        replay.undo(now).unwrap();
        queue.undo(now).unwrap();
        assert_eq!(replay.pending_ops(), queue.pending_ops());
        assert_eq!(
            replay.schedule(&thread).map(|(when, _)| when),
            Some(mailune_core::When::Reminder)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
