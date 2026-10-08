//! Reader description.
//!
//! The fixture body and the two safety flags live in `share/reader.desc`.
//! JavaScript stays off and remote content stays blocked. This file only
//! reads that description.

use crate::Error;

/// What the reader shows, and the two switches that stay closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reader {
    /// Fixture body text.
    pub body: String,
    /// JavaScript is off when this is false.
    pub javascript: bool,
    /// Remote content is blocked when this is false.
    pub remote: bool,
    /// Fixture summary. No model is downloaded to produce it.
    pub summary: String,
    /// The one reply chip.
    pub reply_chip: String,
}

/// Reads the body and the two flags from `text`.
///
/// # Errors
///
/// [`Error::Description`] when the body or either flag is missing.
pub fn load_reader(text: &str) -> Result<Reader, Error> {
    let mut body = None;
    let mut javascript = None;
    let mut remote = None;
    let mut summary = None;
    let mut reply_chip = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        match key {
            "body" => body = Some(value.to_string()),
            "javascript" => javascript = Some(value == "on"),
            "remote" => remote = Some(value == "allowed"),
            "summary" => summary = Some(value.to_string()),
            "reply-chip" => reply_chip = Some(value.to_string()),
            _ => {}
        }
    }
    match (body, javascript, remote, summary, reply_chip) {
        (Some(body), Some(javascript), Some(remote), Some(summary), Some(reply_chip))
            if !body.is_empty() && !summary.is_empty() && !reply_chip.is_empty() =>
        {
            Ok(Reader {
                body,
                javascript,
                remote,
                summary,
                reply_chip,
            })
        }
        _ => Err(Error::Description),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::load_reader;

    #[test]
    fn the_reader_shows_the_body_with_javascript_off_and_remote_blocked() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/reader.desc");
        let text = std::fs::read_to_string(path).unwrap();
        let reader = load_reader(&text).unwrap();
        assert_eq!(reader.body, "See you at the dock");
        assert!(!reader.javascript);
        assert!(!reader.remote);
    }

    #[test]
    fn the_reader_includes_a_summary_and_one_reply_chip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/reader.desc");
        let text = std::fs::read_to_string(path).unwrap();
        let reader = load_reader(&text).unwrap();
        assert_eq!(reader.summary, "A short note about the dock");
        assert_eq!(reader.reply_chip, "Thanks");
    }
}
