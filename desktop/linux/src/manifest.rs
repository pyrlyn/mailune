//! Flatpak manifest description.
//!
//! `share/app.mailune.Mailune.json` names the app id and the binary.
//! The reusable Flatpak workflow is not in this repo, so this file only
//! reads those two fields.

use crate::Error;

/// The app id and the binary a Flatpak manifest names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Flatpak app id.
    pub app_id: String,
    /// Binary the manifest installs.
    pub command: String,
}

/// Reads the app id and the binary from a small JSON object.
///
/// # Errors
///
/// [`Error::Description`] when either field is missing.
pub fn load_manifest(text: &str) -> Result<Manifest, Error> {
    let mut app_id = None;
    let mut command = None;
    for line in text.lines() {
        let line = line.trim().trim_end_matches(',');
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        let value = value.trim().trim_matches('"');
        if value.is_empty() {
            continue;
        }
        match key {
            "app-id" => app_id = Some(value.to_string()),
            "command" => command = Some(value.to_string()),
            _ => {}
        }
    }
    match (app_id, command) {
        (Some(app_id), Some(command)) => Ok(Manifest { app_id, command }),
        _ => Err(Error::Description),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::load_manifest;

    #[test]
    fn the_manifest_names_the_app_id() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/app.mailune.Mailune.json");
        let text = std::fs::read_to_string(path).unwrap();
        let manifest = load_manifest(&text).unwrap();
        assert_eq!(manifest.app_id, "app.mailune.Mailune");
        assert_eq!(manifest.command, "mailune");
    }
}
