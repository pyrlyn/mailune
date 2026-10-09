//! Compose assist: draft, rewrite, tone, shorten and proofread.
//!
//! Each action is one versioned template. The tone is a closed set, so the
//! person's choice is never free text inside the prompt. The result is a
//! suggestion for the composer; it is not sent.

use crate::{Completion, Error, Privacy, Prompt, Provider, lookup, render};

/// Tones the composer offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Formal and polite.
    Formal,
    /// Warm and casual.
    Friendly,
    /// Short and to the point.
    Direct,
}

impl Tone {
    fn word(self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Friendly => "friendly",
            Self::Direct => "direct",
        }
    }
}

/// What the person asked the assistant to do with the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeAction {
    /// Write a draft from the person's notes.
    Draft,
    /// Rewrite for clarity, same meaning.
    Rewrite,
    /// Rewrite in a tone.
    Tone(Tone),
    /// Make it shorter.
    Shorten,
    /// Fix spelling and grammar only.
    Proofread,
}

/// Runs `action` on `text`. The text may quote mail, so it stays data.
///
/// # Errors
///
/// The provider's error, or [`Error::BadOutput`] for an empty reply.
pub async fn assist<P: Provider>(
    provider: &P,
    action: ComposeAction,
    text: &str,
    privacy: Privacy,
) -> Result<String, Error> {
    let (id, input) = match action {
        ComposeAction::Draft => ("compose-draft", text.to_string()),
        ComposeAction::Rewrite => ("compose-rewrite", text.to_string()),
        ComposeAction::Tone(tone) => ("compose-tone", format!("Tone: {}\n\n{text}", tone.word())),
        ComposeAction::Shorten => ("compose-shorten", text.to_string()),
        ComposeAction::Proofread => ("compose-proofread", text.to_string()),
    };
    let template = lookup(id, 1)?;
    let prompt = Prompt {
        feature: template.feature,
        text: render(template, &input),
        privacy: privacy.class,
        encrypted: privacy.encrypted,
    };
    let Completion::Text(reply) = provider.complete(&prompt).await?;
    let reply = reply.trim();
    if reply.is_empty() {
        return Err(Error::BadOutput);
    }
    Ok(reply.to_string())
}

#[cfg(test)]
mod tests {
    use super::{ComposeAction, Tone, assist};
    use crate::summary::tests::LOCAL;
    use crate::testing::drive;
    use crate::{Error, Feature, LocalProvider, ScriptedEngine};

    #[test]
    fn each_action_returns_text_from_the_scripted_engine() {
        let actions = [
            (ComposeAction::Draft, "email draft", "Hi Ana, Friday works."),
            (ComposeAction::Rewrite, "rewrite", "Friday works for me."),
            (
                ComposeAction::Tone(Tone::Formal),
                "Tone: formal",
                "Dear Ana,",
            ),
            (ComposeAction::Shorten, "shorter", "Friday."),
            (ComposeAction::Proofread, "spelling", "Friday works."),
        ];
        let engine = ScriptedEngine::new(
            actions.iter().map(|(_, _, reply)| format!(" {reply}\n")),
            [Feature::Compose],
        );
        let provider = LocalProvider::new(engine, 256);
        for (index, (action, marker, reply)) in actions.iter().enumerate() {
            let text = drive(assist(&provider, *action, "friday ok? ana", LOCAL)).unwrap();
            assert_eq!(text, *reply);
            let prompt = &provider.engine().seen()[index].prompt;
            assert!(prompt.contains(marker), "{marker}");
            assert!(prompt.contains("friday ok? ana"));
        }
    }

    #[test]
    fn an_empty_reply_is_not_a_suggestion() {
        let provider = LocalProvider::new(ScriptedEngine::new(["  "], [Feature::Compose]), 64);
        assert!(matches!(
            drive(assist(&provider, ComposeAction::Shorten, "text", LOCAL)),
            Err(Error::BadOutput)
        ));
    }
}
