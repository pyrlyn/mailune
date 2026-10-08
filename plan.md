# Mailune

GitHub: not created yet.

A local-first, AI-first mail client: one Rust core (IMAP/SMTP, JMAP, Gmail API, Microsoft Graph, sync, storage and search, AI orchestration, crypto) behind native SwiftUI, WinUI 3, GTK4/Vala and Jetpack Compose shells and a web client. See `research.md` and `docs/architecture.md`.

| # | Status | Priority | Complexity | Readiness | Agent |
| --- | --- | --- | --- | --- | --- |
| C1 | todo | P0 | 2 | 0% | |
| C11 | todo | P0 | 2 | 0% | |
| F4 | todo | P0 | 3 | 0% | |
| F5 | todo | P0 | 2 | 0% | |
| F8 | todo | P0 | 2 | 0% | |
| F9 | todo | P0 | 2 | 0% | |
| F10 | todo | P0 | 2 | 0% | |
| R1 | todo | P0 | 2 | 0% | |
| R15 | todo | P0 | 2 | 0% | |
| R16 | todo | P0 | 2 | 0% | |
| R17 | todo | P0 | 2 | 0% | |
| T1 | todo | P0 | 3 | 0% | |
| X1 | todo | P0 | 3 | 0% | |
| X2 | todo | P0 | 2 | 0% | |
| X3 | todo | P0 | 3 | 0% | |
| X4 | todo | P0 | 2 | 0% | |
| X9 | todo | P0 | 2 | 0% | |
| X10 | todo | P0 | 2 | 0% | |
| X11 | todo | P0 | 2 | 0% | |

### C1. SecretStore integration: tokens, passwords, DB key; Android via host callback

Depends on: F8, X2. Reuse: X2 secret-store; cox no_real_keychain_in_tests.rs guard.

Done when: Secrets never in config or logs (tests); test suite never touches real keychain. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### C11. Threat model and trust boundaries document (core, AI, MCP)

Depends on: F7. Reuse: aulo conventions (untrusted model output, fail closed); cox permission design.

Done when: docs/threat-model.md reviewed by creator. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### F4. mailune-config: typed TOML, layering, committed JSON Schema

Depends on: F2, X1. Reuse: X1 layered-config (from cox-config/src/load.rs, rtok src/config/validate.rs).

Done when: Unknown keys reported with file:line; schema staleness test; only this module imports figment/toml. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### F5. Telemetry: tracing, rotating logs, secret redaction, optional OTLP (off by default)

Depends on: F2, X3. Reuse: X3 telemetry-setup (from aulo-telemetry, cox-telemetry).

Done when: Logs redact tokens (test); no println! in libraries. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### F8. I/O and host traits: Net, Clock, Fs, SecretStore, Notifier, NetworkState, AuthSession, PlatformModel, BackgroundScheduler

Depends on: F7. Reuse: cox-ffi src/host.rs (AppHost); ketch-ffi callbacks.rs.

Done when: Traits documented; fakes exist in testkit (T1). Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### F9. Cancellation tokens and progress reporting

Depends on: F8. Reuse: ketch-core src/cancel.rs; ketch-ffi callbacks.rs.

Done when: A long sync aborts within 100 ms of cancel (test). Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### F10. Gettext catalogs for core-originated strings

Depends on: F2, X4. Reuse: X4 gettext-catalog (from cox-i18n); research/mail-app lang/*.po.

Done when: en + ru + de/fr/ja load; missing key falls back to msgid. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### R1. Core CI: pyrlyn/ci ci-rust.yml matrix + changes.yml + pipeline.yml

Depends on: F2. Reuse: pyrlyn/ci ci-rust.yml, changes.yml, pipeline.yml; packages/crates path-gates.

Done when: Required checks green on PR. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### R15. SonarCloud + coverage (cargo-llvm-cov)

Depends on: R1. Reuse: pyrlyn/ci sonarcloud.yml; rtok justfile coverage.

Done when: Coverage visible; soft-fail without token. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### R16. License check, CLA, commitlint no-agent-attribution

Depends on: R1. Reuse: pyrlyn/ci license-check.yml, cla.yml; ketch commitlint.

Done when: Commit with Co-Authored-By fails. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### R17. Brand entry (pyrlyn/brand brands/mailune) and landing docs/site.md

Depends on: F1. Reuse: apps/brand, apps/landing CONTENT_CONTRACT.md.

Done when: Landing build lists Mailune. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### T1. mailune-testkit: fakes for all host traits + Submission→Event scenario builder

Depends on: F8. Reuse: cox core determinism; test-util feature pattern (rust.md).

Done when: Used by first scenario test in mailune-core. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### X1. Layered TOML config loader in packages/crates (extend config-schema or add layered-config)

Depends on: nothing. Reuse: packages/crates config-schema (aulo S1 T1.14, in flight) + cox-config src/load.rs, rtok src/config/layers.rs; extend config-schema rather than add a second crate.

Done when: Crate published per packages/crates release-plz; cox switched to it; rust.md row. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### X2. Extract keychain secret store (env → keyring, no inline secrets) — secret-store

Depends on: nothing. Reuse: cox-provider-http, runa-cloud src/secrets.rs, aulo-server auth/token.rs.

Done when: One implementation; one consumer migrated; no-real-keychain test helper included. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### X3. Extract telemetry setup with redaction — telemetry-setup

Depends on: nothing. Reuse: aulo-telemetry, cox-telemetry, rtok src/otel.

Done when: aulo or cox migrated; redaction test moves with it. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### X4. Extract gettext catalog loader — gettext-catalog

Depends on: nothing. Reuse: cox-i18n.

Done when: cox migrated; .po fixtures + plural test. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### X9. Extract ABI drift test helper (cbindgen + csbindgen regenerate & diff, BLESS env) — abi-drift

Depends on: nothing. Reuse: scull crates/scull-ffi/tests/bindings.rs; ketch-capi drift test.

Done when: scull uses it; diff shown on drift. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### X10. Consume text-sanitize from packages/crates (aulo S1 T1.11, in flight)

Depends on: nothing. Reuse: packages/crates text-sanitize (from cox-sanitize).

Done when: Mailune uses it for headers and plain text; fuzz target lives with the crate. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### X11. Extract SQLite change feed (PRAGMA data_version poller) — sqlite-change-feed

Depends on: nothing. Reuse: cox-store src/watch.rs.

Done when: Two-connection test sees writes from another process. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.
