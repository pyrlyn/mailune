//! Accounts recorded under a scratch directory.
//!
//! The directory is `MAILUNE_HOME`. This module never opens a socket and never
//! falls back to the real user home.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "accounts.json";

/// One account stored in the scratch home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Mail address. Not looked up on the network.
    pub address: String,
    /// Display label. Empty when the person did not set one.
    pub label: String,
}

/// `MAILUNE_HOME`, or an error when it is unset or empty.
///
/// # Errors
///
/// Returns an error when the variable is missing or blank.
pub fn home_from_env() -> anyhow::Result<PathBuf> {
    match std::env::var("MAILUNE_HOME") {
        Ok(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => bail!("MAILUNE_HOME is not set"),
    }
}

/// Appends an account. A second add of the same address fails.
///
/// # Errors
///
/// Returns an error when the address is not `local@domain`, the address is
/// already stored, or the scratch file cannot be written.
pub fn add(home: &Path, address: &str, label: &str) -> anyhow::Result<Account> {
    let address = address.trim();
    if !valid_address(address) {
        bail!("address must be a local part, @, and a domain");
    }
    let mut accounts = load(home)?;
    if accounts.iter().any(|account| account.address == address) {
        bail!("account {address} is already stored");
    }
    let account = Account {
        address: address.to_string(),
        label: label.trim().to_string(),
    };
    accounts.push(account.clone());
    store(home, &accounts)?;
    Ok(account)
}

/// Accounts in `home`, in the order they were added. Missing file means none.
///
/// # Errors
///
/// Returns an error when the file exists and is not a JSON array of accounts.
pub fn list(home: &Path) -> anyhow::Result<Vec<Account>> {
    load(home)
}

fn file_path(home: &Path) -> PathBuf {
    home.join(FILE_NAME)
}

fn load(home: &Path) -> anyhow::Result<Vec<Account>> {
    let path = file_path(home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn store(home: &Path, accounts: &[Account]) -> anyhow::Result<()> {
    fs::create_dir_all(home).with_context(|| format!("creating {}", home.display()))?;
    let path = file_path(home);
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(accounts).context("encoding accounts")?;
    fs::write(&tmp, &body).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

fn valid_address(address: &str) -> bool {
    let Some((local, domain)) = address.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !domain.contains('@')
        && !address.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{add, list};

    static SCRATCH: AtomicU64 = AtomicU64::new(0);

    fn scratch() -> PathBuf {
        let n = SCRATCH.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("mailune-account-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn add_then_list_round_trips_through_a_temp_dir() {
        let home = scratch();
        let stored = add(&home, "ada@example.com", "Ada").unwrap();
        assert_eq!(stored.address, "ada@example.com");
        assert_eq!(list(&home).unwrap(), vec![stored]);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn missing_file_lists_nothing() {
        let home = scratch();
        assert!(list(&home).unwrap().is_empty());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn rejects_an_address_without_a_domain() {
        let home = scratch();
        assert!(add(&home, "not-an-address", "").is_err());
        assert!(list(&home).unwrap().is_empty());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn rejects_a_duplicate_address() {
        let home = scratch();
        add(&home, "ada@example.com", "").unwrap();
        assert!(add(&home, "ada@example.com", "again").is_err());
        assert_eq!(list(&home).unwrap().len(), 1);
        let _ = fs::remove_dir_all(&home);
    }
}
