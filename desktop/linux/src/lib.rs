//! Linux catalogues and an in-memory host.
//!
//! English is always loaded. A key missing from the selected language uses
//! the English line. This is a plain `key=value` file, not a gettext runtime.
//! The host keeps secrets, banners, the network path, and the OAuth redirect
//! in process. It does not open a socket.

mod host;

pub use host::LinuxHost;

use std::collections::BTreeMap;
use std::path::Path;

/// A catalogue for one language, with English behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    selected: BTreeMap<String, String>,
    english: BTreeMap<String, String>,
}

/// Failure while loading a catalogue. The message does not include the file text.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The language code is not two lowercase letters, or that file is absent.
    #[error("unknown language")]
    Language,
    /// The English file or a selected file could not be read.
    #[error("catalogue could not be read")]
    Read,
}

impl Catalogue {
    /// The line for `key`, or the English line when the selected language omits it.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.selected
            .get(key)
            .or_else(|| self.english.get(key))
            .map(String::as_str)
    }
}

/// Loads `language` from `dir`, keeping `en.txt` as the fallback.
///
/// # Errors
///
/// [`Error::Language`] when `language` is not a two-letter code or its file
/// is missing. [`Error::Read`] when the English file cannot be read.
pub fn load(dir: &Path, language: &str) -> Result<Catalogue, Error> {
    if !language_code(language) {
        return Err(Error::Language);
    }
    let english = read_map(dir, "en")?;
    let selected = if language == "en" {
        english.clone()
    } else {
        read_map(dir, language).map_err(|_| Error::Language)?
    };
    Ok(Catalogue { selected, english })
}

fn language_code(language: &str) -> bool {
    language.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase())
}

fn read_map(dir: &Path, language: &str) -> Result<BTreeMap<String, String>, Error> {
    let text =
        std::fs::read_to_string(dir.join(format!("{language}.txt"))).map_err(|_| Error::Read)?;
    Ok(parse(&text))
}

fn parse(text: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.is_empty() {
            continue;
        }
        entries.insert(key.to_string(), value.to_string());
    }
    entries
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Error, load};

    fn locales() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("locales")
    }

    #[test]
    fn catalogues_load_by_language_and_a_missing_key_is_english() {
        let dir = locales();
        let english = load(&dir, "en").unwrap();
        assert_eq!(english.text("inbox"), Some("Inbox"));
        assert_eq!(english.text("compose"), Some("Compose"));

        let german = load(&dir, "de").unwrap();
        assert_eq!(german.text("inbox"), Some("Posteingang"));
        assert_eq!(german.text("compose"), Some("Compose"));
    }

    #[test]
    fn an_unknown_language_is_rejected() {
        let error = load(&locales(), "zz").unwrap_err();
        assert!(matches!(error, Error::Language));
        let error = load(&locales(), "..").unwrap_err();
        assert!(matches!(error, Error::Language));
    }
}
