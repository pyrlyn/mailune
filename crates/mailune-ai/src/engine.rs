//! Local generation without a model process.
//!
//! [`Provider`](crate::Provider) only completes a prompt. A local engine also
//! embeds and returns JSON, and a test can supply both without starting a
//! runtime or fetching weights.

use std::collections::HashMap;
use std::fmt;

use serde_json::Value;

use crate::Error;

/// On-device generation. Implementors are tests or a script; this crate does
/// not spawn a process and does not download weights.
pub trait LocalEngine {
    /// Free-form text for `prompt`.
    ///
    /// # Errors
    ///
    /// [`Error::Unscripted`] when this engine has no answer for `prompt`.
    fn generate(&self, prompt: &str) -> Result<String, Error>;

    /// A vector for `text`. The length is the engine's choice.
    ///
    /// # Errors
    ///
    /// [`Error::Unscripted`] when this engine has no vector for `text`.
    fn embed(&self, text: &str) -> Result<Vec<f32>, Error>;

    /// JSON for `prompt`. Callers treat the value as untrusted.
    ///
    /// # Errors
    ///
    /// [`Error::Unscripted`] when this engine has no JSON for `prompt`.
    fn structured(&self, prompt: &str) -> Result<Value, Error>;
}

/// Canned answers keyed by the exact prompt or text.
///
/// The maps are not printed: a prompt may contain mail.
pub struct ScriptedEngine {
    text: HashMap<String, String>,
    vectors: HashMap<String, Vec<f32>>,
    json: HashMap<String, Value>,
}

impl ScriptedEngine {
    /// An engine with no scripts. Every call returns [`Error::Unscripted`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: HashMap::new(),
            vectors: HashMap::new(),
            json: HashMap::new(),
        }
    }

    /// Returns `text` from [`LocalEngine::generate`] when the prompt matches.
    pub fn script_text(&mut self, prompt: impl Into<String>, text: impl Into<String>) {
        self.text.insert(prompt.into(), text.into());
    }

    /// Returns `vector` from [`LocalEngine::embed`] when the text matches.
    pub fn script_embed(&mut self, text: impl Into<String>, vector: Vec<f32>) {
        self.vectors.insert(text.into(), vector);
    }

    /// Returns `value` from [`LocalEngine::structured`] when the prompt matches.
    pub fn script_json(&mut self, prompt: impl Into<String>, value: Value) {
        self.json.insert(prompt.into(), value);
    }
}

impl Default for ScriptedEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ScriptedEngine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScriptedEngine")
            .field("text", &self.text.len())
            .field("vectors", &self.vectors.len())
            .field("json", &self.json.len())
            .finish()
    }
}

impl LocalEngine for ScriptedEngine {
    fn generate(&self, prompt: &str) -> Result<String, Error> {
        self.text.get(prompt).cloned().ok_or(Error::Unscripted)
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        self.vectors.get(text).cloned().ok_or(Error::Unscripted)
    }

    fn structured(&self, prompt: &str) -> Result<Value, Error> {
        self.json.get(prompt).cloned().ok_or(Error::Unscripted)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{LocalEngine, ScriptedEngine};
    use crate::Error;

    /// The test is the engine. No binary is spawned.
    struct Twice;

    impl LocalEngine for Twice {
        fn generate(&self, prompt: &str) -> Result<String, Error> {
            Ok(format!("{prompt}{prompt}"))
        }

        fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
            let len = u16::try_from(text.len()).unwrap_or(u16::MAX);
            Ok(vec![f32::from(len)])
        }

        fn structured(&self, prompt: &str) -> Result<Value, Error> {
            Ok(json!({ "echo": prompt }))
        }
    }

    #[test]
    fn a_test_engine_returns_text_a_vector_and_json() {
        let engine = Twice;
        assert_eq!(engine.generate("ab").unwrap(), "abab");
        assert_eq!(engine.embed("abcd").unwrap(), vec![4.0]);
        assert_eq!(engine.structured("hi").unwrap(), json!({ "echo": "hi" }));
    }

    #[test]
    fn a_script_returns_only_what_was_recorded() {
        let mut engine = ScriptedEngine::new();
        engine.script_text("prompt", "answer");
        engine.script_embed("mail", vec![1.0, 0.5]);
        engine.script_json("prompt", json!({ "label": "invoice" }));
        assert_eq!(engine.generate("prompt").unwrap(), "answer");
        assert_eq!(engine.embed("mail").unwrap(), vec![1.0, 0.5]);
        assert_eq!(
            engine.structured("prompt").unwrap(),
            json!({ "label": "invoice" })
        );
        assert!(matches!(engine.generate("other"), Err(Error::Unscripted)));
        assert!(matches!(engine.embed("other"), Err(Error::Unscripted)));
        assert!(matches!(engine.structured("other"), Err(Error::Unscripted)));
        assert_eq!(
            format!("{engine:?}"),
            "ScriptedEngine { text: 1, vectors: 1, json: 1 }"
        );
        assert!(!format!("{engine:?}").contains("answer"));
    }
}
