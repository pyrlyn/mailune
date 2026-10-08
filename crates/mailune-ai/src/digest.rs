//! A digest of mail since an instant.
//!
//! Older messages are left out before a summary is asked for, so a script
//! that only knows the recent thread is enough.

use crate::{Error, LocalEngine, SummaryCache, SummaryKind, summarize};

/// One message the digest may include.
#[derive(Debug, Clone, Copy)]
pub struct DigestMessage<'a> {
    /// Unix seconds. Compared with `since`.
    pub at: u64,
    /// Plain text. May be mail.
    pub text: &'a str,
}

/// Short summaries of messages at or after `since`, in input order.
///
/// A message older than `since` is skipped and never reaches the engine.
/// The instant itself is included.
///
/// # Errors
///
/// Errors from [`summarize`](crate::summarize).
pub fn digest(
    engine: &impl LocalEngine,
    cache: &mut SummaryCache,
    messages: &[DigestMessage<'_>],
    since: u64,
) -> Result<Vec<String>, Error> {
    let mut summaries = Vec::new();
    for message in messages {
        if message.at < since {
            continue;
        }
        summaries.push(summarize(engine, cache, SummaryKind::Short, message.text)?);
    }
    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use serde_json::Value;

    use super::{DigestMessage, digest};
    use crate::{Error, LocalEngine, ScriptedEngine, SummaryCache, lookup, render};

    struct Counting<'a> {
        inner: &'a ScriptedEngine,
        calls: Cell<u32>,
    }

    impl LocalEngine for Counting<'_> {
        fn generate(&self, prompt: &str) -> Result<String, Error> {
            self.calls.set(self.calls.get() + 1);
            self.inner.generate(prompt)
        }

        fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
            self.inner.embed(text)
        }

        fn structured(&self, prompt: &str) -> Result<Value, Error> {
            self.inner.structured(prompt)
        }
    }

    fn script(engine: &mut ScriptedEngine, thread: &str, answer: &str) {
        let template = lookup("summarize-short", 1).unwrap();
        engine.script_text(render(template, thread), answer);
    }

    #[test]
    fn a_digest_skips_messages_older_than_since() {
        let older = "Ancient note.";
        let recent = "Ship the report.";
        let newer = "Hold the review.";
        let mut scripted = ScriptedEngine::new();
        script(&mut scripted, recent, "Ship it.");
        script(&mut scripted, newer, "Hold it.");
        let engine = Counting {
            inner: &scripted,
            calls: Cell::new(0),
        };
        let messages = [
            DigestMessage {
                at: 50,
                text: older,
            },
            DigestMessage {
                at: 100,
                text: recent,
            },
            DigestMessage {
                at: 150,
                text: newer,
            },
        ];
        let mut cache = SummaryCache::new();
        let none = digest(&engine, &mut cache, &messages, 1_000).unwrap();
        assert!(none.is_empty());
        assert_eq!(engine.calls.get(), 0);
        let summaries = digest(&engine, &mut cache, &messages, 100).unwrap();
        assert_eq!(
            summaries,
            vec!["Ship it.".to_owned(), "Hold it.".to_owned()]
        );
        assert_eq!(engine.calls.get(), 2);
        let again = digest(&engine, &mut cache, &messages, 100).unwrap();
        assert_eq!(again, summaries);
        assert_eq!(engine.calls.get(), 2);
    }
}
