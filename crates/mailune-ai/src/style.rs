//! Per-recipient writing style, learned from sent plain text.
//!
//! Counting only, no model call: the greeting and sign-off the person uses
//! most with a recipient, sentence and message length, and how often they use
//! exclamation marks. Quoted text and the signature are dropped first, so the
//! other side's words and a fixed footer do not count as the person's style.

use std::collections::HashMap;

use crate::redact_for_cloud;

/// What the composer can tell a model about writing to one recipient.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleProfile {
    /// Most used opening word, such as `Hi` or `Dear`.
    pub greeting: Option<String>,
    /// Most used closing line without its comma, such as `Best`.
    pub signoff: Option<String>,
    /// Mean words per sentence.
    pub sentence_words: f32,
    /// Mean words per message.
    pub message_words: f32,
    /// Share of messages with an exclamation mark.
    pub exclaims: f32,
    /// Messages learned from.
    pub samples: u32,
}

#[derive(Debug, Clone, Default)]
struct Tally {
    greetings: HashMap<String, u32>,
    signoffs: HashMap<String, u32>,
    sentences: u32,
    words: u32,
    exclaiming: u32,
    samples: u32,
}

/// Style tallies keyed by recipient address.
#[derive(Debug, Clone, Default)]
pub struct StyleBook {
    recipients: HashMap<String, Tally>,
}

const GREETINGS: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "dear",
    "morning",
    "afternoon",
    "evening",
];

impl StyleBook {
    /// An empty book.
    pub fn new() -> Self {
        Self::default()
    }

    /// Learns from one sent message to `recipient`.
    pub fn learn(&mut self, recipient: &str, body: &str) {
        let text = redact_for_cloud(body);
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let Some(first) = lines.first() else {
            return;
        };
        let tally = self
            .recipients
            .entry(recipient.trim().to_ascii_lowercase())
            .or_default();
        tally.samples += 1;
        if let Some(greeting) = greeting(first) {
            *tally.greetings.entry(greeting).or_default() += 1;
        }
        // The sign-off is the last line, or the one before a name under it.
        let closing = lines
            .get(1..)
            .map_or(&[][..], |rest| &rest[rest.len().saturating_sub(2)..]);
        if let Some(signoff) = closing.iter().rev().find_map(|line| signoff(line)) {
            *tally.signoffs.entry(signoff).or_default() += 1;
        }
        let words = u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX);
        let sentences = text
            .split(['.', '!', '?'])
            .filter(|sentence| !sentence.trim().is_empty())
            .count();
        tally.words = tally.words.saturating_add(words);
        tally.sentences = tally
            .sentences
            .saturating_add(u32::try_from(sentences.max(1)).unwrap_or(u32::MAX));
        if text.contains('!') {
            tally.exclaiming += 1;
        }
    }

    /// The profile for `recipient`, once at least one message was learned.
    pub fn profile(&self, recipient: &str) -> Option<StyleProfile> {
        let tally = self
            .recipients
            .get(&recipient.trim().to_ascii_lowercase())?;
        let samples = tally.samples.max(1) as f32;
        Some(StyleProfile {
            greeting: most_used(&tally.greetings),
            signoff: most_used(&tally.signoffs),
            sentence_words: tally.words as f32 / tally.sentences.max(1) as f32,
            message_words: tally.words as f32 / samples,
            exclaims: tally.exclaiming as f32 / samples,
            samples: tally.samples,
        })
    }
}

fn greeting(line: &str) -> Option<String> {
    let word = line.split(|c: char| c.is_whitespace() || c == ',').next()?;
    let known = GREETINGS
        .iter()
        .any(|greeting| word.eq_ignore_ascii_case(greeting));
    known.then(|| word.to_string())
}

fn signoff(line: &str) -> Option<String> {
    let stripped = line.strip_suffix(',')?;
    (stripped.split_whitespace().count() <= 3 && !stripped.is_empty()).then(|| stripped.to_string())
}

fn most_used(counts: &HashMap<String, u32>) -> Option<String> {
    // Ties break on the text so the profile does not depend on hash order.
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(text, _)| text.clone())
}

/// Renders `profile` as guidance a compose prompt can include.
pub fn guidance(profile: &StyleProfile) -> String {
    let mut lines = Vec::new();
    if let Some(greeting) = &profile.greeting {
        lines.push(format!("Open with \"{greeting}\"."));
    }
    if let Some(signoff) = &profile.signoff {
        lines.push(format!("Close with \"{signoff},\"."));
    }
    lines.push(format!(
        "Keep sentences around {:.0} words and the message around {:.0} words.",
        profile.sentence_words, profile.message_words
    ));
    if profile.exclaims < 0.2 {
        lines.push("Avoid exclamation marks.".into());
    }
    lines.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{StyleBook, guidance};

    #[test]
    fn a_recipient_profile_is_learned_and_rendered_as_guidance() {
        let mut book = StyleBook::new();
        book.learn(
            "Ana@Acme.io",
            "Hi Ana,\n\nThe deck is ready. I moved the demo.\n\nBest,\nIvan\n\n> old quoted text!\n",
        );
        book.learn(
            "ana@acme.io",
            "Hi Ana,\nFriday works. See you there.\nBest,\nIvan\n-- \nIvan Tugay | Acme!",
        );
        book.learn(
            "boss@acme.io",
            "Dear Maria,\nPlease find the report attached. Thank you!\nKind regards,\nIvan",
        );

        let ana = book.profile("ana@acme.io").unwrap();
        assert_eq!(ana.samples, 2);
        assert_eq!(ana.greeting.as_deref(), Some("Hi"));
        assert_eq!(ana.signoff.as_deref(), Some("Best"));
        assert_eq!(ana.exclaims, 0.0);
        assert!(ana.sentence_words < 6.0);
        let text = guidance(&ana);
        assert!(text.contains("Open with \"Hi\"."));
        assert!(text.contains("Close with \"Best,\"."));
        assert!(text.contains("Avoid exclamation marks."));

        let boss = book.profile("boss@acme.io").unwrap();
        assert_eq!(boss.greeting.as_deref(), Some("Dear"));
        assert_eq!(boss.signoff.as_deref(), Some("Kind regards"));
        assert!(!guidance(&boss).contains("Avoid exclamation"));

        assert!(book.profile("nobody@acme.io").is_none());
        book.learn("empty@acme.io", "\n> only quoted\n");
        assert!(book.profile("empty@acme.io").is_none());
    }
}
