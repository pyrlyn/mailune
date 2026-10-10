//! Embedding store: vectors per message chunk, nearest neighbours by cosine.
//!
//! The scan is exact cosine in Rust over one account's vectors for one model (S8). S14 measured
//! it against a usearch index and the creator kept the exact scan; the vectors stay decoded in
//! memory (`vector_cache`) so a query no longer reloads and decrypts every row.

use diesel::prelude::*;
use mailune_protocol::{AccountId, MessageId};

use crate::open::{data_version, database_error};
use crate::schema::embeddings;
use crate::vector_cache::{SetKey, VectorSet};
use crate::{Error, Store};

/// One chunk of one message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChunkRef {
    /// Message the chunk came from.
    pub message: MessageId,
    /// Chunk index inside that message.
    pub chunk: u32,
}

/// A stored chunk and its cosine similarity to the query.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbour {
    /// The chunk.
    pub chunk: ChunkRef,
    /// Cosine similarity in `[-1, 1]`.
    pub score: f32,
}

impl Store {
    /// Stores or replaces the vector for one chunk under `model`.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the write fails or the message is unknown.
    pub fn put_embedding(
        &mut self,
        account: &AccountId,
        chunk: &ChunkRef,
        model: &str,
        vector: &[f32],
    ) -> Result<(), Error> {
        let bytes: Vec<u8> = vector
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let dim = i32::try_from(vector.len()).unwrap_or(i32::MAX);
        let index = i32::try_from(chunk.chunk).unwrap_or(i32::MAX);
        let cached = self.vectors.holds(account.as_str(), model);
        // Only a cached set needs the row's old dimension to stay in step.
        let old_dim: Option<i32> = if cached {
            embeddings::table
                .find((account.as_str(), chunk.message.as_str(), index, model))
                .select(embeddings::dim)
                .first(&mut self.conn)
                .optional()
                .map_err(database_error)?
        } else {
            None
        };
        diesel::insert_into(embeddings::table)
            .values((
                embeddings::account_id.eq(account.as_str()),
                embeddings::message_id.eq(chunk.message.as_str()),
                embeddings::chunk.eq(index),
                embeddings::model.eq(model),
                embeddings::dim.eq(dim),
                embeddings::vector.eq(&bytes),
            ))
            .on_conflict((
                embeddings::account_id,
                embeddings::message_id,
                embeddings::chunk,
                embeddings::model,
            ))
            .do_update()
            .set((embeddings::dim.eq(dim), embeddings::vector.eq(&bytes)))
            .execute(&mut self.conn)
            .map_err(database_error)?;
        if cached {
            let old_dim = old_dim.map(|old| usize::try_from(old).unwrap_or(usize::MAX));
            self.vectors
                .record_write(account.as_str(), model, chunk, old_dim, vector);
        }
        Ok(())
    }

    /// Drops the in-memory vectors, for example when the OS reports memory pressure. The next
    /// [`Store::nearest`] loads them again.
    pub fn clear_vector_cache(&mut self) {
        self.vectors.clear();
    }

    /// The `limit` chunks closest to `query` by cosine, best first.
    ///
    /// Vectors of another dimension and zero vectors are skipped: they cannot
    /// be compared. Ties keep a stable order by message id and chunk.
    ///
    /// The first query for an account, model and dimension loads its vectors into memory (see
    /// `vector_cache`); later ones scan memory until another connection writes.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the query fails.
    pub fn nearest(
        &mut self,
        account: &AccountId,
        model: &str,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<Neighbour>, Error> {
        let query_norm = norm(query.iter().copied());
        if query_norm == 0.0 || limit == 0 {
            return Ok(Vec::new());
        }
        self.vectors.sync(data_version(&mut self.conn)?);
        let key = SetKey {
            account: account.as_str().to_owned(),
            model: model.to_owned(),
            dim: query.len(),
        };
        if let Some(set) = self.vectors.get(&key) {
            return Ok(set.nearest(query, query_norm, limit));
        }
        let dim = i32::try_from(query.len()).unwrap_or(i32::MAX);
        let rows: Vec<(String, i32, Vec<u8>)> = embeddings::table
            .filter(embeddings::account_id.eq(account.as_str()))
            .filter(embeddings::model.eq(model))
            .filter(embeddings::dim.eq(dim))
            .select((
                embeddings::message_id,
                embeddings::chunk,
                embeddings::vector,
            ))
            .load(&mut self.conn)
            .map_err(database_error)?;
        let set = VectorSet::from_rows(query.len(), rows);
        if !self.vectors.fits(&set) {
            return Ok(set.nearest(query, query_norm, limit));
        }
        Ok(self
            .vectors
            .insert(key, set)
            .nearest(query, query_norm, limit))
    }
}

pub(crate) fn norm(values: impl Iterator<Item = f32>) -> f32 {
    values.map(|value| value * value).sum::<f32>().sqrt()
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{AccountId, MessageId};

    use super::ChunkRef;
    use crate::testutil::{account, message, seeded};

    fn chunk(id: &str, index: u32) -> ChunkRef {
        ChunkRef {
            message: MessageId::new(id),
            chunk: index,
        }
    }

    #[test]
    fn nearest_orders_by_cosine_and_ignores_other_models_and_sizes() {
        let (_dir, mut store) = seeded();
        for id in ["m1", "m2", "m3"] {
            store.upsert_message(&message(id, "t1", 1, false)).unwrap();
        }
        let acc = account();
        store
            .put_embedding(&acc, &chunk("m1", 0), "a", &[1.0, 0.0])
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m2", 0), "a", &[0.6, 0.8])
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m3", 0), "a", &[0.0, 1.0])
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m3", 1), "a", &[0.0, 0.0])
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m3", 2), "a", &[1.0, 0.0, 0.0])
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m1", 1), "b", &[1.0, 0.0])
            .unwrap();

        let hits = store.nearest(&acc, "a", &[2.0, 0.1], 2).unwrap();
        let ids: Vec<(&str, u32)> = hits
            .iter()
            .map(|hit| (hit.chunk.message.as_str(), hit.chunk.chunk))
            .collect();
        assert_eq!(ids, [("m1", 0), ("m2", 0)]);
        assert!(hits[0].score > 0.99 && hits[0].score <= 1.0);
        let all = store.nearest(&acc, "a", &[2.0, 0.1], 10).unwrap();
        assert_eq!(all.len(), 3);
        assert!(
            store
                .nearest(&AccountId::new("other"), "a", &[1.0, 0.0], 5)
                .unwrap()
                .is_empty()
        );
        assert!(store.nearest(&acc, "a", &[0.0, 0.0], 5).unwrap().is_empty());
    }

    #[test]
    fn a_vector_is_replaced_in_place() {
        let (_dir, mut store) = seeded();
        store
            .upsert_message(&message("m1", "t1", 1, false))
            .unwrap();
        let acc = account();
        store
            .put_embedding(&acc, &chunk("m1", 0), "a", &[1.0, 0.0])
            .unwrap();
        store
            .put_embedding(&acc, &chunk("m1", 0), "a", &[0.0, 1.0])
            .unwrap();
        let hits = store.nearest(&acc, "a", &[0.0, 1.0], 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert!((hits[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_vector_for_an_unknown_message_is_refused() {
        let (_dir, mut store) = seeded();
        assert!(
            store
                .put_embedding(&account(), &chunk("ghost", 0), "a", &[1.0])
                .is_err()
        );
    }
}
