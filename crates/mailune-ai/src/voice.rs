//! Speech for the composer.
//!
//! [`ScriptedRecognizer`] returns a fixed phrase and never opens a microphone.
//! [`dictate_whisper`] is the local model path. It reads a model file the
//! caller already has; this module does not download one.

use std::path::Path;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::Error;

/// Turns a PCM buffer into text for the composer.
pub trait Recognizer {
    /// Recognize `samples`. 16 kHz mono `f32` when a real model is used.
    ///
    /// # Errors
    ///
    /// [`Error::DictateFailed`] when recognition cannot produce text.
    fn recognize(&self, samples: &[f32]) -> Result<String, Error>;
}

/// A recognizer whose text is fixed. Tests use this so they never load a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedRecognizer {
    text: String,
}

impl ScriptedRecognizer {
    /// `text` is what the composer will receive.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Recognizer for ScriptedRecognizer {
    fn recognize(&self, _samples: &[f32]) -> Result<String, Error> {
        // The script stands in for a model. The buffer is intentionally ignored.
        Ok(self.text.clone())
    }
}

/// Text the composer should insert.
///
/// # Errors
///
/// [`Error::DictateFailed`] when `recognizer` fails.
pub fn dictate(recognizer: &impl Recognizer, samples: &[f32]) -> Result<String, Error> {
    recognizer.recognize(samples)
}

/// Runs whisper.cpp on `samples` using a model file at `model`.
///
/// `samples` are 16 kHz mono `f32`. An empty buffer is refused before the
/// library sees it: whisper.cpp can fault on that input.
///
/// # Errors
///
/// [`Error::DictateFailed`] when the model cannot be loaded or the run fails.
/// The error does not include the model path or the audio.
pub fn dictate_whisper(model: &Path, samples: &[f32]) -> Result<String, Error> {
    if samples.is_empty() {
        return Err(Error::DictateFailed);
    }
    let context = WhisperContext::new_with_params(model, WhisperContextParameters::default())
        .map_err(|_| Error::DictateFailed)?;
    let mut state = context.create_state().map_err(|_| Error::DictateFailed)?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    state
        .full(params, samples)
        .map_err(|_| Error::DictateFailed)?;
    let mut text = String::new();
    for segment in state.as_iter() {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&segment.to_string());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::{ScriptedRecognizer, dictate};

    #[test]
    fn a_scripted_recognizer_returns_text_for_the_composer() {
        let recognizer = ScriptedRecognizer::new("See you Thursday");
        let text = dictate(&recognizer, &[]).unwrap();
        assert_eq!(text, "See you Thursday");
    }
}
