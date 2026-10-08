//! Picks an embedding model by scoring candidates on labelled probes, then
//! stores the winner's chunk vectors. Embedders are pluggable engines, so a
//! broken one is skipped and reported, never fatal on its own.

use mailune_protocol::AccountId;

use crate::vector::{ChunkRef, norm};
use crate::{Error, Store};

/// A text embedding engine.
pub trait Embedder {
    /// Model name the vectors are stored under.
    fn model(&self) -> &str;

    /// One vector per text, in order.
    ///
    /// # Errors
    ///
    /// Any engine failure, as text for the skip report.
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String>;
}

/// A query and the chunk that should answer it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe<'a> {
    /// What a person might ask.
    pub query: &'a str,
    /// The chunk that answers it.
    pub relevant: ChunkRef,
}

/// The chosen model.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// Winning model name.
    pub model: String,
    /// Its mean reciprocal rank over the probes, 0 to 1.
    pub score: f32,
    /// Models skipped because their engine failed or returned bad vectors.
    pub skipped: Vec<String>,
}

impl Store {
    /// Embeds `chunks` with every candidate, scores each by mean reciprocal
    /// rank of the relevant chunk over `probes`, and stores the best
    /// candidate's vectors. A tie keeps the earlier candidate.
    ///
    /// # Errors
    ///
    /// [`Error::NoEmbedder`] when no candidate produced usable vectors;
    /// [`Error::Database`] when a write fails.
    pub fn embed_best(
        &mut self,
        account: &AccountId,
        chunks: &[(ChunkRef, &str)],
        probes: &[Probe<'_>],
        candidates: &[&dyn Embedder],
    ) -> Result<Choice, Error> {
        let texts: Vec<&str> = chunks.iter().map(|(_, text)| *text).collect();
        let queries: Vec<&str> = probes.iter().map(|probe| probe.query).collect();
        let mut best: Option<(f32, &dyn Embedder, Vec<Vec<f32>>)> = None;
        let mut skipped = Vec::new();
        for candidate in candidates {
            let Some((vectors, asked)) = usable(*candidate, &texts, &queries) else {
                skipped.push(candidate.model().to_string());
                continue;
            };
            let score = mean_reciprocal_rank(chunks, &vectors, probes, &asked);
            if best.as_ref().is_none_or(|(top, _, _)| score > *top) {
                best = Some((score, *candidate, vectors));
            }
        }
        let (score, winner, vectors) = best.ok_or(Error::NoEmbedder)?;
        for ((chunk, _), vector) in chunks.iter().zip(&vectors) {
            self.put_embedding(account, chunk, winner.model(), vector)?;
        }
        Ok(Choice {
            model: winner.model().to_string(),
            score,
            skipped,
        })
    }
}

type Vectors = Vec<Vec<f32>>;

/// Chunk and query vectors, or `None` when the engine failed, returned the
/// wrong count, mixed dimensions, or sent a non-finite value.
fn usable(embedder: &dyn Embedder, texts: &[&str], queries: &[&str]) -> Option<(Vectors, Vectors)> {
    let vectors = embedder.embed(texts).ok()?;
    let asked = embedder.embed(queries).ok()?;
    if vectors.len() != texts.len() || asked.len() != queries.len() {
        return None;
    }
    let dim = vectors.first().or(asked.first())?.len();
    let sound = vectors
        .iter()
        .chain(&asked)
        .all(|vector| vector.len() == dim && vector.iter().all(|value| value.is_finite()));
    (dim > 0 && sound).then_some((vectors, asked))
}

fn mean_reciprocal_rank(
    chunks: &[(ChunkRef, &str)],
    vectors: &[Vec<f32>],
    probes: &[Probe<'_>],
    asked: &[Vec<f32>],
) -> f32 {
    if probes.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    for (probe, query) in probes.iter().zip(asked) {
        let Some(target) = chunks
            .iter()
            .position(|(chunk, _)| *chunk == probe.relevant)
        else {
            continue;
        };
        let target_score = cosine(query, &vectors[target]);
        let higher = vectors
            .iter()
            .filter(|vector| cosine(query, vector) > target_score)
            .count();
        // Ties (the target among them) share the worst rank of the group, so
        // an embedder that cannot tell chunks apart scores low.
        let ties = vectors
            .iter()
            .filter(|vector| cosine(query, vector) == target_score)
            .count();
        let rank = higher + ties;
        total += 1.0 / rank as f32;
    }
    total / probes.len() as f32
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let denominator = norm(left.iter().copied()) * norm(right.iter().copied());
    if denominator == 0.0 {
        return 0.0;
    }
    left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>() / denominator
}

#[cfg(test)]
mod tests {
    use mailune_mime::chunk_plain;
    use mailune_protocol::MessageId;

    use super::{Embedder, Probe};
    use crate::testutil::{account, message, seeded};
    use crate::{ChunkRef, Error};

    /// Hashes words into buckets, so shared words mean close vectors.
    struct Words;
    /// Every text gets the same vector: it cannot tell chunks apart.
    struct Flat;
    /// A broken engine.
    struct Broken;

    impl Embedder for Words {
        fn model(&self) -> &str {
            "words-64"
        }
        fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts
                .iter()
                .map(|text| {
                    let mut vector = vec![0.0; 64];
                    for word in text.split_whitespace() {
                        let word = word
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_lowercase();
                        let bucket = word.bytes().fold(7usize, |hash, byte| {
                            hash.wrapping_mul(31).wrapping_add(usize::from(byte))
                        });
                        vector[bucket % 64] += 1.0;
                    }
                    vector
                })
                .collect())
        }
    }

    impl Embedder for Flat {
        fn model(&self) -> &str {
            "flat-4"
        }
        fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts.iter().map(|_| vec![1.0; 4]).collect())
        }
    }

    impl Embedder for Broken {
        fn model(&self) -> &str {
            "broken"
        }
        fn embed(&self, _: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            Ok(vec![vec![f32::NAN]])
        }
    }

    #[test]
    fn the_better_embedder_wins_and_its_vectors_are_stored() {
        let (_dir, mut store) = seeded();
        let bodies = [
            (
                "m1",
                "The dock meeting moved to Friday at noon.\n\n> old quoted text\n",
            ),
            (
                "m2",
                "Invoice 42 for the boat repair is attached, due next month.",
            ),
            (
                "m3",
                "Photos from the harbour trip, mostly sunsets and gulls.",
            ),
        ];
        let mut owned = Vec::new();
        for (index, (id, body)) in bodies.iter().enumerate() {
            store
                .upsert_message(&message(id, "t1", 100 + index as i64, true))
                .unwrap();
            for (chunk, piece) in chunk_plain(body).into_iter().enumerate() {
                let reference = ChunkRef {
                    message: MessageId::new(*id),
                    chunk: u32::try_from(chunk).unwrap(),
                };
                owned.push((reference, piece.text));
            }
        }
        assert!(!owned[0].1.contains("quoted"));
        let chunks: Vec<(ChunkRef, &str)> = owned
            .iter()
            .map(|(reference, text)| (reference.clone(), text.as_str()))
            .collect();
        let probe = |query, id: &str| Probe {
            query,
            relevant: ChunkRef {
                message: MessageId::new(id),
                chunk: 0,
            },
        };
        let probes = [
            probe("when is the dock meeting", "m1"),
            probe("boat repair invoice", "m2"),
            probe("harbour sunsets photos", "m3"),
        ];

        let choice = store
            .embed_best(&account(), &chunks, &probes, &[&Broken, &Flat, &Words])
            .unwrap();
        assert_eq!(choice.model, "words-64");
        assert!(
            (choice.score - 1.0).abs() < f32::EPSILON,
            "{}",
            choice.score
        );
        assert_eq!(choice.skipped, ["broken"]);

        let query = Words.embed(&["invoice for the boat"]).unwrap();
        let hits = store.nearest(&account(), "words-64", &query[0], 1).unwrap();
        assert_eq!(hits[0].chunk.message, MessageId::new("m2"));
        assert!(
            store
                .nearest(&account(), "flat-4", &[1.0; 4], 1)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn no_usable_embedder_is_an_error() {
        let (_dir, mut store) = seeded();
        store.upsert_message(&message("m1", "t1", 1, true)).unwrap();
        let chunks = [(
            ChunkRef {
                message: MessageId::new("m1"),
                chunk: 0,
            },
            "text",
        )];
        assert!(matches!(
            store.embed_best(&account(), &chunks, &[], &[&Broken]),
            Err(Error::NoEmbedder)
        ));
    }
}
