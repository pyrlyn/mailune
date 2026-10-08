//! Thread summaries, cached by content hash.
//!
//! The key is a SHA-256 over the template id, its version and the rendered
//! thread, so an edited thread or a new prompt version misses the cache and a
//! re-opened thread does not call the model again.

use std::collections::HashMap;
use std::time::SystemTime;

use mailune_protocol::MessageId;
use sha2::{Digest, Sha256};

use crate::catalog::hex;
use crate::{Completion, Error, PrivacyClass, Prompt, PromptTemplate, Provider, lookup, render};

/// One message as plain text. The body is untrusted data.
#[derive(Clone, PartialEq, Eq)]
pub struct MailText {
    /// Message id, for provenance.
    pub id: MessageId,
    /// When it arrived.
    pub received: SystemTime,
    /// `From`.
    pub from: String,
    /// `Subject`.
    pub subject: String,
    /// Plain body.
    pub body: String,
}

impl std::fmt::Debug for MailText {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MailText")
            .field("id", &self.id)
            .field("received", &self.received)
            .finish_non_exhaustive()
    }
}

/// Which summary the person asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SummaryKind {
    /// One or two sentences.
    Short,
    /// A paragraph per topic.
    Detailed,
    /// A list of things someone has to do.
    ActionItems,
}

impl SummaryKind {
    /// The published template for this kind.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownPrompt`] if the registry lost the template.
    pub fn template(self) -> Result<&'static PromptTemplate, Error> {
        match self {
            Self::Short => lookup("summarize-short", 1),
            Self::Detailed => lookup("summarize-detailed", 1),
            Self::ActionItems => lookup("action-items", 1),
        }
    }
}

/// Where a thread's prompt may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Privacy {
    /// Account or feature class.
    pub class: PrivacyClass,
    /// The thread has an end-to-end encrypted message.
    pub encrypted: bool,
}

/// Joins messages into the text a template's `{message}` slot gets.
pub fn render_thread(messages: &[MailText]) -> String {
    messages
        .iter()
        .map(|message| {
            format!(
                "From: {}\nSubject: {}\n\n{}\n",
                message.from, message.subject, message.body
            )
        })
        .collect::<Vec<_>>()
        .join("---\n")
}

/// Summaries already computed in this process.
#[derive(Debug, Clone, Default)]
pub struct SummaryCache {
    entries: HashMap<String, String>,
}

impl SummaryCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of cached summaries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing is cached.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The `kind` summary of `messages`, from the cache or from `provider`.
    ///
    /// # Errors
    ///
    /// The provider's error. A failed call caches nothing.
    pub async fn summarize<P: Provider>(
        &mut self,
        provider: &P,
        messages: &[MailText],
        kind: SummaryKind,
        privacy: Privacy,
    ) -> Result<String, Error> {
        let thread = render_thread(messages);
        self.complete(provider, kind.template()?, &thread, privacy)
            .await
    }

    /// Fills `template` with `text` and completes it, from the cache when the
    /// same template version has seen the same text.
    ///
    /// # Errors
    ///
    /// The provider's error. A failed call caches nothing.
    pub async fn complete<P: Provider>(
        &mut self,
        provider: &P,
        template: &PromptTemplate,
        text: &str,
        privacy: Privacy,
    ) -> Result<String, Error> {
        let key = cache_key(template, text);
        if let Some(hit) = self.entries.get(&key) {
            return Ok(hit.clone());
        }
        let prompt = Prompt {
            feature: template.feature,
            text: render(template, text),
            privacy: privacy.class,
            encrypted: privacy.encrypted,
        };
        let Completion::Text(text) = provider.complete(&prompt).await?;
        let text = text.trim().to_string();
        self.entries.insert(key, text.clone());
        Ok(text)
    }
}

/// Content hash of `thread` under `template`.
pub fn cache_key(template: &PromptTemplate, thread: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(template.id.as_bytes());
    hasher.update(template.version.to_be_bytes());
    hasher.update(thread.as_bytes());
    hex(&hasher.finalize())
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::{Duration, SystemTime};

    use mailune_protocol::MessageId;

    use super::{MailText, Privacy, SummaryCache, SummaryKind};
    use crate::testing::drive;
    use crate::{Error, Feature, LocalProvider, PrivacyClass, ScriptedEngine};

    pub(crate) fn mail(id: &str, minutes: u64, body: &str) -> MailText {
        MailText {
            id: MessageId::new(id),
            received: SystemTime::UNIX_EPOCH + Duration::from_secs(minutes * 60),
            from: "Ana <ana@acme.io>".into(),
            subject: "Launch".into(),
            body: body.into(),
        }
    }

    pub(crate) const LOCAL: Privacy = Privacy {
        class: PrivacyClass::LocalPreferred,
        encrypted: false,
    };

    #[test]
    fn three_kinds_are_cached_by_content_hash() {
        let engine = ScriptedEngine::new(
            [
                "Launch moved to Friday.",
                "Ana moved the launch. QA needs a day.",
                "- Bo: book the room",
                "Launch moved again.",
            ],
            [Feature::Summarize],
        );
        let provider = LocalProvider::new(engine, 256);
        let thread = vec![mail("m1", 1, "Launch moves to Friday. Bo, book the room.")];
        let mut cache = SummaryCache::new();
        let short = drive(cache.summarize(&provider, &thread, SummaryKind::Short, LOCAL)).unwrap();
        let detailed =
            drive(cache.summarize(&provider, &thread, SummaryKind::Detailed, LOCAL)).unwrap();
        let actions =
            drive(cache.summarize(&provider, &thread, SummaryKind::ActionItems, LOCAL)).unwrap();
        assert_eq!(short, "Launch moved to Friday.");
        assert!(detailed.contains("QA"));
        assert_eq!(actions, "- Bo: book the room");
        assert_eq!(cache.len(), 3);

        // Same content: no model call.
        let again = drive(cache.summarize(&provider, &thread, SummaryKind::Short, LOCAL)).unwrap();
        assert_eq!(again, short);
        assert_eq!(provider.engine().seen().len(), 3);
        assert!(provider.engine().seen()[2].prompt.contains("action"));

        // Changed content: a new key and a new call.
        let edited = vec![mail("m1", 1, "Launch moves to Monday.")];
        let moved = drive(cache.summarize(&provider, &edited, SummaryKind::Short, LOCAL)).unwrap();
        assert_eq!(moved, "Launch moved again.");
        assert_eq!(cache.len(), 4);

        // A failed call caches nothing.
        let failed = drive(cache.summarize(&provider, &thread[..0], SummaryKind::Short, LOCAL));
        assert!(matches!(failed, Err(Error::Engine(_))));
        assert_eq!(cache.len(), 4);
        assert!(!format!("{:?}", thread[0]).contains("Friday"));
    }
}
