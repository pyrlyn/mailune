//! Local engine adapter: generation, embeddings and JSON-shaped output.
//!
//! [`LocalEngine`] is what a bundled runtime (llama.cpp through runa) will
//! implement. This crate never starts a process or downloads a model; the
//! runtime lives in an adapter crate, and tests use [`ScriptedEngine`].

use std::future::Future;
use std::sync::Mutex;

use serde::de::DeserializeOwned;

use crate::{Completion, Error, Feature, ModelCapability, ModelKind, Prompt, Provider};

/// Shape the engine is asked to produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Free text.
    Text,
    /// One JSON value. A runtime that supports grammars constrains to it.
    Json,
}

/// One generation request. `prompt` may contain mail, so `Debug` hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct Generate {
    /// Text for the model.
    pub prompt: String,
    /// Upper bound on generated tokens.
    pub max_tokens: u32,
    /// Requested output shape.
    pub format: OutputFormat,
}

impl std::fmt::Debug for Generate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Generate")
            .field("prompt", &"redacted")
            .field("max_tokens", &self.max_tokens)
            .field("format", &self.format)
            .finish()
    }
}

/// A model that runs on this device.
pub trait LocalEngine: Send + Sync {
    /// What the router may pick this engine for.
    fn capability(&self) -> ModelCapability;

    /// Generates text for `request`.
    ///
    /// # Errors
    ///
    /// [`Error::Engine`] when the runtime fails.
    fn generate(&self, request: &Generate) -> impl Future<Output = Result<String, Error>> + Send;

    /// One embedding vector for `text`.
    ///
    /// # Errors
    ///
    /// [`Error::Engine`] when the runtime fails.
    fn embed(&self, text: &str) -> impl Future<Output = Result<Vec<f32>, Error>> + Send;
}

/// Asks `engine` for JSON and decodes it as `T`. The output is untrusted, so
/// a reply that does not decode is an error, not a guess.
///
/// # Errors
///
/// [`Error::BadOutput`] when the reply is not a `T`; engine errors as is.
pub async fn generate_json<T, E>(engine: &E, prompt: String, max_tokens: u32) -> Result<T, Error>
where
    T: DeserializeOwned,
    E: LocalEngine,
{
    let request = Generate {
        prompt,
        max_tokens,
        format: OutputFormat::Json,
    };
    let reply = engine.generate(&request).await?;
    serde_json::from_str(reply.trim()).map_err(|_| Error::BadOutput)
}

/// A [`LocalEngine`] behind the router's [`Provider`] trait.
pub struct LocalProvider<E> {
    engine: E,
    max_tokens: u32,
}

impl<E> LocalProvider<E> {
    /// Wraps `engine`; every completion is capped at `max_tokens`.
    pub fn new(engine: E, max_tokens: u32) -> Self {
        Self { engine, max_tokens }
    }

    /// The wrapped engine.
    pub fn engine(&self) -> &E {
        &self.engine
    }
}

impl<E: LocalEngine> Provider for LocalProvider<E> {
    fn name(&self) -> &str {
        "local"
    }

    fn capabilities(&self) -> Vec<ModelCapability> {
        let mut capability = self.engine.capability();
        // Whatever the engine says, it runs here: never let it pose as cloud.
        capability.kind = ModelKind::Local;
        vec![capability]
    }

    async fn complete(&self, prompt: &Prompt) -> Result<Completion, Error> {
        let request = Generate {
            prompt: prompt.text.clone(),
            max_tokens: self.max_tokens,
            format: OutputFormat::Text,
        };
        self.engine.generate(&request).await.map(Completion::Text)
    }
}

/// An engine that replays canned replies in order and embeds by hashing
/// words. For tests, evaluation cassettes, and a device with no model yet.
#[derive(Debug, Default)]
pub struct ScriptedEngine {
    replies: Mutex<Vec<String>>,
    seen: Mutex<Vec<Generate>>,
    features: Vec<Feature>,
}

/// Dimensions of a [`ScriptedEngine`] embedding.
pub const SCRIPTED_DIMENSIONS: usize = 16;

impl ScriptedEngine {
    /// Replays `replies` first to last for every feature in `features`.
    pub fn new(
        replies: impl IntoIterator<Item = impl Into<String>>,
        features: impl IntoIterator<Item = Feature>,
    ) -> Self {
        let mut replies: Vec<String> = replies.into_iter().map(Into::into).collect();
        replies.reverse();
        Self {
            replies: Mutex::new(replies),
            seen: Mutex::new(Vec::new()),
            features: features.into_iter().collect(),
        }
    }

    /// Requests received so far, oldest first.
    pub fn seen(&self) -> Vec<Generate> {
        self.seen
            .lock()
            .map(|seen| seen.clone())
            .unwrap_or_default()
    }

    fn next_reply(&self, request: &Generate) -> Result<String, Error> {
        self.seen.lock().map_err(poisoned)?.push(request.clone());
        self.replies
            .lock()
            .map_err(poisoned)?
            .pop()
            .ok_or_else(|| Error::Engine("script exhausted".into()))
    }
}

fn poisoned<T>(_: T) -> Error {
    Error::Engine("scripted engine state".into())
}

impl LocalEngine for ScriptedEngine {
    fn capability(&self) -> ModelCapability {
        ModelCapability {
            id: "scripted".into(),
            kind: ModelKind::Local,
            context_tokens: None,
            features: self.features.clone(),
        }
    }

    async fn generate(&self, request: &Generate) -> Result<String, Error> {
        self.next_reply(request)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        Ok(hash_embedding(text))
    }
}

/// A unit vector from word hashes: the same words give the same vector, and
/// shared words point the vectors closer together.
pub fn hash_embedding(text: &str) -> Vec<f32> {
    let mut vector = [0f32; SCRIPTED_DIMENSIONS];
    for word in text.split_whitespace() {
        let hash = word
            .to_lowercase()
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            });
        let slot = usize::try_from(hash % SCRIPTED_DIMENSIONS as u64).unwrap_or(0);
        vector[slot] += 1.0;
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        vector.iter_mut().for_each(|value| *value /= norm);
    }
    vector.to_vec()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{
        LocalEngine, LocalProvider, OutputFormat, SCRIPTED_DIMENSIONS, ScriptedEngine,
        generate_json, hash_embedding,
    };
    use crate::testing::drive;
    use crate::{Completion, Error, Feature, ModelKind, PrivacyClass, Prompt, Provider};

    #[derive(Debug, Deserialize, PartialEq)]
    struct Label {
        label: String,
        score: u8,
    }

    #[test]
    fn generation_embedding_and_json_come_from_the_trait() {
        let engine = ScriptedEngine::new(
            [
                "plain reply",
                r#" {"label":"invoice","score":7} "#,
                "not json",
            ],
            [Feature::Summarize],
        );
        let provider = LocalProvider::new(engine, 128);
        assert_eq!(provider.capabilities()[0].kind, ModelKind::Local);
        let prompt = Prompt {
            feature: Feature::Summarize,
            text: "mail body".into(),
            privacy: PrivacyClass::LocalOnly,
            encrypted: true,
        };
        let Completion::Text(text) = drive(provider.complete(&prompt)).unwrap();
        assert_eq!(text, "plain reply");

        let engine = provider.engine();
        let label: Label = drive(generate_json(engine, "classify".into(), 32)).unwrap();
        assert_eq!(
            label,
            Label {
                label: "invoice".into(),
                score: 7
            }
        );
        let bad = drive(generate_json::<Label, _>(engine, "classify".into(), 32));
        assert!(matches!(bad, Err(Error::BadOutput)));
        assert!(matches!(
            drive(generate_json::<Label, _>(engine, "again".into(), 32)),
            Err(Error::Engine(_))
        ));

        let seen = engine.seen();
        assert_eq!(seen[0].max_tokens, 128);
        assert_eq!(seen[1].format, OutputFormat::Json);
        assert!(!format!("{:?}", seen[0]).contains("mail body"));

        let vector = drive(engine.embed("Invoice due Friday")).unwrap();
        assert_eq!(vector.len(), SCRIPTED_DIMENSIONS);
        assert_eq!(vector, hash_embedding("invoice DUE friday"));
        let norm: f32 = vector.iter().map(|v| v * v).sum();
        assert!((norm - 1.0).abs() < 1e-5);
        assert!(hash_embedding("").iter().all(|v| *v == 0.0));
    }
}
