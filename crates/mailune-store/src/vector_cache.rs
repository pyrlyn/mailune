//! Decoded embedding vectors kept in memory, so [`crate::Store::nearest`] scans memory instead of
//! loading and decrypting every row on every query (S14: that load was 1.36 s of 1.44 s at
//! 100k messages).
//!
//! The cache lives only in the process: nothing here is written to disk, and vector buffers are
//! zeroized when they are dropped or outgrown, because an embedding can leak mail content.
//! It is bounded by [`MAX_CACHED_BYTES`]; a set that would not fit is scanned once and dropped.
//!
//! Freshness: this connection's own writes update the cache in place (`record_write`). When the
//! change feed (`feed`) reports that another connection or process changed embeddings, the whole
//! cache is dropped; writes to other topics leave it alone.

use std::cmp::Ordering;
use std::collections::HashMap;

use mailune_protocol::MessageId;
use zeroize::Zeroize;

use crate::vector::{ChunkRef, Neighbour, norm};

/// Vector bytes the cache may hold across all sets: 256 MiB. One 384-dimension chunk per
/// message at 100k messages is about 154 MB.
pub(crate) const MAX_CACHED_BYTES: usize = 256 * 1024 * 1024;

const F32_BYTES: usize = std::mem::size_of::<f32>();

/// Rows of one account and model with one dimension: what one `nearest` call scans.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SetKey {
    pub(crate) account: String,
    pub(crate) model: String,
    pub(crate) dim: usize,
}

/// One set's vectors in the order the database returned the rows, which keeps the order of
/// equal scores the same as the uncached scan's.
#[derive(Debug)]
pub(crate) struct VectorSet {
    dim: usize,
    chunks: Vec<ChunkRef>,
    /// `None` for a row the scan skips: a zero vector, or a blob of the wrong length.
    norms: Vec<Option<f32>>,
    /// `chunks.len() * dim` values, row after row.
    values: Vec<f32>,
    index: HashMap<ChunkRef, usize>,
}

impl VectorSet {
    /// Decodes `(message, chunk, little-endian f32 bytes)` rows of dimension `dim`.
    pub(crate) fn from_rows(dim: usize, rows: Vec<(String, i32, Vec<u8>)>) -> Self {
        let mut set = Self {
            dim,
            chunks: Vec::with_capacity(rows.len()),
            norms: Vec::with_capacity(rows.len()),
            // Exact capacity: filling it never reallocates and leaves no unzeroed copy behind.
            values: Vec::with_capacity(rows.len().saturating_mul(dim)),
            index: HashMap::with_capacity(rows.len()),
        };
        for (message, chunk, mut bytes) in rows {
            let chunk = ChunkRef {
                message: MessageId::new(message),
                chunk: u32::try_from(chunk).unwrap_or(0),
            };
            let start = set.values.len();
            let (raw, rest) = bytes.as_chunks::<4>();
            let norm = if rest.is_empty() && raw.len() == dim {
                set.values
                    .extend(raw.iter().map(|raw| f32::from_le_bytes(*raw)));
                set.values.get(start..).and_then(nonzero_norm)
            } else {
                set.values.resize(start + dim, 0.0);
                None
            };
            bytes.zeroize();
            set.finish_row(chunk, norm);
        }
        set
    }

    fn bytes(&self) -> usize {
        self.values.capacity().saturating_mul(F32_BYTES)
    }

    fn push(&mut self, chunk: ChunkRef, vector: &[f32]) {
        if self.values.len() + self.dim > self.values.capacity() {
            let wanted = (self.values.capacity() * 2).max(self.values.len() + self.dim);
            let mut grown = Vec::with_capacity(wanted);
            grown.extend_from_slice(&self.values);
            // A plain push would reallocate and free the old buffer without wiping it.
            self.values.zeroize();
            self.values = grown;
        }
        self.values.extend_from_slice(vector);
        self.finish_row(chunk, nonzero_norm(vector));
    }

    fn finish_row(&mut self, chunk: ChunkRef, norm: Option<f32>) {
        self.norms.push(norm);
        self.index.insert(chunk.clone(), self.chunks.len());
        self.chunks.push(chunk);
    }

    fn replace(&mut self, row: usize, vector: &[f32]) {
        let start = row * self.dim;
        if let Some(slot) = self.values.get_mut(start..start + self.dim) {
            slot.copy_from_slice(vector);
        }
        if let Some(norm) = self.norms.get_mut(row) {
            *norm = nonzero_norm(vector);
        }
    }

    /// The `limit` rows closest to `query` by cosine, best first; ties by message id, then chunk.
    ///
    /// The arithmetic and the sort are the uncached scan's, operation for operation, so the
    /// scores and their order are identical.
    pub(crate) fn nearest(&self, query: &[f32], query_norm: f32, limit: usize) -> Vec<Neighbour> {
        if query.len() != self.dim || self.dim == 0 {
            return Vec::new();
        }
        let mut scored: Vec<(usize, f32)> = self
            .values
            .chunks_exact(self.dim)
            .zip(&self.norms)
            .enumerate()
            .filter_map(|(row, (stored, stored_norm))| {
                let stored_norm = (*stored_norm)?;
                let dot: f32 = stored
                    .iter()
                    .zip(query)
                    .map(|(left, right)| left * right)
                    .sum();
                Some((row, dot / (stored_norm * query_norm)))
            })
            .collect();
        scored.sort_by(|(left_row, left), (right_row, right)| {
            let (left_chunk, right_chunk) = (&self.chunks[*left_row], &self.chunks[*right_row]);
            right
                .partial_cmp(left)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    left_chunk
                        .message
                        .as_str()
                        .cmp(right_chunk.message.as_str())
                })
                .then_with(|| left_chunk.chunk.cmp(&right_chunk.chunk))
        });
        scored.truncate(limit);
        scored
            .into_iter()
            .map(|(row, score)| Neighbour {
                chunk: self.chunks[row].clone(),
                score,
            })
            .collect()
    }
}

impl Drop for VectorSet {
    fn drop(&mut self) {
        self.values.zeroize();
    }
}

/// Every cached set.
#[derive(Debug)]
pub(crate) struct VectorCache {
    sets: HashMap<SetKey, VectorSet>,
    limit: usize,
}

impl Default for VectorCache {
    fn default() -> Self {
        Self::with_limit(MAX_CACHED_BYTES)
    }
}

impl VectorCache {
    pub(crate) fn with_limit(limit: usize) -> Self {
        Self {
            sets: HashMap::new(),
            limit,
        }
    }

    /// Drops every set; their vectors are zeroized.
    pub(crate) fn clear(&mut self) {
        self.sets.clear();
    }

    /// Bytes of vector data held now.
    pub(crate) fn bytes(&self) -> usize {
        self.sets.values().map(VectorSet::bytes).sum()
    }

    pub(crate) fn get(&self, key: &SetKey) -> Option<&VectorSet> {
        self.sets.get(key)
    }

    /// Whether `set` fits under the limit ([`MAX_CACHED_BYTES`] unless a test lowers it). A set
    /// that does not is scanned once and dropped.
    pub(crate) fn fits(&self, set: &VectorSet) -> bool {
        self.bytes().saturating_add(set.bytes()) <= self.limit
    }

    pub(crate) fn insert(&mut self, key: SetKey, set: VectorSet) -> &VectorSet {
        self.sets.entry(key).or_insert(set)
    }

    /// Whether any set of `account` and `model` is cached, so a write must update it.
    pub(crate) fn holds(&self, account: &str, model: &str) -> bool {
        self.sets
            .keys()
            .any(|key| key.account == account && key.model == model)
    }

    /// Applies this connection's own committed write of `vector` to `chunk`; `old_dim` is the
    /// dimension the row had before, `None` for a new row.
    pub(crate) fn record_write(
        &mut self,
        account: &str,
        model: &str,
        chunk: &ChunkRef,
        old_dim: Option<usize>,
        vector: &[f32],
    ) {
        let key = |dim| SetKey {
            account: account.to_owned(),
            model: model.to_owned(),
            dim,
        };
        let new_key = key(vector.len());
        match old_dim {
            // A row that changes dimension keeps its place in the table, which an append to
            // the other set would not; both sets reload instead.
            Some(old) if old != vector.len() => {
                self.sets.remove(&key(old));
                self.sets.remove(&new_key);
            }
            Some(_) => {
                let Some(set) = self.sets.get_mut(&new_key) else {
                    return;
                };
                match set.index.get(chunk).copied() {
                    Some(row) => set.replace(row, vector),
                    None => {
                        self.sets.remove(&new_key);
                    }
                }
            }
            None => {
                let Some(set) = self.sets.get_mut(&new_key) else {
                    return;
                };
                set.push(chunk.clone(), vector);
                if self.bytes() > self.limit {
                    self.sets.remove(&new_key);
                }
            }
        }
    }
}

fn nonzero_norm(vector: &[f32]) -> Option<f32> {
    let value = norm(vector.iter().copied());
    (value != 0.0).then_some(value)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use diesel::prelude::*;
    use mailune_protocol::MessageId;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    use super::VectorCache;
    use crate::Store;
    use crate::schema::embeddings;
    use crate::testutil::{account, message, seeded};
    use crate::vector::{ChunkRef, Neighbour, norm};

    const DIM: usize = 8;
    const MESSAGES: usize = 120;
    const TOP: usize = 15;

    /// S8's scan as it was before the cache: load the rows, score each, sort, truncate.
    fn reference(store: &mut Store, model: &str, query: &[f32], limit: usize) -> Vec<Neighbour> {
        let query_norm = norm(query.iter().copied());
        let rows: Vec<(String, i32, Vec<u8>)> = embeddings::table
            .filter(embeddings::account_id.eq(account().as_str()))
            .filter(embeddings::model.eq(model))
            .filter(embeddings::dim.eq(i32::try_from(query.len()).unwrap()))
            .select((
                embeddings::message_id,
                embeddings::chunk,
                embeddings::vector,
            ))
            .load(&mut store.conn)
            .unwrap();
        let mut scored: Vec<Neighbour> = rows
            .into_iter()
            .filter_map(|(message, chunk, bytes)| {
                let (raw, rest) = bytes.as_chunks::<4>();
                if !rest.is_empty() || raw.len() != query.len() {
                    return None;
                }
                let stored = || raw.iter().map(|raw| f32::from_le_bytes(*raw));
                let stored_norm = norm(stored());
                if stored_norm == 0.0 {
                    return None;
                }
                let dot: f32 = stored().zip(query).map(|(left, right)| left * right).sum();
                Some(Neighbour {
                    chunk: ChunkRef {
                        message: MessageId::new(message),
                        chunk: u32::try_from(chunk).unwrap(),
                    },
                    score: dot / (stored_norm * query_norm),
                })
            })
            .collect();
        scored.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    left.chunk
                        .message
                        .as_str()
                        .cmp(right.chunk.message.as_str())
                })
                .then_with(|| left.chunk.chunk.cmp(&right.chunk.chunk))
        });
        scored.truncate(limit);
        scored
    }

    fn chunk(id: &str, index: u32) -> ChunkRef {
        ChunkRef {
            message: MessageId::new(id),
            chunk: index,
        }
    }

    fn random(rng: &mut StdRng, dim: usize) -> Vec<f32> {
        (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
    }

    /// Messages with two chunks each under model "a", plus rows the scan must skip or tie on.
    fn fixture(store: &mut Store, rng: &mut StdRng) {
        let acc = account();
        for index in 0..MESSAGES {
            let id = format!("m{index:03}");
            store.upsert_message(&message(&id, &id, 1, false)).unwrap();
            for part in 0..2 {
                store
                    .put_embedding(&acc, &chunk(&id, part), "a", &random(rng, DIM))
                    .unwrap();
            }
        }
        // A zero vector, another dimension, another model, and two equal vectors (a tie).
        store
            .put_embedding(&acc, &chunk("m000", 2), "a", &[0.0; DIM])
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m001", 2), "a", &random(rng, DIM + 1))
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m002", 2), "b", &random(rng, DIM))
            .unwrap();
        let twin = random(rng, DIM);
        store
            .put_embedding(&acc, &chunk("m003", 2), "a", &twin)
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m004", 2), "a", &twin)
            .unwrap();
    }

    fn assert_matches_reference(store: &mut Store, query: &[f32]) {
        let expected = reference(store, "a", query, TOP);
        let cached = store.nearest(&account(), "a", query, TOP).unwrap();
        assert_eq!(cached, expected);
    }

    #[test]
    fn cached_results_are_identical_to_the_uncached_scan() {
        let (_dir, mut store) = seeded();
        let mut rng = StdRng::seed_from_u64(14);
        fixture(&mut store, &mut rng);
        let queries: Vec<Vec<f32>> = (0..5).map(|_| random(&mut rng, DIM)).collect();
        for query in &queries {
            assert_matches_reference(&mut store, query);
        }
        assert!(store.vectors.bytes() > 0, "the first query fills the cache");
        // Second round is served from memory.
        for query in &queries {
            assert_matches_reference(&mut store, query);
        }
        let twin_query = reference(&mut store, "a", &queries[0], usize::MAX);
        assert!(twin_query.iter().any(|hit| hit.chunk == chunk("m003", 2)));
    }

    #[test]
    fn this_stores_writes_keep_the_cache_in_step() {
        let (_dir, mut store) = seeded();
        let mut rng = StdRng::seed_from_u64(15);
        fixture(&mut store, &mut rng);
        let query = random(&mut rng, DIM);
        assert_matches_reference(&mut store, &query);
        let acc = account();

        // In place: the same row gets the query itself, so it must come first.
        store
            .put_embedding(&acc, &chunk("m050", 0), "a", &query)
            .unwrap();
        assert_matches_reference(&mut store, &query);
        assert_eq!(
            store.nearest(&acc, "a", &query, 1).unwrap()[0].chunk,
            chunk("m050", 0)
        );

        // A new row is appended.
        store
            .upsert_message(&message("new", "new", 1, false))
            .unwrap();
        store
            .put_embedding(&acc, &chunk("new", 0), "a", &query)
            .unwrap();
        assert_matches_reference(&mut store, &query);

        // A row that changes dimension leaves this set and joins the other.
        store
            .put_embedding(&acc, &chunk("m050", 0), "a", &random(&mut rng, DIM + 1))
            .unwrap();
        assert_matches_reference(&mut store, &query);
        let wide = random(&mut rng, DIM + 1);
        assert_eq!(
            store.nearest(&acc, "a", &wide, TOP).unwrap(),
            reference(&mut store, "a", &wide, TOP)
        );
    }

    #[test]
    fn a_write_from_another_connection_drops_the_cache() {
        let (dir, mut store) = seeded();
        let mut rng = StdRng::seed_from_u64(16);
        fixture(&mut store, &mut rng);
        let query = random(&mut rng, DIM);
        assert_matches_reference(&mut store, &query);

        let mut other = Store::open(&dir.path().join("mail.db"), None).unwrap();
        other
            .put_embedding(&account(), &chunk("m010", 0), "a", &query)
            .unwrap();
        drop(other);

        assert_matches_reference(&mut store, &query);
        assert_eq!(
            store.nearest(&account(), "a", &query, 1).unwrap()[0].chunk,
            chunk("m010", 0)
        );
    }

    #[test]
    fn a_set_over_the_limit_is_scanned_but_not_kept() {
        let (_dir, mut store) = seeded();
        let mut rng = StdRng::seed_from_u64(17);
        fixture(&mut store, &mut rng);
        store.vectors = VectorCache::with_limit(DIM * std::mem::size_of::<f32>());
        let query = random(&mut rng, DIM);
        assert_matches_reference(&mut store, &query);
        assert_eq!(store.vectors.bytes(), 0);
    }

    #[test]
    fn clearing_the_cache_frees_it_and_the_next_query_reloads() {
        let (_dir, mut store) = seeded();
        let mut rng = StdRng::seed_from_u64(18);
        fixture(&mut store, &mut rng);
        let query = random(&mut rng, DIM);
        assert_matches_reference(&mut store, &query);
        store.clear_vector_cache();
        assert_eq!(store.vectors.bytes(), 0);
        assert_matches_reference(&mut store, &query);
        assert!(store.vectors.bytes() > 0);
    }
}
