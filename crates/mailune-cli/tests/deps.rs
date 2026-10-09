//! Enforces crate boundaries from `docs/architecture.md` by reading
//! `cargo metadata`, so the check cannot drift from the manifests.
//!
//! Helpers sit outside `#[test]`, so clippy does not treat them as test code
//! even with `allow-unwrap-in-tests`.

#![allow(clippy::unwrap_used, clippy::panic)]

use cargo_metadata::{CargoOpt, DependencyKind, Metadata, MetadataCommand};

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
fn only_ffi_depends_on_uniffi() {
    assert_only_owner(&metadata(), "uniffi", "mailune-ffi");
}

/// Two crates serve HTTP: the self-hosted server and the push relay. Nothing
/// else listens on a socket, so axum stays out of every other crate.
#[test]
fn only_the_servers_depend_on_axum() {
    let meta = metadata();
    let users = crates_depending_on(&meta, "axum");
    assert!(
        users
            .iter()
            .all(|name| ["mailune-server", "mailune-push"].contains(name)),
        "axum is only allowed in mailune-server and mailune-push, found in {users:?}"
    );
}

/// `mailune-core` is pure. The allowlist is the contract plus `thiserror`;
/// every other crate is treated as I/O (or as a layer above the domain).
/// `proptest` may appear as a dev-dependency only: it generates inputs in
/// memory and never ships in the domain.
#[test]
fn core_depends_on_no_io_crate() {
    const PURE: &[&str] = &["mailune-protocol", "thiserror"];
    const TEST_ONLY: &[&str] = &["proptest"];
    let meta = metadata();
    let core = meta
        .workspace_packages()
        .into_iter()
        .find(|pkg| pkg.name.as_str() == "mailune-core")
        .unwrap();
    let foreign: Vec<&str> = core
        .dependencies
        .iter()
        .filter(|dep| {
            !(dep.kind == DependencyKind::Development && TEST_ONLY.contains(&dep_name(dep)))
        })
        .map(dep_name)
        .filter(|name| !PURE.contains(name))
        .collect();
    assert!(
        foreign.is_empty(),
        "mailune-core may depend only on {PURE:?}; found {foreign:?}"
    );
}
