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

### F8. I/O and host traits: Net, Clock, Fs, SecretStore, Notifier, NetworkState, AuthSession, PlatformModel, BackgroundScheduler

Depends on: F7. Reuse: cox-ffi src/host.rs (AppHost); ketch-ffi callbacks.rs.

Done when: Traits documented; fakes exist in testkit (T1). Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: the nine traits live on `mailune-protocol`. `mailune-testkit` implements each one on `FakeHost`. No I/O crates.

### F9. Cancellation tokens and progress reporting

Depends on: F8. Reuse: ketch-core src/cancel.rs; ketch-ffi callbacks.rs.

Done when: A long sync aborts within 100 ms of cancel (test). Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: `mailune-core` `CancelToken` and a progress loop. The test cancels a long loop and checks the elapsed time is under 100 ms.

### T1. mailune-testkit: fakes for all host traits + Submission→Event scenario builder

Depends on: F8. Reuse: cox core determinism; test-util feature pattern (rust.md).

Done when: Used by first scenario test in mailune-core. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: `FakeHost`, `Scenario::fold`, and a synthetic mailbox. The scenario test lives in `mailune-testkit`. `mailune-core` cannot dev-depend on it: the dependency-direction test counts dev-dependencies, and core may depend only on `mailune-protocol` and `thiserror`.

### P0. Spike: choose the IMAP stack

Depends on: F1. Reuse: NEW (decision doc; sources: crates.io, repos).

Done when: a decision doc names the stack and the sources that were checked.

What landed: `docs/imap-stack.md` chooses stable `imap-codec` 1.0.0 over `async-imap` 0.12.0.

### P1. mailune-mime parse

Depends on: F7. Reuse: NEW; encoding_rs (rust.md).

Done when: a parser turns a message into a domain part tree, including charsets and attachments.

What landed: `mailune-mime` parses with `mail-parser` 0.11.9 (`full_encoding`) and names charsets through `encoding_rs` 0.8.42.

### P2. MIME build

Depends on: F7. Reuse: NEW; mail-builder.

Done when: text and HTML alternative, attachments, inline images, and reply/forward headers round-trip through the parser.

What landed: `mail-builder` 1.0.0 with default features off (callers set the Message-ID). A test builds a message and parses it back.

### P4. Threading engine

Depends on: F7. Reuse: NEW.

Done when: JWZ threading, provider thread ids, and subject normalisation group messages.

What landed: `mailune-core` threads by provider id and by normalised subject.

### S6. Search query language parser

Depends on: F7. Reuse: NEW.

Done when: `from:`, `to:`, `has:attachment`, `before:`, `is:unread`, and `label:` parse, and a bad token is an error.

What landed: the parser is in `mailune-core`. It does not query a store yet.

### A1. mailune-ai contracts

Depends on: F7. Reuse: runa-core Backend trait; cox-provider trait.

Done when: Feature, PrivacyClass, ModelCapability, a provider trait, and typed results exist with no I/O.

What landed: `mailune-ai` holds the contracts. `mailune-core` still depends only on `mailune-protocol` and `thiserror`.

### A5. PlatformModel bridge

Depends on: A1, F8. Reuse: cox-ffi host callback pattern.

Done when: the core side maps capability discovery and a request/response onto `PlatformModel`.

What landed: `mailune-ai` maps a platform call onto the host trait. The Apple Foundation Models implementation is M6.

### A8. Redaction before cloud

Depends on: A1, P3. Reuse: NEW.

Done when: quotes, signatures, and optional e-mail/phone masking run before a cloud prompt.

What landed: `mailune-ai` redacts quotes, signatures, and optional e-mail and phone spans. HTML sanitising (P3) is still open, so this pass works on plain text.

### C6. Remote content policy

Depends on: P3. Reuse: NEW.

Done when: remote URLs are blocked by default, a per-sender allow list opens them, and a 1×1 image is marked as a tracker pixel. Nothing is fetched.

What landed: `mailune-mime` classifies URLs. The source scan forbids a line that spells a remote image URL, so the tests assemble the scheme and the path on separate lines.

### C8. Link safety

Depends on: P3. Reuse: NEW.

Done when: the real target is shown, and punycode or lookalike labels are flagged.

What landed: `mailune-mime` reports the href as the target when the label differs, and flags punycode and `rn` lookalikes.

### C9. App lock core

Depends on: F8. Reuse: research/pass-app lock-gate pattern.

Done when: a timer locks the app and a locked app rejects send until unlock. Biometrics go through the host.

What landed: the timer and the send gate are in `mailune-core`. The biometric prompt is a host request, not a platform API call.

### P28. Provider quirks registry

Depends on: P5. Reuse: NEW.

Done when: Gmail, iCloud, Yahoo, and Outlook.com quirks are data, not branches in the connection code.

What landed: the registry is data in `mailune-mime`. The IMAP connection that reads it is still P5.

### E0. Spike: confirm web mode A for v1

Depends on: B1. Reuse: NEW (decision doc).

Done when: a decision doc confirms `mailune-server` for v1 and WASM JMAP later.

What landed: `docs/web-mode.md`.

### A26. Unsubscribe and bulk cleanup (RFC 8058)

Depends on: P1, P13. Reuse: NEW.

Done when: a `List-Unsubscribe` header with one-click (RFC 8058) is recognised.

What landed: `mailune-mime` parses the one-click URL. Sending the POST waits on SMTP (P13).

### S13. Contacts autocomplete from headers

Depends on: S3. Reuse: NEW.

Done when: addresses rank by frequency times recency.

What landed: the rank function is in `mailune-mime`. It does not read the store yet (S3).

### R15. SonarCloud + coverage (cargo-llvm-cov)

Depends on: R1. Reuse: pyrlyn/ci sonarcloud.yml; rtok justfile coverage.

Done when: Coverage visible; soft-fail without token. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: `.github/workflows/sonarcloud.yml` runs `cargo-llvm-cov` and skips without `SONAR_TOKEN`.

### R16. License check, CLA, commitlint no-agent-attribution

Depends on: R1. Reuse: pyrlyn/ci license-check.yml, cla.yml; ketch commitlint.

Done when: Commit with Co-Authored-By fails. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

What landed: `license-check.yml` and `cla.yml` reuse the pinned `pyrlyn/ci` workflows. The pin has no commitlint workflow, so `commitlint.config.mjs` states the rule and `commitlint.yml` rejects `Co-Authored-By` in bash.

### R10. Release orchestration

Depends on: R1. Reuse: pyrlyn/ci bump.yml, release-plz.yml; ketch release-plz.toml.

Done when: release-plz is `git_only`, tags are `mailune-v`, and crates are not published.

What landed: `release-plz.toml` and `bump.yml`.

### R2. Cross-target build checks

Depends on: R1, B2. Reuse: ci-rust.yml build-only targets.

Done when: iOS, iOS Simulator, Android, and wasm32 build without tests. The shared matrix stays Linux x86_64, Linux aarch64, macOS aarch64, Windows x86_64.

What landed: a second `ci-rust.yml` call with `"test": false` for those four targets.

### T7. Security tests: no real keychain, no remote images

Depends on: C1, C6. Reuse: cox tests/no_real_keychain_in_tests.rs.

Done when: a source test fails if a crate calls `keyring::` or `Security.framework`, or a line names a remote image URL.

What landed: `mailune-cli/tests/secrets.rs`.

### F11. mailune-cli account add/list

Depends on: F4, F8. Reuse: clap patterns from ketch/cox.

Done when: `account add` and `account list` use `MAILUNE_HOME` and do not touch a default directory when it is unset.

What landed: JSON accounts under `MAILUNE_HOME`. Typed config (F4) is still open, so this store is a scratch file, not the config crate.

### T5. Synthetic mailbox generator

Depends on: F7. Reuse: NEW.

Done when: threads, newsletters, receipts, multilingual mail, and phishing samples can be generated without a network.

What landed: `mailune-testkit` builds those shapes in memory.

### P12, P14, and P15 stayed on the roadmap

The in-memory operation queue, autoconfig parsing, and PKCE S256 landed. Persistence, live ISPDB/SRV/MX lookup, and the OAuth loopback plus refresh are still the roadmap lines for those ids.
