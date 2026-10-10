//! Workspace sources must not call the OS keychain or name a remote image URL.
//!
//! Needles are built at runtime so this file does not contain the text it
//! forbids. The scan covers every Rust file under `crates/` and the Swift
//! sources, fixtures and specs of the native shells under `desktop/`.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn keyring_call() -> String {
    format!("{}{}", "keyring", "::")
}

fn security_framework() -> String {
    format!("{}.{}", "Security", "framework")
}

fn schemes() -> [String; 2] {
    ["http", "https"].map(|name| format!("{name}://"))
}

fn image_suffixes() -> [&'static str; 6] {
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg"]
}

fn sources(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        // Build output and generated projects are not sources.
        if name.starts_with('.') || name.starts_with("DerivedData") || name.ends_with(".xcodeproj")
        {
            continue;
        }
        if path.is_dir() {
            sources(&path, extensions, out);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.contains(&ext))
        {
            out.push(path);
        }
    }
}

/// Lines that call the real keychain or spell a remote image URL.
fn violations(source: &str) -> Vec<String> {
    let keyring = keyring_call();
    let security = security_framework();
    let schemes = schemes();
    let mut hits = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let line_no = index + 1;
        if line.contains(&keyring) || line.contains(&security) {
            hits.push(format!("line {line_no}: real keychain"));
        }
        let remote = schemes.iter().any(|scheme| line.contains(scheme));
        let image = image_suffixes().iter().any(|suffix| line.contains(suffix));
        if remote && image {
            hits.push(format!("line {line_no}: remote image url"));
        }
    }
    hits
}

#[test]
fn workspace_sources_do_not_call_the_keychain_or_fetch_remote_images() {
    let root = workspace_root();
    let mut files = Vec::new();
    sources(&root.join("crates"), &["rs"], &mut files);
    assert!(!files.is_empty(), "the workspace has Rust sources");
    sources(
        &root.join("desktop"),
        &["swift", "json", "yml", "sh"],
        &mut files,
    );
    assert!(
        files
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "swift")),
        "the macOS shell has Swift sources"
    );
    let mut found = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        for hit in violations(&source) {
            found.push(format!("{}: {hit}", path.display()));
        }
    }
    assert!(
        found.is_empty(),
        "a source calls the real keychain or names a remote image:\n{}",
        found.join("\n")
    );
}

#[test]
fn scanner_flags_a_planted_keychain_call() {
    let planted = format!(
        "fn load() {{ {}Entry::new(\"app\", \"id\"); }}",
        keyring_call()
    );
    assert!(
        violations(&planted)
            .iter()
            .any(|hit| hit.contains("keychain"))
    );
}

#[test]
fn scanner_flags_security_framework() {
    let planted = format!("let framework = \"{}\";", security_framework());
    assert!(
        violations(&planted)
            .iter()
            .any(|hit| hit.contains("keychain"))
    );
}

#[test]
fn scanner_flags_a_remote_image_url() {
    let planted = format!(
        "let url = \"{}cdn.example/logo{}\";",
        schemes()[1],
        image_suffixes()[0]
    );
    assert!(
        violations(&planted)
            .iter()
            .any(|hit| hit.contains("remote image"))
    );
}

#[test]
fn scanner_allows_a_local_image_name() {
    let planted = format!("let path = \"icons/logo{}\";", image_suffixes()[0]);
    assert!(violations(&planted).is_empty());
}
