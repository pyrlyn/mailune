//! Change feed (S7): what other connections to the same file changed, as typed topics.
//!
//! `PRAGMA data_version` moves only when another connection commits, so a poll with nothing new
//! costs one pragma. When it moves, the per-topic counters that the migration's triggers bump are
//! read and compared with the last ones seen. This connection's own writes bump the counters too
//! but not the pragma, so they surface with the next foreign commit: an extra invalidation, never
//! a missed one.

use std::collections::{BTreeMap, BTreeSet};

use diesel::prelude::*;

use crate::open::{data_version, database_error};
use crate::schema::change_counters;
use crate::{Error, Store};

/// A group of tables a reader may cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Topic {
    /// Accounts.
    Accounts,
    /// Mailboxes.
    Mailboxes,
    /// Thread rows: subjects, counts and newest stamps.
    Threads,
    /// Messages with their mailboxes, parts and keywords.
    Messages,
    /// Embedding vectors.
    Embeddings,
    /// Sync cursors.
    SyncState,
    /// The pending operation queue.
    Ops,
    /// Contacts.
    Contacts,
}

impl Topic {
    fn from_column(name: &str) -> Option<Self> {
        Some(match name {
            "accounts" => Self::Accounts,
            "mailboxes" => Self::Mailboxes,
            "threads" => Self::Threads,
            "messages" => Self::Messages,
            "embeddings" => Self::Embeddings,
            "sync_state" => Self::SyncState,
            "ops" => Self::Ops,
            "contacts" => Self::Contacts,
            _ => return None,
        })
    }
}

/// Topics another connection changed since the previous [`Store::poll_changes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalidation {
    /// Changed topics, in [`Topic`] order, without repeats. Never empty.
    pub topics: Vec<Topic>,
}

impl Invalidation {
    /// Whether `topic` changed.
    #[must_use]
    pub fn contains(&self, topic: Topic) -> bool {
        self.topics.contains(&topic)
    }
}

/// What this connection has seen, and the topics not yet handed to the caller.
#[derive(Debug)]
pub(crate) struct ChangeFeed {
    data_version: i64,
    counters: BTreeMap<Topic, i64>,
    pending: BTreeSet<Topic>,
}

impl ChangeFeed {
    /// Starts from the file as it is now: nothing before this point is reported.
    pub(crate) fn start(conn: &mut SqliteConnection) -> Result<Self, Error> {
        Ok(Self {
            data_version: data_version(conn)?,
            counters: counters(conn)?,
            pending: BTreeSet::new(),
        })
    }

    /// Topics changed by other connections since the last call. They are also kept for
    /// [`Store::poll_changes`], so an internal check does not hide them from the caller.
    fn observe(&mut self, conn: &mut SqliteConnection) -> Result<BTreeSet<Topic>, Error> {
        // The pragma is read before the counters: a commit landing in between is then seen in
        // the counters now and only moves the pragma for the next call, which finds no change.
        let version = data_version(conn)?;
        if version == self.data_version {
            return Ok(BTreeSet::new());
        }
        let now = counters(conn)?;
        let changed: BTreeSet<Topic> = now
            .iter()
            .filter(|(topic, value)| self.counters.get(topic) != Some(value))
            .map(|(topic, _)| *topic)
            .collect();
        self.data_version = version;
        self.counters = now;
        self.pending.extend(&changed);
        Ok(changed)
    }
}

fn counters(conn: &mut SqliteConnection) -> Result<BTreeMap<Topic, i64>, Error> {
    let rows: Vec<(String, i64)> = change_counters::table
        .select((change_counters::topic, change_counters::version))
        .load(conn)
        .map_err(database_error)?;
    Ok(rows
        .into_iter()
        .filter_map(|(name, version)| Topic::from_column(&name).map(|topic| (topic, version)))
        .collect())
}

impl Store {
    /// What other connections to this file changed since the previous call, or `None`.
    ///
    /// Cheap when nothing changed: one pragma read. This connection's own writes are reported
    /// only together with the next write from elsewhere.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the feed cannot be read.
    pub fn poll_changes(&mut self) -> Result<Option<Invalidation>, Error> {
        self.observe_changes()?;
        let topics: Vec<Topic> = std::mem::take(&mut self.feed.pending).into_iter().collect();
        Ok((!topics.is_empty()).then_some(Invalidation { topics }))
    }

    /// Applies foreign changes to this connection's caches.
    pub(crate) fn observe_changes(&mut self) -> Result<(), Error> {
        let changed = self.feed.observe(&mut self.conn)?;
        if changed.contains(&Topic::Embeddings) {
            self.vectors.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::MessageId;

    use super::{Invalidation, Topic};
    use crate::Store;
    use crate::testutil::{account, message, seeded};
    use crate::vector::ChunkRef;

    fn reopen(dir: &tempfile::TempDir) -> Store {
        Store::open(&dir.path().join("mail.db"), None).unwrap()
    }

    #[test]
    fn a_second_connection_sees_a_write_as_a_typed_invalidation() {
        let (dir, mut writer) = seeded();
        let mut reader = reopen(&dir);
        assert_eq!(reader.poll_changes().unwrap(), None);

        writer
            .upsert_message(&message("m1", "t1", 1, false))
            .unwrap();
        assert_eq!(
            reader.poll_changes().unwrap(),
            Some(Invalidation {
                topics: vec![Topic::Threads, Topic::Messages]
            })
        );
        assert_eq!(reader.poll_changes().unwrap(), None);

        writer
            .set_sync_state(&account(), "jmap:Email", "s1")
            .unwrap();
        let changes = reader.poll_changes().unwrap().unwrap();
        assert!(changes.contains(Topic::SyncState));
        assert!(!changes.contains(Topic::Messages));
    }

    #[test]
    fn a_connections_own_writes_are_not_reported_alone() {
        let (dir, mut store) = seeded();
        store
            .upsert_message(&message("m1", "t1", 1, false))
            .unwrap();
        assert_eq!(store.poll_changes().unwrap(), None);

        // They ride along with the next foreign commit rather than being lost.
        let mut other = reopen(&dir);
        other.set_sync_state(&account(), "scope", "s").unwrap();
        let changes = store.poll_changes().unwrap().unwrap();
        assert!(changes.contains(Topic::Messages) && changes.contains(Topic::SyncState));
    }

    #[test]
    fn changes_before_open_are_not_reported() {
        let (dir, mut store) = seeded();
        store
            .upsert_message(&message("m1", "t1", 1, false))
            .unwrap();
        drop(store);
        assert_eq!(reopen(&dir).poll_changes().unwrap(), None);
    }

    #[test]
    fn the_vector_cache_drops_on_embedding_changes_only() {
        let (dir, mut store) = seeded();
        store
            .upsert_message(&message("m1", "t1", 1, false))
            .unwrap();
        let chunk = ChunkRef {
            message: MessageId::new("m1"),
            chunk: 0,
        };
        store
            .put_embedding(&account(), &chunk, "a", &[1.0, 0.0])
            .unwrap();
        store.nearest(&account(), "a", &[1.0, 0.0], 1).unwrap();
        assert!(store.vectors.bytes() > 0);
        // Clear the own-write residue so the next poll sees only the foreign commit.
        let mut other = reopen(&dir);
        other.set_sync_state(&account(), "scope", "s").unwrap();
        store.poll_changes().unwrap();
        store.nearest(&account(), "a", &[1.0, 0.0], 1).unwrap();

        other
            .upsert_message(&message("m2", "t2", 2, false))
            .unwrap();
        assert!(store.poll_changes().unwrap().is_some());
        assert!(
            store.vectors.bytes() > 0,
            "a message write keeps the vectors"
        );

        other
            .put_embedding(&account(), &chunk, "a", &[0.0, 1.0])
            .unwrap();
        // The search drops the stale vectors, and the caller still gets the invalidation.
        let hits = store.nearest(&account(), "a", &[0.0, 1.0], 1).unwrap();
        assert!((hits[0].score - 1.0).abs() < 1e-6);
        assert_eq!(
            store.poll_changes().unwrap(),
            Some(Invalidation {
                topics: vec![Topic::Embeddings]
            })
        );
    }
}
