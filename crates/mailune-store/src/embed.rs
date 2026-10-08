//! Embedding rows and cosine nearest neighbours.
//!
//! The distance is computed in Rust. sqlite-vec is not linked.

use diesel::prelude::*;

use crate::schema::embeddings;
use crate::{Error, Store};

/// One neighbour, higher cosine first.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbour {
    /// Stored id.
    pub id: String,
    /// Cosine similarity with the query.
    pub score: f32,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = embeddings)]
#[diesel(primary_key(id))]
struct NewEmbedding {
    id: String,
    dims: i32,
    vector: Vec<u8>,
}

#[derive(Queryable)]
struct StoredEmbedding {
    id: String,
    dims: i32,
    vector: Vec<u8>,
}

impl Store {
    /// Inserts or replaces the vector for `id`.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the write fails or the length does not fit.
    pub fn upsert_embedding(&mut self, id: &str, vector: &[f32]) -> Result<(), Error> {
        let dims = i32::try_from(vector.len()).map_err(|_| Error::Query)?;
        let row = NewEmbedding {
            id: id.to_string(),
            dims,
            vector: encode(vector),
        };
        diesel::insert_into(embeddings::table)
            .values(&row)
            .on_conflict(embeddings::id)
            .do_update()
            .set(&row)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    /// The `limit` stored vectors closest to `query` by cosine.
    ///
    /// A row whose length differs from `query` is skipped.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the rows cannot be read.
    pub fn nearest(&mut self, query: &[f32], limit: i64) -> Result<Vec<Neighbour>, Error> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        let rows = embeddings::table
            .load::<StoredEmbedding>(&mut self.conn)
            .map_err(|_| Error::Query)?;
        let mut scored = Vec::new();
        for row in rows {
            let Some(vector) = decode(&row.vector) else {
                continue;
            };
            if vector.len() != query.len() || i32::try_from(vector.len()).ok() != Some(row.dims) {
                continue;
            }
            scored.push(Neighbour {
                id: row.id,
                score: cosine(query, &vector),
            });
        }
        scored.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.id.cmp(&right.id))
        });
        let keep = usize::try_from(limit).unwrap_or(scored.len());
        scored.truncate(keep);
        Ok(scored)
    }
}

fn encode(vector: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn decode(bytes: &[u8]) -> Option<Vec<f32>> {
    let (chunks, rest) = bytes.as_chunks::<4>();
    if !rest.is_empty() {
        return None;
    }
    Some(chunks.iter().copied().map(f32::from_le_bytes).collect())
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut left_norm = 0.0f32;
    let mut right_norm = 0.0f32;
    for (a, b) in left.iter().zip(right) {
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    let denom = left_norm.sqrt() * right_norm.sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}

#[cfg(test)]
mod tests {
    use crate::{Store, scratch_dir};

    #[test]
    fn nearest_neighbour_is_the_closest_cosine() {
        let dir = scratch_dir();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        store.upsert_embedding("a", &[1.0, 0.0]).unwrap();
        store.upsert_embedding("b", &[0.0, 1.0]).unwrap();
        store.upsert_embedding("c", &[1.0, 1.0]).unwrap();
        store.upsert_embedding("wide", &[1.0, 0.0, 0.0]).unwrap();
        store.upsert_embedding("b", &[0.0, 2.0]).unwrap();

        let hits = store.nearest(&[0.0, 1.0], 2).unwrap();
        assert_eq!(hits[0].id, "b");
        assert!((hits[0].score - 1.0).abs() < 0.001);
        assert_eq!(hits[1].id, "c");
        assert!(hits.iter().all(|hit| hit.id != "wide"));
        assert!(store.nearest(&[0.0, 1.0], 0).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
