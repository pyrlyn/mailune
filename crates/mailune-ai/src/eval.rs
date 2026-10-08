//! Evaluation cassettes for a feature call.
//!
//! A cassette replays one feature through the local engine. The metric fails
//! the run when the output is not the recorded text. Nothing here calls the
//! network, and this stays in the crate rather than becoming a shared test kit.

use std::fmt;

use crate::{ComposeOp, Error, Feature, LocalEngine, SummaryCache, SummaryKind, assist, summarize};

/// One recorded feature call. The strings may be mail, so [`Debug`] redacts them.
pub struct Cassette {
    /// Which feature to replay.
    pub feature: Feature,
    /// The thread or the draft notes.
    pub input: String,
    /// The text the metric requires.
    pub expected: String,
}

impl fmt::Debug for Cassette {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Cassette")
            .field("feature", &self.feature)
            .field("input", &"redacted")
            .field("expected", &"redacted")
            .finish()
    }
}

/// Fails when `actual` is not the recorded text.
///
/// The error does not include either string: both may be mail.
///
/// # Errors
///
/// [`Error::Drift`] when the strings differ.
pub fn exact_match(expected: &str, actual: &str) -> Result<(), Error> {
    if expected == actual {
        Ok(())
    } else {
        Err(Error::Drift)
    }
}

/// Replays `cassette` and checks the output with [`exact_match`].
///
/// # Errors
///
/// Errors from the feature, or [`Error::Drift`] when the output changed.
pub fn replay(
    engine: &impl LocalEngine,
    cache: &mut SummaryCache,
    cassette: &Cassette,
) -> Result<(), Error> {
    let actual = match cassette.feature {
        Feature::Summarize => summarize(engine, cache, SummaryKind::Short, &cassette.input)?,
        Feature::DraftReply => assist(engine, ComposeOp::Draft, &cassette.input)?,
    };
    exact_match(&cassette.expected, &actual)
}

#[cfg(test)]
mod tests {
    use super::{Cassette, exact_match, replay};
    use crate::{Error, Feature, ScriptedEngine, SummaryCache, lookup, render};

    #[test]
    fn a_cassette_replays_a_feature_and_a_metric_fails_on_drift() {
        let summary_input = "Ship the report";
        let summary = lookup("summarize-short", 1).unwrap();
        let notes = "ask for the file";
        let draft = lookup("compose-draft", 1).unwrap();
        let mut engine = ScriptedEngine::new();
        engine.script_text(render(summary, summary_input), "Ship it.");
        engine.script_text(render(draft, notes), "Could you send the file?");

        let cassette = Cassette {
            feature: Feature::Summarize,
            input: summary_input.into(),
            expected: "Ship it.".into(),
        };
        replay(&engine, &mut SummaryCache::new(), &cassette).unwrap();
        assert!(!format!("{cassette:?}").contains("Ship"));

        let drifted = Cassette {
            expected: "Do not ship.".into(),
            ..cassette
        };
        assert!(matches!(
            replay(&engine, &mut SummaryCache::new(), &drifted),
            Err(Error::Drift)
        ));
        assert!(matches!(exact_match("a", "b"), Err(Error::Drift)));

        let draft_cassette = Cassette {
            feature: Feature::DraftReply,
            input: notes.into(),
            expected: "Could you send the file?".into(),
        };
        replay(&engine, &mut SummaryCache::new(), &draft_cassette).unwrap();
    }
}
