//! Evaluation cassettes.
//!
//! A cassette is one recorded feature call: the input, the digest of the
//! prompt the feature built, the model's reply, and the output the feature
//! made from that reply. Replaying it through a scripted engine reruns the
//! feature's own prompt building and output handling with no model and no
//! network, so a template edit or a parsing change that alters results fails
//! the run instead of shipping quietly.

use std::collections::HashMap;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::catalog::hex;
use crate::{
    ComposeAction, Error, Feature, LocalProvider, Privacy, PrivacyClass, ScriptedEngine,
    SummaryCache, SummaryKind, assist, rule_from_sentence,
};

/// Generous enough for every replayed reply; the engine ignores it.
const REPLAY_TOKENS: u32 = 1024;

/// The feature call a cassette recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalCall {
    /// A short thread summary.
    SummarizeShort,
    /// Compose: rewrite.
    Rewrite,
    /// Compose: shorten.
    Shorten,
    /// Compose: proofread.
    Proofread,
    /// A sentence turned into a rule.
    RuleFromSentence,
}

impl EvalCall {
    fn feature(self) -> Feature {
        match self {
            Self::SummarizeShort => Feature::Summarize,
            Self::Rewrite | Self::Shorten | Self::Proofread => Feature::Compose,
            Self::RuleFromSentence => Feature::Rules,
        }
    }
}

/// One recorded call, read from a JSON fixture.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cassette {
    /// Name shown when it drifts.
    pub name: String,
    /// Feature call.
    pub call: EvalCall,
    /// What the feature was given.
    pub input: String,
    /// SHA-256 hex of the prompt the feature built.
    pub prompt_sha256: String,
    /// The model's recorded reply.
    pub reply: String,
    /// The output the feature made from the reply.
    pub expected: String,
    /// Lowest [`token_f1`] that still passes. `1.0` asks for the same words.
    pub min_score: f32,
}

/// What one replay found.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalResult {
    /// Cassette name.
    pub name: String,
    /// The output the feature made this time.
    pub output: String,
    /// [`token_f1`] of the output against the recording.
    pub score: f32,
    /// The feature built a different prompt than the one recorded.
    pub prompt_drift: bool,
}

impl EvalResult {
    /// Whether the replay matches the recording closely enough.
    pub fn passed(&self, cassette: &Cassette) -> bool {
        !self.prompt_drift && self.score >= cassette.min_score
    }
}

/// F1 over lowercase word counts, in `0.0..=1.0`. Two empty texts score 1.
pub fn token_f1(output: &str, expected: &str) -> f32 {
    let output = words(output);
    let expected = words(expected);
    let (found, wanted): (u32, u32) = (output.values().sum(), expected.values().sum());
    if found == 0 && wanted == 0 {
        return 1.0;
    }
    let common: u32 = output
        .iter()
        .map(|(word, count)| (*count).min(expected.get(word).copied().unwrap_or(0)))
        .sum();
    if common == 0 {
        return 0.0;
    }
    let precision = common as f32 / found as f32;
    let recall = common as f32 / wanted as f32;
    2.0 * precision * recall / (precision + recall)
}

fn words(text: &str) -> HashMap<String, u32> {
    let mut counts = HashMap::new();
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        *counts.entry(word.to_lowercase()).or_insert(0) += 1;
    }
    counts
}

/// Replays `cassette` through the feature it recorded.
///
/// # Errors
///
/// The feature's own error, such as [`Error::BadOutput`] when a recorded rule
/// reply no longer parses.
pub async fn replay(cassette: &Cassette) -> Result<EvalResult, Error> {
    let feature = cassette.call.feature();
    let provider = LocalProvider::new(
        ScriptedEngine::new([cassette.reply.as_str()], [feature]),
        REPLAY_TOKENS,
    );
    // Replays never leave the device, whatever the recording's class was.
    let privacy = Privacy {
        class: PrivacyClass::LocalOnly,
        encrypted: false,
    };
    let input = cassette.input.as_str();
    let output = match cassette.call {
        EvalCall::SummarizeShort => {
            SummaryCache::new()
                .complete(&provider, SummaryKind::Short.template()?, input, privacy)
                .await?
        }
        EvalCall::Rewrite => assist(&provider, ComposeAction::Rewrite, input, privacy).await?,
        EvalCall::Shorten => assist(&provider, ComposeAction::Shorten, input, privacy).await?,
        EvalCall::Proofread => assist(&provider, ComposeAction::Proofread, input, privacy).await?,
        EvalCall::RuleFromSentence => {
            format!("{:?}", rule_from_sentence(&provider, input, privacy).await?)
        }
    };
    let prompt = provider
        .engine()
        .seen()
        .first()
        .map(|request| hex(&Sha256::digest(request.prompt.as_bytes())))
        .unwrap_or_default();
    Ok(EvalResult {
        name: cassette.name.clone(),
        score: token_f1(&output, &cassette.expected),
        output,
        prompt_drift: !prompt.eq_ignore_ascii_case(&cassette.prompt_sha256),
    })
}

/// Replays every cassette and fails when any drifted.
///
/// # Errors
///
/// [`Error::Drift`] naming every cassette that failed or drifted.
pub async fn run_cassettes(cassettes: &[Cassette]) -> Result<Vec<EvalResult>, Error> {
    let mut results = Vec::with_capacity(cassettes.len());
    let mut drifted = Vec::new();
    for cassette in cassettes {
        match replay(cassette).await {
            Ok(result) if result.passed(cassette) => results.push(result),
            Ok(result) => {
                drifted.push(cassette.name.clone());
                results.push(result);
            }
            Err(_) => drifted.push(cassette.name.clone()),
        }
    }
    if drifted.is_empty() {
        Ok(results)
    } else {
        Err(Error::Drift(drifted.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::{Cassette, EvalCall, replay, run_cassettes, token_f1};
    use crate::Error;
    use crate::testing::drive;

    fn cassettes() -> Vec<Cassette> {
        serde_json::from_str(include_str!("../cassettes/features.json")).unwrap()
    }

    #[test]
    fn recorded_cassettes_replay_without_drift() {
        let cassettes = cassettes();
        assert!(cassettes.len() >= 4);
        let results = drive(run_cassettes(&cassettes)).unwrap();
        assert!(results.iter().all(|result| !result.prompt_drift));
    }

    #[test]
    fn a_changed_output_or_prompt_fails_the_run() {
        let mut cassettes = cassettes();
        let shorten = cassettes
            .iter_mut()
            .find(|cassette| cassette.call == EvalCall::Shorten)
            .unwrap();
        shorten.expected = "Completely different words here.".into();
        let Err(Error::Drift(names)) = drive(run_cassettes(&cassettes)) else {
            panic!("drift not caught");
        };
        assert_eq!(names, "shorten-meeting");

        let mut prompt = cassettes[0].clone();
        prompt.input.push_str(" and one more clause");
        let result = drive(replay(&prompt)).unwrap();
        assert!(result.prompt_drift);
        assert!(!result.passed(&prompt));

        let mut rule = cassettes
            .into_iter()
            .find(|cassette| cassette.call == EvalCall::RuleFromSentence)
            .unwrap();
        rule.reply = r#"{"when":[],"then":{"action":"archive"}}"#.into();
        assert!(drive(run_cassettes(&[rule])).is_err());
    }

    #[test]
    fn token_f1_scores_word_overlap() {
        assert_eq!(token_f1("", ""), 1.0);
        assert_eq!(token_f1("Friday works.", "friday works"), 1.0);
        assert_eq!(token_f1("a b", "c d"), 0.0);
        let half = token_f1("friday works", "friday is fine works");
        assert!((half - 2.0 / 3.0).abs() < 1e-6, "{half}");
    }
}
