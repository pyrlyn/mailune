//! UI scenario description.
//!
//! GTK is absent, so this does not drive AT-SPI. The scenario only names
//! the fixture subject in `share/scenario.desc`.

use crate::Error;

/// A scenario the UI would walk if GTK were present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    /// Fixture subject the scenario names.
    pub subject: String,
}

/// Reads the fixture subject from `text`.
///
/// # Errors
///
/// [`Error::Description`] when the subject line is missing.
pub fn load_scenario(text: &str) -> Result<Scenario, Error> {
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
            subject = Some(value.to_string());
        }
    }
    subject
        .map(|subject| Scenario { subject })
        .ok_or(Error::Description)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::load_scenario;

    #[test]
    fn the_scenario_names_a_fixture_subject() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/scenario.desc");
        let text = std::fs::read_to_string(path).unwrap();
        let scenario = load_scenario(&text).unwrap();
        assert_eq!(scenario.subject, "See you at the dock");
    }
}
