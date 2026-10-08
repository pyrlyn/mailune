//! A writing style learned from mail the person already sent.
//!
//! The profile is a few lines and some counts from those messages. A model
//! would only paraphrase what the letters already show, so nothing here calls one.

use std::collections::HashMap;

const SIGNOFFS: &[&str] = &[
    "thanks",
    "thank you",
    "cheers",
    "best",
    "regards",
    "sincerely",
    "best regards",
    "kind regards",
    "yours",
    "talk soon",
];

const FORMAL: &[&str] = &[" dear ", " sincerely ", " regards ", " yours faithfully "];
const CASUAL: &[&str] = &[" hey ", " hi ", " cheers ", " thanks "];

struct Accum {
    messages: u32,
    greeting: Option<String>,
    signoff: Option<String>,
    chars: u64,
    sentences: u64,
    formal: u32,
    casual: u32,
}

impl Accum {
    fn new() -> Self {
        Self {
            messages: 0,
            greeting: None,
            signoff: None,
            chars: 0,
            sentences: 0,
            formal: 0,
            casual: 0,
        }
    }
}

/// Per-recipient style learned from sent plain text.
pub struct StyleBook {
    profiles: HashMap<String, Accum>,
}

impl StyleBook {
    /// An empty book.
    #[must_use]
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Folds one sent message into the profile for `recipient`.
    ///
    /// Blank text and a blank recipient are ignored. The latest greeting and
    /// sign-off win, because that is the style the person just used.
    pub fn learn(&mut self, recipient: &str, sent: &str) {
        let sent = sent.trim();
        let key = recipient.trim().to_ascii_lowercase();
        if sent.is_empty() || key.is_empty() {
            return;
        }
        let profile = self.profiles.entry(key).or_insert_with(Accum::new);
        let lines = non_empty_lines(sent);
        let (greeting, signoff) = greeting_and_signoff(&lines);
        if greeting.is_some() {
            profile.greeting = greeting;
        }
        if signoff.is_some() {
            profile.signoff = signoff;
        }
        let (chars, sentences) = sentence_stats(&body_text(&lines));
        profile.chars = profile.chars.saturating_add(chars);
        profile.sentences = profile.sentences.saturating_add(sentences);
        let flat = flatten(sent);
        if FORMAL.iter().any(|marker| flat.contains(marker)) {
            profile.formal = profile.formal.saturating_add(1);
        }
        if CASUAL.iter().any(|marker| flat.contains(marker)) {
            profile.casual = profile.casual.saturating_add(1);
        }
        profile.messages = profile.messages.saturating_add(1);
    }

    /// Guidance for writing to `recipient`, or `None` when nothing was learned.
    #[must_use]
    pub fn guidance(&self, recipient: &str) -> Option<String> {
        let profile = self.profiles.get(&recipient.trim().to_ascii_lowercase())?;
        if profile.messages == 0 {
            return None;
        }
        let mut parts = Vec::new();
        if let Some(greeting) = &profile.greeting {
            parts.push(format!("Start with a greeting like {greeting}"));
        }
        if let Some(signoff) = &profile.signoff {
            parts.push(format!("End with a sign-off like {signoff}"));
        }
        if let Some(average) = profile.chars.checked_div(profile.sentences) {
            parts.push(format!("Keep sentences near {average} characters"));
        }
        let tone = if profile.formal > profile.casual {
            "formal"
        } else if profile.casual > profile.formal {
            "casual"
        } else {
            "neutral"
        };
        parts.push(format!("The tone is {tone}"));
        Some(parts.join(". ") + ".")
    }
}

impl Default for StyleBook {
    fn default() -> Self {
        Self::new()
    }
}

fn non_empty_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

fn is_greeting(line: &str) -> bool {
    line.ends_with(',') && line.chars().count() <= 60 && !line.contains('.')
}

fn is_signoff(line: &str) -> bool {
    let normalized = line.trim().trim_end_matches(',').trim().to_lowercase();
    SIGNOFFS.contains(&normalized.as_str())
}

fn greeting_and_signoff(lines: &[&str]) -> (Option<String>, Option<String>) {
    if lines.len() < 2 {
        return (None, None);
    }
    let greeting = lines
        .first()
        .copied()
        .filter(|line| is_greeting(line))
        .map(ToOwned::to_owned);
    let signoff = lines
        .last()
        .copied()
        .filter(|line| is_signoff(line))
        .map(ToOwned::to_owned);
    (greeting, signoff)
}

fn body_text(lines: &[&str]) -> String {
    let Some(first) = lines.first() else {
        return String::new();
    };
    let Some(last) = lines.last() else {
        return String::new();
    };
    if lines.len() < 2 {
        return lines.join(" ");
    }
    let start = usize::from(is_greeting(first));
    let end = if is_signoff(last) {
        lines.len() - 1
    } else {
        lines.len()
    };
    if start >= end {
        return String::new();
    }
    lines[start..end].join(" ")
}

fn sentence_stats(body: &str) -> (u64, u64) {
    let mut chars = 0u64;
    let mut count = 0u64;
    for sentence in body.split(['.', '!', '?']) {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }
        chars = chars.saturating_add(u64::try_from(sentence.chars().count()).unwrap_or(0));
        count = count.saturating_add(1);
    }
    if count > 0 {
        return (chars, count);
    }
    let len = u64::try_from(body.trim().chars().count()).unwrap_or(0);
    if len == 0 { (0, 0) } else { (len, 1) }
}

/// Markers are whole words. A line break is whitespace, not part of the word.
fn flatten(text: &str) -> String {
    let mut flat = String::from(" ");
    for character in text.chars() {
        if character.is_whitespace() {
            if !flat.ends_with(' ') {
                flat.push(' ');
            }
        } else {
            for lower in character.to_lowercase() {
                flat.push(lower);
            }
        }
    }
    if !flat.ends_with(' ') {
        flat.push(' ');
    }
    flat
}

#[cfg(test)]
mod tests {
    use super::StyleBook;

    #[test]
    fn a_profile_is_learned_per_recipient_without_a_model() {
        let mut book = StyleBook::new();
        book.learn("ada@acme.io", "   ");
        assert!(book.guidance("ada@acme.io").is_none());

        book.learn("Ada@acme.io", "Hey Ada,\n\nThe build is ready.\n\nCheers");
        let casual = book.guidance("ada@acme.io").unwrap();
        assert!(casual.contains("Hey Ada,"));
        assert!(casual.contains("Cheers"));
        assert!(casual.contains("casual"));
        assert!(casual.contains("18 characters"));
        assert!(book.guidance("other@acme.io").is_none());

        book.learn(
            "boss@acme.io",
            "Dear Morgan,\n\nPlease find the attached file.\n\nSincerely",
        );
        let formal = book.guidance("boss@acme.io").unwrap();
        assert!(formal.contains("Dear Morgan,"));
        assert!(formal.contains("Sincerely"));
        assert!(formal.contains("formal"));
        assert!(casual.contains("casual"));
    }
}
