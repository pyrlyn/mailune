//! Thread summaries cached by content hash.
//!
//! The same thread and template version must not call the engine twice. The
//! hash includes the template id and version so a prompt change misses.

use std::collections::HashMap;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::catalog::from_digest;
use crate::{Error, LocalEngine, PromptTemplate, lookup, render};

/// Which summary the person asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryKind {
    /// A few lines.
    Short,
    /// The thread in full.
    Detailed,
    /// What someone still has to do.
    ActionItems,
}

impl SummaryKind {
    fn template_id(self) -> &'static str {
        match self {
            Self::Short => "summarize-short",
            Self::Detailed => "summarize-detailed",
            Self::ActionItems => "summarize-actions",
        }
    }
}

/// Summaries already produced. The text is not printed: it may be mail.
pub struct SummaryCache {
    entries: HashMap<[u8; 32], String>,
}

impl SummaryCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl Default for SummaryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SummaryCache {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SummaryCache")
            .field("entries", &self.entries.len())
            .finish()
    }
}

/// Returns the cached summary, or asks `engine` and stores the result.
///
/// # Errors
///
/// [`Error::UnknownPrompt`] when that summary template is not published.
/// [`Error::Unscripted`] when the engine has no answer.
pub fn summarize(
    engine: &impl LocalEngine,
    cache: &mut SummaryCache,
    kind: SummaryKind,
    thread: &str,
) -> Result<String, Error> {
    let template = lookup(kind.template_id(), 1)?;
    let key = cache_key(template, thread);
    if let Some(hit) = cache.entries.get(&key) {
        return Ok(hit.clone());
    }
    let text = engine.generate(&render(template, thread))?;
    cache.entries.insert(key, text.clone());
    Ok(text)
}

fn cache_key(template: &PromptTemplate, message: &str) -> [u8; 32] {
    let version = template.version.to_le_bytes();
    let mut hasher = Sha256::new();
    // A length prefix keeps two different splits of the same bytes from colliding.
    for part in [
        template.id.as_bytes(),
        version.as_slice(),
        message.as_bytes(),
    ] {
        let len = u64::try_from(part.len()).unwrap_or(u64::MAX);
        hasher.update(len.to_le_bytes());
        hasher.update(part);
    }
    from_digest(hasher.finalize().as_slice())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use serde_json::Value;

    use super::{SummaryCache, SummaryKind, summarize};
    use crate::{Error, LocalEngine, ScriptedEngine, lookup, render};

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

    fn script(engine: &mut ScriptedEngine, id: &str, thread: &str, answer: &str) {
        let template = lookup(id, 1).unwrap();
        engine.script_text(render(template, thread), answer);
    }

    #[test]
    fn three_summary_kinds_are_cached_by_content_hash() {
        let thread = "Ship the report tomorrow.";
        let mut scripted = ScriptedEngine::new();
        script(&mut scripted, "summarize-short", thread, "Ship tomorrow.");
        script(
            &mut scripted,
            "summarize-detailed",
            thread,
            "The report ships tomorrow.",
        );
        script(
            &mut scripted,
            "summarize-actions",
            thread,
            "Ship the report.",
        );
        let engine = Counting {
            inner: &scripted,
            calls: Cell::new(0),
        };
        let mut cache = SummaryCache::new();
        assert_eq!(
            summarize(&engine, &mut cache, SummaryKind::Short, thread).unwrap(),
            "Ship tomorrow."
        );
        assert_eq!(
            summarize(&engine, &mut cache, SummaryKind::Short, thread).unwrap(),
            "Ship tomorrow."
        );
        assert_eq!(engine.calls.get(), 1);
        assert_eq!(
            summarize(&engine, &mut cache, SummaryKind::Detailed, thread).unwrap(),
            "The report ships tomorrow."
        );
        assert_eq!(
            summarize(&engine, &mut cache, SummaryKind::ActionItems, thread).unwrap(),
            "Ship the report."
        );
        assert_eq!(engine.calls.get(), 3);
        let other = "Hold the report.";
        assert!(matches!(
            summarize(&engine, &mut cache, SummaryKind::Short, other),
            Err(Error::Unscripted)
        ));
        assert_eq!(format!("{cache:?}"), "SummaryCache { entries: 3 }");
    }
}
