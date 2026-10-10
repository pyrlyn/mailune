//! Layer loading: each file is checked against the types on its own, then figment merges them.
//!
//! figment errors carry the key path but not the line, so every present file is first parsed
//! alone with `toml`, whose errors keep a byte span. That span becomes `file:line`.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use figment::Figment;
use figment::providers::{Format, Serialized, Toml};

use crate::{Config, Error};

/// User config file name, in the Mailune home directory.
pub const USER_FILE: &str = "config.toml";

/// Project config file name, in a project directory.
pub const PROJECT_FILE: &str = ".mailune.toml";

/// Defaults, then `user`, then `project` when given; a later layer wins key by key.
///
/// A missing file is skipped.
///
/// # Errors
/// [`Error::Read`] when a present file cannot be read, [`Error::Parse`] with the file and line
/// when a file is not TOML or does not fit [`Config`] (an unknown key included), and
/// [`Error::Merge`] when the merged layers do not.
pub fn load(user: &Path, project: Option<&Path>) -> Result<Config, Error> {
    let mut figment = Figment::from(Serialized::defaults(Config::default()));
    for file in std::iter::once(user).chain(project) {
        if let Some(text) = checked_layer(file)? {
            // The text already read is merged, so the file is read once and cannot change
            // between the check and the merge.
            figment = figment.merge(Toml::string(&text));
        }
    }
    figment
        .extract()
        .map_err(|error| Error::Merge(error.to_string()))
}

/// The file's text when it exists and fits [`Config`] on its own.
fn checked_layer(file: &Path) -> Result<Option<String>, Error> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::Read {
                path: file.to_path_buf(),
                source,
            });
        }
    };
    toml::from_str::<Config>(&text).map_err(|error| Error::Parse {
        file: file.to_path_buf(),
        line: line_at(&text, error.span().map_or(0, |span| span.start)),
        message: error.message().to_owned(),
    })?;
    Ok(Some(text))
}

/// 1-based line of byte `offset`, clamped to the text.
fn line_at(text: &str, offset: usize) -> usize {
    let end = offset.min(text.len());
    text.as_bytes()[..end]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

#[cfg(test)]
#[allow(clippy::result_large_err)] // `Jail::expect_with` fixes the closure error to `figment::Error`.
mod tests {
    use std::fs;
    use std::path::Path;

    use figment::Jail;

    use super::{PROJECT_FILE, USER_FILE, line_at, load};
    use crate::{Account, Config, Error};

    fn write(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    fn as_figment(error: Error) -> figment::Error {
        figment::Error::from(error.to_string())
    }

    #[test]
    fn project_layer_overrides_listen_and_keeps_the_user_accounts() {
        Jail::expect_with(|jail| {
            let root = jail.directory();
            write(
                root,
                "home/config.toml",
                "listen = \"127.0.0.1:1\"\n\n[[accounts]]\nid = \"ada\"\nhost = \"imap.example\"\n",
            );
            write(root, "project/.mailune.toml", "listen = \"127.0.0.1:2\"\n");
            let config = load(
                &root.join("home").join(USER_FILE),
                Some(&root.join("project").join(PROJECT_FILE)),
            )
            .map_err(as_figment)?;
            assert_eq!(config.listen, "127.0.0.1:2");
            assert_eq!(
                config.accounts,
                [Account {
                    id: "ada".into(),
                    host: "imap.example".into()
                }]
            );
            Ok(())
        });
    }

    #[test]
    fn missing_files_give_the_defaults() {
        Jail::expect_with(|jail| {
            let root = jail.directory();
            let config =
                load(&root.join("none.toml"), Some(&root.join("nope.toml"))).map_err(as_figment)?;
            assert_eq!(config, Config::default());
            Ok(())
        });
    }

    #[test]
    fn an_unknown_key_names_the_file_and_its_line() {
        Jail::expect_with(|jail| {
            let file = jail.directory().join(USER_FILE);
            fs::write(&file, "listen = \"127.0.0.1:9\"\n\nnot_a_field = true\n").unwrap();
            let error = load(&file, None).unwrap_err();
            let Error::Parse { line, .. } = &error else {
                panic!("expected a parse error, got {error}");
            };
            assert_eq!(*line, 3);
            let message = error.to_string();
            assert!(
                message.starts_with(&format!("{}:3: ", file.display())),
                "{message}"
            );
            assert!(message.contains("not_a_field"), "{message}");
            Ok(())
        });
    }

    #[test]
    fn an_unknown_key_inside_an_account_is_found_in_the_project_layer() {
        Jail::expect_with(|jail| {
            let root = jail.directory();
            write(root, "home/config.toml", "listen = \"127.0.0.1:1\"\n");
            write(
                root,
                "project/.mailune.toml",
                "[[accounts]]\nid = \"ada\"\nhost = \"imap.example\"\npasword = \"typo\"\n",
            );
            let project = root.join("project").join(PROJECT_FILE);
            let error = load(&root.join("home").join(USER_FILE), Some(&project)).unwrap_err();
            let message = error.to_string();
            assert!(
                message.starts_with(&format!("{}:4: ", project.display())),
                "{message}"
            );
            assert!(message.contains("pasword"), "{message}");
            Ok(())
        });
    }

    #[test]
    fn a_value_of_the_wrong_type_names_its_line() {
        Jail::expect_with(|jail| {
            let file = jail.directory().join(USER_FILE);
            fs::write(&file, "# comment\nlisten = 4173\n").unwrap();
            match load(&file, None) {
                Err(Error::Parse { line, .. }) => assert_eq!(line, 2),
                other => panic!("expected a parse error, got {other:?}"),
            }
            Ok(())
        });
    }

    #[test]
    fn line_at_counts_newlines_and_clamps_past_the_end() {
        assert_eq!(line_at("a\nb\nc", 0), 1);
        assert_eq!(line_at("a\nb\nc", 2), 2);
        assert_eq!(line_at("a\nb\nc", 99), 3);
    }
}
