//! Three reply suggestions for one thread.
//!
//! Each suggestion is its own published prompt so a scripted engine can
//! return three different lines. The thread is data in every prompt.

use crate::{Error, LocalEngine, lookup, render};

const TEMPLATES: [&str; 3] = ["reply-one", "reply-two", "reply-three"];

/// Three short replies for `thread`, in prompt order.
///
/// # Errors
///
/// [`Error::UnknownPrompt`] when a reply template is not published.
/// [`Error::Unscripted`] when the engine has no answer.
pub fn suggest_replies(engine: &impl LocalEngine, thread: &str) -> Result<[String; 3], Error> {
    Ok([
        one(engine, TEMPLATES[0], thread)?,
        one(engine, TEMPLATES[1], thread)?,
        one(engine, TEMPLATES[2], thread)?,
    ])
}

fn one(engine: &impl LocalEngine, id: &str, thread: &str) -> Result<String, Error> {
    let template = lookup(id, 1)?;
    engine.generate(&render(template, thread))
}

#[cfg(test)]
mod tests {
    use super::{TEMPLATES, suggest_replies};
    use crate::{ScriptedEngine, lookup, render};

    #[test]
    fn a_thread_yields_three_suggestions() {
        let thread = "Can you review the draft today?";
        let mut engine = ScriptedEngine::new();
        let answers = [
            "Yes, this afternoon.",
            "Tomorrow works.",
            "Sending notes now.",
        ];
        for (id, answer) in TEMPLATES.into_iter().zip(answers) {
            let template = lookup(id, 1).unwrap();
            engine.script_text(render(template, thread), answer);
        }
        assert_eq!(
            suggest_replies(&engine, thread).unwrap(),
            answers.map(str::to_owned)
        );
    }
}
