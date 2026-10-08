//! Typed Mailune config.
//!
//! A shared layered-config package is not in this repo, so this crate loads
//! TOML itself. Defaults sit under the user file, and a project file sits
//! above that. Secrets are not config.

use std::fs;
use std::path::{Path, PathBuf};

use figment::Figment;
use figment::providers::{Format, Serialized, Toml};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Listen address when no file sets `listen`.
const DEFAULT_LISTEN: &str = "127.0.0.1:4173";

/// Failure while reading or checking a config file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be read.
    #[error("{path}: {source}")]
    Read {
        /// File that could not be read.
        path: PathBuf,
        /// OS error.
        #[source]
        source: std::io::Error,
    },
    /// The file is not a TOML table.
    #[error("{file}: {message}")]
    Parse {
        /// File that failed to parse.
        file: PathBuf,
        /// Parser message.
        message: String,
    },
    /// A key is not in the schema.
    #[error("{file}:{line}: unknown key `{key}`")]
    UnknownKey {
        /// File that contains the key.
        file: PathBuf,
        /// 1-based line of the key.
        line: usize,
        /// Key name.
        key: String,
    },
    /// A known key has a value the schema rejects.
    #[error("{message}")]
    Invalid {
        /// Figment message. It names the key, not a secret.
        message: String,
    },
    /// The JSON Schema could not be serialized.
    #[error("schema: {0}")]
    Schema(#[from] serde_json::Error),
}

/// Root of `config.toml` and `.mailune.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Address the server binds. Not a credential.
    pub listen: String,
    /// Accounts named in this file. Tokens stay in the secret store.
    pub accounts: Vec<Account>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: DEFAULT_LISTEN.to_string(),
            accounts: Vec::new(),
        }
    }
}

/// One account. The host is a server name, never a password.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// Stable account id.
    pub id: String,
    /// IMAP or JMAP host.
    pub host: String,
    /// Where mail content may be sent for AI.
    #[serde(default)]
    pub privacy: Privacy,
}

/// Privacy class for one account.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Privacy {
    /// Encrypted or otherwise local-only mail.
    LocalOnly,
    /// Prefer a local model. This is the default.
    #[default]
    LocalPreferred,
    /// A cloud model is allowed when the feature is opted in.
    CloudAllowed,
}

/// Loads defaults, then `user`, then `project` when it is set.
///
/// A missing file is skipped. An unknown key names the file and the line.
///
/// # Errors
///
/// Returns an error when a present file cannot be read, is not TOML, has an
/// unknown key, or has a value the schema rejects.
pub fn load(user: &Path, project: Option<&Path>) -> Result<Config, Error> {
    let mut files = vec![user];
    if let Some(project) = project {
        files.push(project);
    }
    for file in &files {
        if file.is_file() {
            reject_unknown(file)?;
        }
    }
    let mut figment = Figment::from(Serialized::defaults(Config::default()));
    figment = figment.merge(Toml::file(user));
    if let Some(project) = project {
        figment = figment.merge(Toml::file(project));
    }
    figment.extract().map_err(|err| Error::Invalid {
        message: err.to_string(),
    })
}

/// Pretty JSON Schema of [`Config`], with a trailing newline.
///
/// # Errors
///
/// Returns an error when the schema value cannot be serialized.
pub fn schema_json() -> Result<String, Error> {
    let schema = schemars::schema_for!(Config);
    let mut json = serde_json::to_string_pretty(&schema)?;
    if !json.ends_with('\n') {
        json.push('\n');
    }
    Ok(json)
}

fn reject_unknown(file: &Path) -> Result<(), Error> {
    let text = fs::read_to_string(file).map_err(|source| Error::Read {
        path: file.to_path_buf(),
        source,
    })?;
    let value: toml::Value = toml::from_str(&text).map_err(|err| Error::Parse {
        file: file.to_path_buf(),
        message: err.to_string(),
    })?;
    let toml::Value::Table(_) = &value else {
        return Err(Error::Parse {
            file: file.to_path_buf(),
            message: "expected a table".to_string(),
        });
    };
    walk(file, &text, &[], &value)
}

fn walk(file: &Path, text: &str, path: &[String], value: &toml::Value) -> Result<(), Error> {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                if !allowed(path, key) {
                    return Err(Error::UnknownKey {
                        file: file.to_path_buf(),
                        line: line_of(text, key),
                        key: key.clone(),
                    });
                }
                let mut next = path.to_vec();
                next.push(key.clone());
                walk(file, text, &next, child)?;
            }
            Ok(())
        }
        toml::Value::Array(items) => {
            for item in items {
                walk(file, text, path, item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn allowed(path: &[String], key: &str) -> bool {
    match path {
        [] => matches!(key, "listen" | "accounts"),
        [section] if section == "accounts" => matches!(key, "id" | "host" | "privacy"),
        _ => false,
    }
}

fn line_of(text: &str, key: &str) -> usize {
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let head = match trimmed.split_once('=') {
            Some((name, _)) => name.trim(),
            None => trimmed,
        };
        let head = head.trim_matches('"');
        if head == key || head == format!("[{key}]") || head == format!("[[{key}]]") {
            return index + 1;
        }
    }
    1
}

#[cfg(test)]
#[allow(clippy::result_large_err)] // `Jail::expect_with` fixes the closure error to `figment::Error`.
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{Config, Privacy, load, schema_json};

    const COMMITTED_SCHEMA: &str = include_str!("../schema/config.schema.json");

    fn write_file(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory");
        }
        fs::write(path, body).expect("config file");
    }

    #[test]
    fn project_layer_overrides_listen_and_keeps_the_account() {
        figment::Jail::expect_with(|jail| {
            let root = jail.directory();
            write_file(
                root,
                "user/config.toml",
                "listen = \"127.0.0.1:1\"\n\n[[accounts]]\nid = \"ada\"\nhost = \"imap.example\"\n",
            );
            write_file(root, "project/.mailune.toml", "listen = \"127.0.0.1:2\"\n");
            let config = load(
                &root.join("user/config.toml"),
                Some(&root.join("project/.mailune.toml")),
            )
            .map_err(|err| figment::Error::from(err.to_string()))?;
            assert_eq!(config.listen, "127.0.0.1:2");
            assert_eq!(config.accounts.len(), 1);
            assert_eq!(config.accounts[0].id, "ada");
            assert_eq!(config.accounts[0].host, "imap.example");
            assert_eq!(config.accounts[0].privacy, Privacy::LocalPreferred);
            Ok(())
        });
    }

    #[test]
    fn a_missing_file_is_the_default() {
        figment::Jail::expect_with(|jail| {
            let config = load(&jail.directory().join("missing.toml"), None)
                .map_err(|err| figment::Error::from(err.to_string()))?;
            assert_eq!(config, Config::default());
            Ok(())
        });
    }

    #[test]
    fn an_unknown_key_names_the_file_and_line() {
        figment::Jail::expect_with(|jail| {
            let root = jail.directory();
            write_file(
                root,
                "user/config.toml",
                "listen = \"127.0.0.1:9\"\n\nnot_a_field = true\n",
            );
            let file = root.join("user/config.toml");
            let err = load(&file, None).expect_err("unknown key");
            let message = err.to_string();
            assert!(message.contains(&file.display().to_string()), "{message}");
            assert!(message.contains(":3:"), "{message}");
            assert!(message.contains("not_a_field"), "{message}");
            Ok(())
        });
    }

    #[test]
    fn the_committed_schema_matches_the_generated_one() {
        let generated = schema_json().expect("schema");
        assert_eq!(generated, COMMITTED_SCHEMA);
    }

    #[test]
    fn only_this_crate_imports_figment_or_toml() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../");
        let mut hits = Vec::new();
        let entries = fs::read_dir(&crates).expect("crates directory");
        for entry in entries {
            let entry = entry.expect("crate entry");
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) == Some("mailune-config") {
                continue;
            }
            if path.is_dir() {
                scan_tree(&path, &mut hits);
            }
        }
        assert!(
            hits.is_empty(),
            "figment or toml is imported outside mailune-config:\n{}",
            hits.join("\n")
        );
    }

    fn scan_tree(dir: &Path, hits: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                    continue;
                }
                scan_tree(&path, hits);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                let Ok(source) = fs::read_to_string(&path) else {
                    continue;
                };
                for (index, line) in source.lines().enumerate() {
                    if imports_figment_or_toml(line) {
                        hits.push(format!("{}:{}", path.display(), index + 1));
                    }
                }
            } else if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
                let Ok(source) = fs::read_to_string(&path) else {
                    continue;
                };
                for (index, line) in source.lines().enumerate() {
                    if manifest_depends_on_figment_or_toml(line) {
                        hits.push(format!("{}:{}", path.display(), index + 1));
                    }
                }
            }
        }
    }

    fn imports_figment_or_toml(line: &str) -> bool {
        line.contains("figment::")
            || line.contains("use figment")
            || line.contains("toml::")
            || line.contains("use toml;")
            || line.contains("use toml ")
            || line.contains("use toml::")
    }

    fn manifest_depends_on_figment_or_toml(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("figment")
            || trimmed.starts_with("toml ")
            || trimmed.starts_with("toml=")
    }
}
