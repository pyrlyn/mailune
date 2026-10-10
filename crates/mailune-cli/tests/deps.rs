//! Enforces crate boundaries from `docs/architecture.md` by reading
//! `cargo metadata`, so the check cannot drift from the manifests.
//!
//! Helpers sit outside `#[test]`, so clippy does not treat them as test code
//! even with `allow-unwrap-in-tests`.

#![allow(clippy::unwrap_used, clippy::panic)]

use cargo_metadata::{CargoOpt, Metadata, MetadataCommand};

fn metadata() -> Metadata {
    MetadataCommand::new()
        .features(CargoOpt::AllFeatures)
        .exec()
        .unwrap()
}

fn dep_name(dep: &cargo_metadata::Dependency) -> &str {
    dep.name.as_str()
}

/// Workspace members whose manifest names `dep`, dev-dependencies included.
/// A dev-dependency still links the crate into the test build.
fn crates_depending_on<'a>(meta: &'a Metadata, dep: &str) -> Vec<&'a str> {
    meta.workspace_packages()
        .into_iter()
        .filter(|pkg| pkg.dependencies.iter().any(|d| dep_name(d) == dep))
        .map(|pkg| pkg.name.as_str())
        .collect()
}

fn assert_only_owner(meta: &Metadata, dep: &str, owner: &str) {
    let users = crates_depending_on(meta, dep);
    assert!(
        users.iter().all(|name| *name == owner),
        "{dep} is only allowed in {owner}, found in {users:?}"
    );
}

#[test]
fn only_store_depends_on_diesel() {
    let meta = metadata();
    for dep in ["diesel", "diesel_migrations", "libsqlite3-sys"] {
        assert_only_owner(&meta, dep, "mailune-store");
    }
}

#[test]
fn only_config_depends_on_figment_or_toml() {
    let meta = metadata();
    for dep in ["figment", "toml", "toml_edit"] {
        assert_only_owner(&meta, dep, "mailune-config");
    }
}

#[test]
fn only_ffi_depends_on_uniffi() {
    assert_only_owner(&metadata(), "uniffi", "mailune-ffi");
}

/// `mailune-core` is pure. The allowlist is the contract plus `thiserror`;
/// every other crate is treated as I/O (or as a layer above the domain).
#[test]
fn core_depends_on_no_io_crate() {
    const PURE: &[&str] = &["mailune-protocol", "thiserror"];
    let meta = metadata();
    let core = meta
        .workspace_packages()
        .into_iter()
        .find(|pkg| pkg.name.as_str() == "mailune-core")
        .unwrap();
    let foreign: Vec<&str> = core
        .dependencies
        .iter()
        .map(dep_name)
        .filter(|name| !PURE.contains(name))
        .collect();
    assert!(
        foreign.is_empty(),
        "mailune-core may depend only on {PURE:?}; found {foreign:?}"
    );
}
