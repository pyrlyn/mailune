//! Compose help that returns text from a local engine.
//!
//! Draft, rewrite, tone, shorten, and proofread each have a published prompt.
//! The source text is placed in the message slot, so it stays data.

use crate::{Error, LocalEngine, lookup, render};

/// What the person asked to do with a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeOp {
    /// Write a message from notes.
    Draft,
    /// Say the same thing another way.
    Rewrite,
    /// Change how formal the draft sounds.
    Tone,
    /// Make the draft shorter.
    Shorten,
    /// Fix the draft without changing its meaning.
    Proofread,
}

impl ComposeOp {
    fn template_id(self) -> &'static str {
        match self {
            Self::Draft => "compose-draft",
            Self::Rewrite => "compose-rewrite",
            Self::Tone => "compose-tone",
            Self::Shorten => "compose-shorten",
            Self::Proofread => "compose-proofread",
        }
    }
}

/// Runs `op` on `source` through `engine`.
///
/// # Errors
///
/// [`Error::UnknownPrompt`] when that compose template is not published.
/// [`Error::Unscripted`] when the engine has no answer.
pub fn assist(engine: &impl LocalEngine, op: ComposeOp, source: &str) -> Result<String, Error> {
    let template = lookup(op.template_id(), 1)?;
    engine.generate(&render(template, source))
}

#[cfg(test)]
mod tests {
    use super::{ComposeOp, assist};
    use crate::{ScriptedEngine, lookup, render};

    #[test]
    fn each_compose_op_returns_scripted_text() {
        let source = "the build is ready";
        let cases = [
            (ComposeOp::Draft, "compose-draft", "The build is ready."),
            (
                ComposeOp::Rewrite,
                "compose-rewrite",
                "The build is ready to ship.",
            ),
            (
                ComposeOp::Tone,
                "compose-tone",
                "The build is ready, thanks.",
            ),
            (ComposeOp::Shorten, "compose-shorten", "Build ready."),
            (
                ComposeOp::Proofread,
                "compose-proofread",
                "The build is ready.",
            ),
        ];
        let mut engine = ScriptedEngine::new();
        for (_, id, answer) in cases {
            let template = lookup(id, 1).unwrap();
            engine.script_text(render(template, source), answer);
        }
        for (op, _, answer) in cases {
            assert_eq!(assist(&engine, op, source).unwrap(), answer);
        }
    }
}
