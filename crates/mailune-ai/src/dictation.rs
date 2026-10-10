//! Voice dictation for the composer. The host records and hands over
//! 16 kHz mono samples, so the core never opens the microphone. A
//! [`Recognizer`] turns them into segments and [`dictate`] makes composer
//! text of those. The transcript is only ever inserted into the draft the
//! person is editing; it is never sent or run as a command.
//!
//! whisper.cpp (through `whisper-rs`) will implement [`Recognizer`] in its
//! own adapter crate: it is a C++ build, and this crate stays pure and
//! builds for wasm32, where `whisper-rs-sys` cannot compile.

use std::future::Future;
use std::sync::Mutex;

use crate::Error;

/// Sample rate every recognizer takes; whisper's native rate.
pub const SAMPLE_RATE: usize = 16_000;
/// Longest clip, in seconds. Longer dictation is refused, not truncated.
pub const MAX_SECONDS: usize = 120;

/// Speech to text on the device.
pub trait Recognizer: Send + Sync {
    /// Short name for logs. Not a secret.
    fn name(&self) -> &str;

    /// Text segments for `samples` (16 kHz mono, -1 to 1). `language` is
    /// an ISO 639-1 hint; `None` lets the model detect it.
    ///
    /// # Errors
    ///
    /// [`Error::Engine`] when the runtime fails.
    fn transcribe(
        &self,
        samples: &[f32],
        language: Option<&str>,
    ) -> impl Future<Output = Result<Vec<String>, Error>> + Send;
}

/// Composer text for one recorded clip. Silence is an empty string.
///
/// # Errors
///
/// [`Error::OverBudget`] for a clip over [`MAX_SECONDS`],
/// [`Error::Unsupported`] for samples that are not finite, and the
/// recognizer's error.
pub async fn dictate<R: Recognizer>(
    recognizer: &R,
    samples: &[f32],
    language: Option<&str>,
) -> Result<String, Error> {
    if samples.len() > SAMPLE_RATE * MAX_SECONDS {
        return Err(Error::OverBudget);
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(Error::Unsupported);
    }
    if samples.is_empty() {
        return Ok(String::new());
    }
    let segments = recognizer.transcribe(samples, language).await?;
    let words: Vec<&str> = segments
        .iter()
        .map(|segment| segment.trim())
        .filter(|segment| !is_annotation(segment))
        .flat_map(|segment| segment.split(|c: char| c.is_whitespace() || c.is_control()))
        .filter(|word| !word.is_empty())
        .collect();
    Ok(words.join(" "))
}

/// whisper's non-speech markers (`[BLANK_AUDIO]`, `(wind blowing)`): a
/// whole segment in brackets describes the audio, it is not dictation.
fn is_annotation(segment: &str) -> bool {
    (segment.starts_with('[') && segment.ends_with(']'))
        || (segment.starts_with('(') && segment.ends_with(')'))
}

/// A recognizer that replays canned transcripts in order. For tests and a
/// device with no speech model yet.
#[derive(Debug, Default)]
pub struct ScriptedRecognizer {
    transcripts: Mutex<Vec<Vec<String>>>,
    heard: Mutex<Vec<(usize, Option<String>)>>,
}

impl ScriptedRecognizer {
    /// Replays `transcripts` first to last, one per clip.
    pub fn new<T, S>(transcripts: T) -> Self
    where
        T: IntoIterator<Item = S>,
        S: IntoIterator<Item = &'static str>,
    {
        let mut transcripts: Vec<Vec<String>> = transcripts
            .into_iter()
            .map(|segments| segments.into_iter().map(str::to_string).collect())
            .collect();
        transcripts.reverse();
        Self {
            transcripts: Mutex::new(transcripts),
            heard: Mutex::new(Vec::new()),
        }
    }

    /// Sample count and language hint of each clip so far, oldest first.
    pub fn heard(&self) -> Vec<(usize, Option<String>)> {
        self.heard
            .lock()
            .map(|heard| heard.clone())
            .unwrap_or_default()
    }
}

fn poisoned<T>(_: T) -> Error {
    Error::Engine("scripted recognizer state".into())
}

impl Recognizer for ScriptedRecognizer {
    fn name(&self) -> &str {
        "scripted"
    }

    async fn transcribe(
        &self,
        samples: &[f32],
        language: Option<&str>,
    ) -> Result<Vec<String>, Error> {
        self.heard
            .lock()
            .map_err(poisoned)?
            .push((samples.len(), language.map(str::to_string)));
        self.transcripts
            .lock()
            .map_err(poisoned)?
            .pop()
            .ok_or_else(|| Error::Engine("script exhausted".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_SECONDS, Recognizer, SAMPLE_RATE, ScriptedRecognizer, dictate};
    use crate::Error;
    use crate::testing::drive;

    fn second() -> Vec<f32> {
        vec![0.0; SAMPLE_RATE]
    }

    #[test]
    fn a_scripted_recognizer_returns_text_for_the_composer() {
        let recognizer = ScriptedRecognizer::new([
            vec![" Hi Ana,", "[BLANK_AUDIO]", " Friday\tworks\u{7}  for me. "],
            vec!["(wind blowing)"],
        ]);
        assert_eq!(recognizer.name(), "scripted");
        let text = drive(dictate(&recognizer, &second(), Some("en"))).unwrap();
        assert_eq!(text, "Hi Ana, Friday works for me.");
        let silence = drive(dictate(&recognizer, &second(), None)).unwrap();
        assert_eq!(silence, "");
        assert_eq!(
            recognizer.heard(),
            [(SAMPLE_RATE, Some("en".into())), (SAMPLE_RATE, None)]
        );
        assert!(matches!(
            drive(dictate(&recognizer, &second(), None)),
            Err(Error::Engine(_))
        ));
    }

    #[test]
    fn empty_long_and_broken_clips_do_not_reach_the_recognizer() {
        let recognizer = ScriptedRecognizer::new([vec!["unused"]]);
        assert_eq!(drive(dictate(&recognizer, &[], None)).unwrap(), "");
        let long = vec![0.0; SAMPLE_RATE * MAX_SECONDS + 1];
        assert!(matches!(
            drive(dictate(&recognizer, &long, None)),
            Err(Error::OverBudget)
        ));
        assert!(matches!(
            drive(dictate(&recognizer, &[0.1, f32::NAN], None)),
            Err(Error::Unsupported)
        ));
        assert!(recognizer.heard().is_empty());
    }
}
