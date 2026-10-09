//! Typed Mailune config: the TOML files Mailune owns, their layering, and their JSON Schema.
//!
//! The only crate that reads or writes config, so the rest of the workspace never links
//! `figment` or `toml` (enforced by `mailune-cli/tests/deps.rs`). Secrets are not config: tokens
//! and passwords stay in the OS keychain.
//!
//! - [`load`] — defaults, then the user file, then the project file.
//! - [`schema_json`] — the JSON Schema committed as `schema/config.schema.json`.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod load;
mod schema;

pub use load::{PROJECT_FILE, USER_FILE, load};
pub use schema::schema_json;

/// Listen address when no file sets `listen`.
const DEFAULT_LISTEN: &str = "127.0.0.1:4173";

/// Why config could not be loaded or described.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A present file could not be read.
    #[error("{}: {source}", path.display())]
    Read {
        /// File that could not be read.
        path: PathBuf,
        /// OS error.
        #[source]
        source: std::io::Error,
    },
    /// A file is not valid TOML or does not fit the types: an unknown key, a wrong type, a
    /// missing required field.
    #[error("{}:{line}: {message}", file.display())]
    Parse {
        /// File that was rejected.
        file: PathBuf,
        /// 1-based line the parser points at.
        line: usize,
        /// Parser message. It names the key; config holds no secret it could echo.
        message: String,
    },
    /// The layers parsed one by one but not once merged.
    #[error("merged config: {0}")]
    Merge(String),
    /// The JSON Schema could not be serialized.
    #[error("schema: {0}")]
    Schema(#[from] serde_json::Error),
}

/// Root of the user `config.toml` and the project `.mailune.toml`.
///
/// Every table rejects unknown keys, so a typo is an error at its line instead of a silently
/// ignored setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Address `mailune-server` binds, such as `127.0.0.1:4173`.
    pub listen: String,
    /// Accounts this file names. A later layer that sets `accounts` replaces the list.
    pub accounts: Vec<Account>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: DEFAULT_LISTEN.to_owned(),
            accounts: Vec::new(),
        }
    }
}

/// One mail account. Its credentials live in the keychain under `id`, never here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// Stable account id, also the keychain entry name.
    pub id: String,
    /// Server host name, such as `imap.example.com`.
    pub host: String,
}
