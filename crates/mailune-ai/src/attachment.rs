//! Summaries of text attachments.
//!
//! A PDF name is refused. The PDF crate in the workspace inventory writes
//! pages; it does not extract text, so this module does not pretend to read one.

use crate::{Error, LocalEngine, lookup, render};

/// Summarizes the plain text of an attachment through `engine`.
///
/// # Errors
///
/// [`Error::NotTextAttachment`] when `name` is a PDF.
/// [`Error::UnknownPrompt`] when the attachment template is missing.
/// [`Error::Unscripted`] when the engine has no summary.
pub fn summarize_attachment(
    engine: &impl LocalEngine,
    name: &str,
    text: &str,
) -> Result<String, Error> {
    if is_pdf(name) {
        return Err(Error::NotTextAttachment);
    }
    let template = lookup("summarize-attachment", 1)?;
    engine.generate(&render(template, text))
}

fn is_pdf(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("pdf"))
}

#[cfg(test)]
mod tests {
    use super::summarize_attachment;
    use crate::{Error, ScriptedEngine, lookup, render};

    #[test]
    fn a_text_attachment_is_summarized_by_the_scripted_engine() {
        let text = "The invoice total is 40.";
        let template = lookup("summarize-attachment", 1).unwrap();
        let mut engine = ScriptedEngine::new();
        engine.script_text(render(template, text), "Invoice for 40.");
        assert_eq!(
            summarize_attachment(&engine, "invoice.txt", text).unwrap(),
            "Invoice for 40."
        );
        assert!(matches!(
            summarize_attachment(&engine, "scan.PDF", text),
            Err(Error::NotTextAttachment)
        ));
    }
}
