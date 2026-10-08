//! Feature routing and privacy classes.
//!
//! Encrypted mail is local-only even when the account would allow a cloud
//! model. This crate does not call a network provider.

mod agent;
mod attachment;
pub mod catalog;
mod cloud;
mod compose;
mod digest;
mod engine;
mod eval;
mod guard;
mod ledger;
mod phishing;
mod platform;
mod priority;
mod prompts;
mod redact;
mod router;
mod rules;
mod style;
mod summary;
mod triage;

#[cfg(test)]
mod testing;

pub use agent::{
    Agent, AuditLine, Confirmation, Outcome, Pending, Preview, Requested, Scope, ToolCall,
    UndoRecord, needs_confirmation, parse_call,
};
pub use attachment::{ATTACHMENT_LIMIT, Attachment, attachment_text, summarize_attachment};
pub use cloud::{CloudApi, CloudRequest, KeyHeader, build_request, parse_response};
pub use compose::{ComposeAction, Tone, assist};
pub use digest::{Digest, DigestEntry, DigestThread, daily_digest};
pub use engine::{
    Generate, LocalEngine, LocalProvider, OutputFormat, SCRIPTED_DIMENSIONS, ScriptedEngine,
    generate_json, hash_embedding,
};
pub use eval::{Cassette, EvalCall, EvalResult, replay, run_cassettes, token_f1};
pub use guard::{Policy, Tool, ToolProposal, admit, parse_proposal, proposal_from_mail};
pub use ledger::{FlowRecord, Ledger, Retention};
pub use phishing::{
    Assessment, AuthOutcome, LinkFlags, ModelVerdict, PhishingSignals, Reason, Risk,
    assess_phishing, combine,
};
pub use platform::{PlatformBridge, map_capability};
pub use priority::{Priority, PriorityInput, assess};
pub use prompts::{PromptTemplate, lookup, registry, render};
pub use redact::redact_for_cloud;
pub use router::{FeaturePolicy, RouteRequest, Router, probe};
pub use rules::{
    Condition, EnabledRule, ProposedRule, RuleAction, RuleMessage, RulePreview, rule_from_sentence,
};
pub use style::{StyleBook, StyleProfile, guidance};
pub use summary::{MailText, Privacy, SummaryCache, SummaryKind, cache_key, render_thread};
pub use triage::{TriageInput, categorize};

use std::fmt;
use std::future::Future;

/// What the person asked the model to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// A short summary of one conversation.
    Summarize,
    /// A draft reply.
    DraftReply,
    /// Draft, rewrite, tone, shorten or proofread in the composer.
    Compose,
    /// Turn a sentence into a mail rule.
    Rules,
    /// A phishing and scam verdict on one message.
    Phishing,
}

/// Where a feature may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyClass {
    /// The device only. Forced for encrypted mail.
    LocalOnly,
    /// Prefer the device. Cloud is not used.
    LocalPreferred,
    /// A bring-your-own-key cloud model is allowed.
    CloudAllowed,
}

/// Which kind of model a capability describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// The host's on-device model.
    Platform,
    /// A model bundled with the app.
    Local,
    /// A cloud model. Never used for encrypted mail.
    Cloud,
}

/// One model the router can choose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCapability {
    /// Stable name.
    pub id: String,
    /// Where this model runs.
    pub kind: ModelKind,
    /// Context window, when known.
    pub context_tokens: Option<u32>,
    /// Features this model can serve.
    pub features: Vec<Feature>,
}

/// A completion request. `text` may contain mail, so `Debug` redacts it.
#[derive(Clone, PartialEq, Eq)]
pub struct Prompt {
    /// Feature being served.
    pub feature: Feature,
    /// Text for the model. May contain mail. Do not log it.
    pub text: String,
    /// Account or feature class, before the encrypted-mail override.
    pub privacy: PrivacyClass,
    /// The message is end-to-end encrypted.
    pub encrypted: bool,
}

impl fmt::Debug for Prompt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Prompt")
            .field("feature", &self.feature)
            .field("text", &"redacted")
            .field("privacy", &self.privacy)
            .field("encrypted", &self.encrypted)
            .finish()
    }
}

/// Typed model output. The text is untrusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completion {
    /// Plain text the model returned.
    Text(String),
}

/// A model backend. Callers are generic over it; the future is not `dyn`.
pub trait Provider: Send + Sync {
    /// Short name for logs. Not a secret.
    fn name(&self) -> &str;

    /// Models this provider can run.
    fn capabilities(&self) -> Vec<ModelCapability>;

    /// Runs `prompt`.
    ///
    /// # Errors
    ///
    /// [`Error`] when the provider cannot run the prompt.
    fn complete(&self, prompt: &Prompt) -> impl Future<Output = Result<Completion, Error>> + Send;
}

/// Failure returned by routing and by providers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The contract crate refused the call.
    #[error(transparent)]
    Contract(#[from] mailune_protocol::Error),
    /// A cloud call was asked for mail that must stay on the device.
    #[error("cloud is not allowed for this mail")]
    CloudForbidden,
    /// The feature has no privacy row, so the router will not guess one.
    #[error("no privacy policy for this feature")]
    NoPolicy,
    /// The prompt is larger than the feature budget.
    #[error("prompt exceeds the feature budget")]
    OverBudget,
    /// No model on the fallback chain can serve this prompt.
    #[error("no provider can serve this prompt")]
    NoProvider,
    /// A tool proposal was missing, malformed, or not on the allow-list.
    #[error("tool call denied")]
    ToolDenied,
    /// No published template has that id and version.
    #[error("unknown prompt template")]
    UnknownPrompt,
    /// A tool call named a conversation outside the granted scope.
    #[error("tool call is outside the granted scope")]
    OutOfScope,
    /// Send and delete wait for the person to confirm in the app.
    #[error("tool call needs confirmation in the app")]
    NeedsConfirmation,
    /// The local runtime failed. The text names the failure, never the prompt.
    #[error("local engine: {0}")]
    Engine(String),
    /// Model output did not have the shape the feature asked for.
    #[error("model output did not have the expected shape")]
    BadOutput,
    /// A model blob had the wrong size or digest. It was deleted.
    #[error("model blob failed verification")]
    BadBlob,
    /// The input is of a kind this feature does not read.
    #[error("unsupported input for this feature")]
    Unsupported,
    /// A cloud provider answered with an error. Only its error type is kept.
    #[error("cloud provider error: {0}")]
    Cloud(String),
    /// Evaluation cassettes no longer match their recording. Names them.
    #[error("evaluation drifted: {0}")]
    Drift(String),
}

/// Class after the encrypted-mail rule. Encrypted mail is always local-only.
pub fn effective_privacy(class: PrivacyClass, encrypted: bool) -> PrivacyClass {
    if encrypted {
        PrivacyClass::LocalOnly
    } else {
        class
    }
}

/// Picks a model for `feature`. Local and platform models win. A cloud model
/// is eligible only when the effective class is [`PrivacyClass::CloudAllowed`].
pub fn select(
    models: &[ModelCapability],
    feature: Feature,
    class: PrivacyClass,
    encrypted: bool,
) -> Option<&ModelCapability> {
    let class = effective_privacy(class, encrypted);
    let supports = |model: &&ModelCapability| model.features.contains(&feature);
    let on_device =
        |model: &&ModelCapability| matches!(model.kind, ModelKind::Platform | ModelKind::Local);
    models
        .iter()
        .find(|model| supports(model) && on_device(model))
        .or_else(|| {
            if class == PrivacyClass::CloudAllowed {
                models
                    .iter()
                    .find(|model| supports(model) && model.kind == ModelKind::Cloud)
            } else {
                None
            }
        })
}

/// Allows a cloud call only when the effective class is cloud-allowed.
///
/// # Errors
///
/// [`Error::CloudForbidden`] for local-only and local-preferred mail, including
/// every encrypted message.
pub fn allow_cloud(prompt: &Prompt) -> Result<(), Error> {
    if effective_privacy(prompt.privacy, prompt.encrypted) == PrivacyClass::CloudAllowed {
        Ok(())
    } else {
        Err(Error::CloudForbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Completion, Error, Feature, ModelCapability, ModelKind, PrivacyClass, Prompt, Provider,
        allow_cloud, select,
    };
    use crate::testing::drive;

    fn model(id: &str, kind: ModelKind) -> ModelCapability {
        ModelCapability {
            id: id.into(),
            kind,
            context_tokens: None,
            features: vec![Feature::Summarize],
        }
    }

    struct Echo;

    impl Provider for Echo {
        fn name(&self) -> &str {
            "echo"
        }

        fn capabilities(&self) -> Vec<ModelCapability> {
            vec![model("echo", ModelKind::Local)]
        }

        async fn complete(&self, prompt: &Prompt) -> Result<Completion, Error> {
            Ok(Completion::Text(prompt.text.clone()))
        }
    }

    #[test]
    fn encrypted_mail_stays_on_a_local_model() {
        let models = vec![
            model("cloud", ModelKind::Cloud),
            model("local", ModelKind::Local),
        ];
        let chosen = select(
            &models,
            Feature::Summarize,
            PrivacyClass::CloudAllowed,
            true,
        )
        .expect("local model");
        assert_eq!(chosen.id, "local");
        assert_eq!(chosen.kind, ModelKind::Local);
        let prompt = Prompt {
            feature: Feature::Summarize,
            text: "secret body".into(),
            privacy: PrivacyClass::CloudAllowed,
            encrypted: true,
        };
        assert!(matches!(allow_cloud(&prompt), Err(Error::CloudForbidden)));
        assert!(!format!("{prompt:?}").contains("secret body"));
    }

    #[test]
    fn local_preferred_does_not_select_a_cloud_model() {
        let models = vec![model("cloud", ModelKind::Cloud)];
        assert!(
            select(
                &models,
                Feature::Summarize,
                PrivacyClass::LocalPreferred,
                false
            )
            .is_none()
        );
        let chosen = select(
            &models,
            Feature::Summarize,
            PrivacyClass::CloudAllowed,
            false,
        )
        .expect("cloud model");
        assert_eq!(chosen.kind, ModelKind::Cloud);
    }

    #[test]
    fn a_provider_returns_typed_text() {
        let echo = Echo;
        assert_eq!(echo.name(), "echo");
        assert_eq!(echo.capabilities()[0].id, "echo");
        let prompt = Prompt {
            feature: Feature::Summarize,
            text: "hello".into(),
            privacy: PrivacyClass::LocalOnly,
            encrypted: false,
        };
        let Completion::Text(text) = drive(echo.complete(&prompt)).unwrap();
        assert_eq!(text, "hello");
    }
}
