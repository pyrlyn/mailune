//! Desktop entry.
//!
//! `share/mailune.desktop` names the app and registers a mailto handler.
//! This file only reads those two lines.

use crate::Error;

/// The name and the mailto handler from a desktop entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopFile {
    /// Application name.
    pub name: String,
    /// True when the entry handles mailto.
    pub mailto: bool,
}

/// Reads the name and the mailto handler from `text`.
///
/// # Errors
///
/// [`Error::Description`] when the name or the mailto handler is missing.
pub fn load_desktop_file(text: &str) -> Result<DesktopFile, Error> {
    let mut name = None;
    let mut mailto = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "Name" if !value.is_empty() => name = Some(value.to_string()),
            "MimeType"
                if value
                    .split(';')
                    .any(|part| part == "x-scheme-handler/mailto") =>
            {
                mailto = true;
            }
            _ => {}
        }
    }
    match name {
        Some(name) if mailto => Ok(DesktopFile { name, mailto }),
        _ => Err(Error::Description),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::load_desktop_file;

    #[test]
    fn the_desktop_file_names_the_app_and_a_mailto_handler() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/mailune.desktop");
        let text = std::fs::read_to_string(path).unwrap();
        let entry = load_desktop_file(&text).unwrap();
        assert_eq!(entry.name, "Mailune");
        assert!(entry.mailto);
    }
}
