# Done

Finished tasks moved out of `plan.md`.

### F1. Bootstrap repo apps/mailune with required files

Depends on: nothing. Reuse: ketch & cox AGENTS.md structure; pyrlyn-.github CONTRIBUTING; mise.toml from cox.

Done when: AGENTS.md, README, plan, todo, done, roadmap, ideas, toolchain exist; plan.md table ↔ todo.md in sync. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed (2026-10-08): `apps/mailune` with AGENTS.md (CLAUDE.md symlink), README, plan, todo, done, roadmap, ideas, toolchain, research.md, docs/architecture.md and the cox license set. Local git repository only; no commit and no GitHub repository yet.

### F2. Cargo virtual workspace skeleton (crates/*, resolver 3, workspace lints, dist profile)

Depends on: F1. Reuse: cox root Cargo.toml (workspace.package/dependencies/lints, dist profile).

Done when: Empty mailune-protocol, mailune-core, mailune-app, mailune-cli build; deny unwrap/expect/panic outside tests. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: virtual workspace (`resolver = "3"`, `crates/*`, `default-members` = `mailune-cli`), workspace lints denying `unwrap`/`expect`/`panic` outside tests, lean `dev` profile and `dist` profile. Empty `mailune-protocol`, `mailune-core`, `mailune-app` (one `thiserror` enum each) and `mailune-cli` (`anyhow` only), dependency direction cli → app → core → protocol. `mise.toml` pins Rust 1.99.0. `nextest`, `clippy -D warnings` and `fmt --check` are green.

### F3. Dependency-graph test enforcing crate boundaries

Depends on: F2. Reuse: scull tests/deps.rs; cox cargo-metadata tests.

Done when: Fails if a non-store crate depends on diesel, non-ffi on uniffi, or core on any I/O crate. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: `mailune-cli/tests/deps.rs` reads `cargo metadata` via `cargo_metadata` 0.23.1. Only `mailune-store` may depend on `diesel`, `diesel_migrations` or `libsqlite3-sys`; only `mailune-ffi` may depend on `uniffi`; `mailune-core` may depend only on `mailune-protocol` and `thiserror`.

### F6. Conventions tooling: //! header check, forward-only FFI test scaffold

Depends on: F2. Reuse: cox tests/forward_only.rs (syn).

Done when: CI fails on missing //! header or a multi-expression exported FFI body. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: `mailune-cli/tests/conventions.rs` fails when a `crates/**/*.rs` file does not open with `//!`, and when a `#[uniffi::export]` body is not one expression statement. Fixtures cover both failures. `crates/mailune-ffi/src` is scanned once that crate exists. `syn` 3.0.6.

### F7. mailune-protocol contract types: ids, Envelope, Address, Flags, Mailbox roles, Submission, Event

Depends on: F2. Reuse: NEW (shape from cox-protocol; fields from research/mail-app ui/data.slint).

Done when: serde + schemars; JSON Schema snapshot (insta); no I/O deps. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: `mailune-protocol` types with serde and schemars — opaque string ids, `Address`, `Flags`, `MailboxRole` (the UI folder keys), `Envelope` (headers and flags, not a MIME body), `Submission` (the mock's `Actions`, plus `Summarize`), `Event` (`Notice` and a thread-list `Snapshot`). Sign-in passwords are not a submission field. JSON Schema snapshot is committed. No I/O dependencies.
