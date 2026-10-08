//! Thread list description.
//!
//! One fixture row lives in `share/thread-list.desc`. This file only reads
//! its subject.

use crate::Error;

/// The one row the list description names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadList {
    /// Subject of the fixture row.
    pub subject: String,
}

/// Reads the fixture subject from `text`.
///
/// # Errors
///
/// [`Error::Description`] when the file does not name exactly one subject.
pub fn load_list(text: &str) -> Result<ThreadList, Error> {
    let mut subject = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        if key == "subject" && !value.is_empty() {
            if subject.is_some() {
                return Err(Error::Description);
            }
            subject = Some(value.to_string());
        }
    }
    subject
        .map(|subject| ThreadList { subject })
        .ok_or(Error::Description)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::load_list;

    #[test]
    fn the_fixture_row_has_a_subject() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/thread-list.desc");
        let text = std::fs::read_to_string(path).unwrap();
        let list = load_list(&text).unwrap();
        assert_eq!(list.subject, "See you at the dock");
    }
}
