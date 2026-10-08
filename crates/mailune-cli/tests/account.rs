//! `account add` and `account list` against a temporary `MAILUNE_HOME`.
//!
//! The child process gets the scratch directory. This test does not set the
//! variable on the parent, and it does not open a socket.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static SCRATCH: AtomicU64 = AtomicU64::new(0);

fn scratch() -> PathBuf {
    let n = SCRATCH.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mailune-cli-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mailune"))
}

#[test]
fn add_and_list_use_the_scratch_home() {
    let home = scratch();
    let add = bin()
        .env("MAILUNE_HOME", &home)
        .args([
            "account",
            "add",
            "--address",
            "ada@example.com",
            "--label",
            "Ada",
        ])
        .output()
        .unwrap();
    assert!(
        add.status.success(),
        "add failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );

    let list = bin()
        .env("MAILUNE_HOME", &home)
        .args(["account", "list"])
        .output()
        .unwrap();
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    assert_eq!(
        String::from_utf8(list.stdout).unwrap(),
        "ada@example.com\tAda\n"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn missing_home_fails_without_touching_a_default_directory() {
    let result = bin()
        .env_remove("MAILUNE_HOME")
        .args(["account", "list"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let err = String::from_utf8(result.stderr).unwrap();
    assert!(err.contains("MAILUNE_HOME"), "{err}");
}
