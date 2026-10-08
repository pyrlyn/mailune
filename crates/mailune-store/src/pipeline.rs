//! Embedding pipeline over scripted embedders.
//!
//! The caller supplies both scorers. This module does not download a model
//! and does not depend on the MIME or AI crates.

use std::cmp::Ordering;

use crate::{Error, Store};

/// How many whitespace-separated words go in one stored vector.
const CHUNK_WORDS: usize = 4;

/// A scorer the pipeline did not train. `embed` returns a vector and a quality score.
pub struct ScriptedEmbedder<'a> {
    /// Name used to break a score tie.
    pub name: &'a str,
    /// Vector and score for one chunk. Higher score is better.
    pub embed: &'a dyn Fn(&str) -> (Vec<f32>, f32),
}

/// Which embedder was stored, and how many chunks that was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineResult {
    /// Winning embedder name.
    pub winner: String,
    /// Chunks written. Zero means the text had no words.
    pub chunks: usize,
}

/// Chunks `text`, scores both embedders, and stores the winner's vectors.
///
/// The stored id is `{document_id}#{index}`. The chunk text itself is not written.
///
/// # Errors
///
/// [`Error::Query`] when a vector cannot be stored.
pub fn embed_text(
    store: &mut Store,
    document_id: &str,
    text: &str,
    left: &ScriptedEmbedder<'_>,
    right: &ScriptedEmbedder<'_>,
) -> Result<PipelineResult, Error> {
    let chunks = chunk_words(text);
    if chunks.is_empty() {
        let winner = if left.name <= right.name {
            left.name
        } else {
            right.name
        };
        return Ok(PipelineResult {
            winner: winner.to_string(),
            chunks: 0,
        });
    }
    let left_scored = score(left, &chunks);
    let right_scored = score(right, &chunks);
    let winner = match right_scored.score.total_cmp(&left_scored.score) {
        Ordering::Greater => right_scored,
        Ordering::Less => left_scored,
        Ordering::Equal if right_scored.name < left_scored.name => right_scored,
        Ordering::Equal => left_scored,
    };
    let chunks = winner.vectors.len();
    for (index, vector) in winner.vectors.iter().enumerate() {
        store.upsert_embedding(&format!("{document_id}#{index}"), vector)?;
    }
    Ok(PipelineResult {
        winner: winner.name,
        chunks,
    })
}

struct Scored {
    name: String,
    score: f32,
    vectors: Vec<Vec<f32>>,
}

fn score(embedder: &ScriptedEmbedder<'_>, chunks: &[String]) -> Scored {
    let mut score = 0.0;
    let mut vectors = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let (vector, part) = (embedder.embed)(chunk);
        score += part;
        vectors.push(vector);
    }
    Scored {
        name: embedder.name.to_string(),
        score,
        vectors,
    }
}

fn chunk_words(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    words
        .chunks(CHUNK_WORDS)
        .map(|window| window.join(" "))
        .filter(|chunk| !chunk.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ScriptedEmbedder, embed_text};
    use crate::{Store, scratch_dir};

    #[test]
    fn the_higher_score_is_the_vector_that_is_stored() {
        let dir = scratch_dir();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        let plain = ScriptedEmbedder {
            name: "plain",
            embed: &|_text| (vec![1.0, 0.0], 0.2),
        };
        let rich = ScriptedEmbedder {
            name: "rich",
            embed: &|_text| (vec![0.0, 1.0], 0.9),
        };
        let text = "one two three four five six seven eight";
        let result = embed_text(&mut store, "m1", text, &plain, &rich).unwrap();
        assert_eq!(result.winner, "rich");
        assert_eq!(result.chunks, 2);
        let nearest = store.nearest(&[0.0, 1.0], 2).unwrap();
        assert_eq!(nearest.len(), 2);
        assert!(nearest.iter().all(|hit| hit.id.starts_with("m1#")));
        let _ = std::fs::remove_dir_all(dir);
    }
}
