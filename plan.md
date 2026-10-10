# Mailune

GitHub: https://github.com/pyrlyn/mailune

A local-first, AI-first mail client: one Rust core (IMAP/SMTP, JMAP, Gmail API, Microsoft Graph, sync, storage and search, AI orchestration, crypto) behind native SwiftUI, WinUI 3, GTK4/Vala and Jetpack Compose shells and a web client. See `research.md` and `docs/architecture.md`.

| # | Status | Priority | Complexity | Readiness | Agent |
| --- | --- | --- | --- | --- | --- |
| C1 | todo | P0 | 2 | 40% | |
| C11 | in progress | P0 | 2 | 90% | Cursor / claude-opus-5.5 |
| F5 | todo | P0 | 2 | 0% | |
| F10 | todo | P0 | 2 | 0% | |
| R17 | in progress | P0 | 2 | 90% | Cursor / claude-opus-5.5 |
| X1 | todo | P0 | 3 | 0% | |
| X2 | in progress | P0 | 2 | 60% | Cursor / claude-opus-5.5 |
| X3 | in progress | P0 | 3 | 70% | Cursor / claude-opus-5.5 |
| X4 | in progress | P0 | 2 | 60% | Cursor / claude-opus-5.5 |
| X9 | in progress | P0 | 2 | 60% | Cursor / claude-opus-5.5 |
| X10 | todo | P0 | 2 | 0% | |
| X11 | in progress | P0 | 2 | 80% | Cursor / claude-opus-5.5 |
| C3 | todo | P1 | 3 | 0% | |
| C4 | todo | P1 | 3 | 0% | |
| C5 | todo | P1 | 3 | 0% | |
| C7 | todo | P1 | 2 | 0% | |
| T4 | todo | P1 | 3 | 0% | |
| E1 | todo | P1 | 3 | 0% | |
| B2 | todo | P1 | 3 | 0% | |
| B3 | todo | P1 | 2 | 0% | |
| M5 | todo | P1 | 3 | 0% | |
| M6 | todo | P1 | 3 | 0% | |
| T6 | in progress | P1 | 2 | 90% | Cursor / claude-opus-5.5 |
| R8 | todo | P1 | 3 | 0% | |
| E12 | todo | P1 | 2 | 0% | |
| B6 | todo | P1 | 3 | 0% | |
| L1 | todo | P1 | 2 | 0% | |
| L2 | todo | P1 | 3 | 0% | |
| L3 | todo | P1 | 2 | 0% | |
| L4 | todo | P1 | 2 | 0% | |
| R6 | todo | P1 | 2 | 0% | |
| L5 | todo | P1 | 3 | 0% | |
| L6 | todo | P1 | 3 | 0% | |
| L7 | todo | P1 | 3 | 0% | |
| L8 | todo | P1 | 3 | 0% | |
| L9 | todo | P1 | 3 | 0% | |
| L10 | todo | P1 | 3 | 0% | |
| L11 | todo | P1 | 2 | 0% | |
| L12 | todo | P1 | 2 | 0% | |
| L13 | todo | P1 | 2 | 0% | |
| I2 | todo | P1 | 3 | 0% | |
| I6 | todo | P1 | 2 | 0% | |
| I7 | todo | P1 | 3 | 0% | |
| B5 | todo | P1 | 2 | 0% | |
| W1 | todo | P1 | 2 | 0% | |
| W2 | todo | P1 | 2 | 0% | |
| W3 | todo | P1 | 2 | 0% | |
| W4 | todo | P1 | 3 | 0% | |
| W5 | todo | P1 | 3 | 0% | |
| W6 | todo | P1 | 3 | 0% | |
| W7 | todo | P1 | 3 | 0% | |
| W8 | todo | P1 | 3 | 0% | |
| W9 | todo | P1 | 3 | 0% | |
| W10 | todo | P1 | 3 | 0% | |
| W11 | todo | P1 | 2 | 0% | |
| W12 | todo | P1 | 2 | 0% | |
| R4 | todo | P1 | 2 | 0% | |
| B4 | todo | P1 | 3 | 0% | |
| D1 | todo | P1 | 2 | 0% | |
| D2 | todo | P1 | 2 | 0% | |
| D3 | todo | P1 | 2 | 0% | |
| D4 | todo | P1 | 3 | 0% | |
| D6 | todo | P1 | 3 | 0% | |
| D7 | todo | P1 | 3 | 0% | |
| D8 | todo | P1 | 3 | 0% | |
| D9 | todo | P1 | 3 | 0% | |
| D10 | todo | P1 | 3 | 0% | |
| D5 | todo | P1 | 3 | 0% | |
| D11 | todo | P1 | 2 | 0% | |
| D12 | todo | P1 | 3 | 0% | |
| R5 | todo | P1 | 2 | 0% | |
| R18 | todo | P1 | 2 | 0% | |
| X5 | todo | P0 | 2 | 0% | |
| X6 | todo | P0 | 2 | 0% | |
| X7 | todo | P0 | 2 | 0% | |
| X8 | todo | P0 | 3 | 0% | |
| D13 | todo | P1 | 3 | 0% | |
| I8 | todo | P1 | 3 | 60% | |
| W13 | todo | P1 | 3 | 0% | |
| B10 | todo | P3 | 3 | 0% | |
| B11 | todo | P3 | 4 | 0% | |
| P33 | todo | P3 | 4 | 0% | |
| P34 | todo | P3 | 5 | 0% | |

### C1. SecretStore integration: tokens, passwords, DB key; Android via host callback

Depends on: F8, X2. Reuse: X2 secret-store; cox no_real_keychain_in_tests.rs guard.

Done when: Secrets never in config or logs (tests); test suite never touches real keychain. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Already landed: `mailune-app` reads a bearer token through `SecretStore` and logs `Secret(redacted)` only. Passwords, the database key, and the Android host callback are still open. `X2` (the shared keychain crate) is still open.

Execution plan: `mailune-app` only. Commit on `batch8-ai`. Use the existing `SecretStore` get/put. Do not add `keyring` and do not edit `mailune-protocol` unless a callback cannot be expressed. Tests use a fake store.

### C11. Threat model and trust boundaries document (core, AI, MCP)

Depends on: F7. Reuse: aulo conventions (untrusted model output, fail closed); cox permission design.

Done when: docs/threat-model.md reviewed by creator. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: Write `docs/threat-model.md` for the core, AI pipeline, MCP, and the push relay. Trust boundaries follow the architecture (untrusted mail, model output, MCP clients; fail closed; encrypted mail never leaves the device for a cloud model). Reuse the shape of aulo's untrusted-output rules and cox's `SECURITY.md` guard list: one table of assets, one of boundaries, and the guard that holds each one, naming the code that implements it. Link to the security sections of `docs/architecture.md` instead of restating them. Do not edit other tasks' files. Verify: every guard named in the document exists in the tree.

Progress: `docs/threat-model.md` now covers the push relay and speech as well, links to the Security section of `docs/architecture.md` instead of restating it, and has a guard table naming the function or test that holds each rule (every name checked against the tree). A known-gaps section lists what the tree does not hold yet: SQLCipher only on Apple targets, no header escape/bidi cleaning until X10, fuzz targets for three parsers only, relay channel ids that can trigger wakes, and empty wakes that sync every relay account on a device (a creator decision).

Left: creator review.

### F5. Telemetry: tracing, rotating logs, secret redaction, optional OTLP (off by default)

Depends on: F2, X3. Reuse: X3 telemetry-setup (from aulo-telemetry, cox-telemetry).

Done when: Logs redact tokens (test); no println! in libraries. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: new crate `mailune-telemetry`. Redaction only. Do not add an OTLP exporter. `X3` is not in this repo.

### F10. Gettext catalogs for core-originated strings

Depends on: F2, X4. Reuse: X4 gettext-catalog (from cox-i18n); research/mail-app lang/*.po.

Done when: en + ru + de/fr/ja load; missing key falls back to msgid. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: catalogs plus a loader in `mailune-app`. Do not edit `mailune-core`. `X4` is not in this repo.

### R17. Brand entry (pyrlyn/brand brands/mailune) and landing docs/site.md

Depends on: F1. Reuse: apps/brand, apps/landing CONTENT_CONTRACT.md.

Done when: Landing build lists Mailune. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Already landed: `brands/mailune` (tokens, logos, `dist/`) is on pyrlyn/brand `main` (pyrlyn/brand#4).

Execution plan:
1. Here: rewrite `docs/site.md` to the landing's `CONTENT_CONTRACT.md` (front matter; Overview, Features, Install, Usage examples, Links), with facts from `README.md` and the `mailune` CLI only, and accents from the brand tokens.
2. pyrlyn/landing: `content/projects/mailune.md` as a copy of that file, the theme, status, order, icons and glyph entries the runa page added (pyrlyn/landing#15), and hero, prop and share art generated by `art/gen.mjs` and `art/og.mjs`.
3. Check: the landing's `npm run build`, `check`, `check:docs`, `check:seo` and `test` pass and the catalog lists Mailune.

Progress: `docs/site.md` follows the content contract, and its two usage commands were run against the `mailune` binary. pyrlyn/landing#24 adds the page; its build lists Mailune at `/landing/mailune/`, and `check`, `check:docs` and `test` pass. `check:seo` fails on landing `main` too, on duplicate titles in the synced ketch docs, not on this page.

Left: merge pyrlyn/landing#24. Mailune has no `sync-docs.yml` (it needs the landing deploy key as `SITE_DEPLOY_KEY`), so later edits to `docs/site.md` are copied to the landing by hand until one is added.

### X1. Layered TOML config loader in packages/crates (extend config-schema or add layered-config)

Depends on: nothing. Reuse: packages/crates config-schema (aulo S1 T1.14, in flight) + cox-config src/load.rs, rtok src/config/layers.rs; extend config-schema rather than add a second crate.

Done when: Crate published per packages/crates release-plz; cox switched to it; rust.md row. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: `packages/crates` worktree, on `batch9-store`'s agent. Commit there. Do not push. Do not edit Mailune plan files.

### X2. Extract keychain secret store (env → keyring, no inline secrets) — secret-store

Depends on: nothing. Reuse: cox-provider-http, runa-cloud src/secrets.rs, aulo-server auth/token.rs.

Done when: One implementation; one consumer migrated; no-real-keychain test helper included. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: a new crate in pyrlyn/crates-packages, one pull request. The crates.io name `secret-store` belongs to another owner, so the crate is `keychain-secret` (404 on crates.io, 2026-10-10). One implementation from runa `secrets.rs`, cox `resolve_key_with`, and aulo's token store: a redacted `Secret`, a store trait with keychain, memory and no-op stores, env-then-store resolution, the keychain switch, the inline-secret check, and cox's no-real-keychain guard as a reusable scanner. Gates and `cargo publish --dry-run` run locally. The consumer migration waits for the crates.io release through `bump.yml`.

Progress: `keychain-secret` is in pyrlyn/crates-packages#47 with tests that never touch the keychain. Local gates and `cargo publish --dry-run` pass.

Left: merge it; the creator enables GitHub Actions on pyrlyn/crates-packages (disabled since 2026-09-25, so neither CI nor `bump.yml` runs) and adds `keychain-secret` to the `CARGO_REGISTRY_TOKEN` scope on crates.io; the coordinator runs `bump.yml` with `package=keychain-secret`. Then one consumer (runa `secrets.rs`, or this repo's C1) moves onto the registry version with a local `paths` override.

### X3. Extract telemetry setup with redaction — telemetry-setup

Depends on: nothing. Reuse: aulo-telemetry, cox-telemetry, rtok src/otel.

Done when: aulo or cox migrated; redaction test moves with it. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: `packages/crates` worktree, crate `telemetry-setup` (T21 there), extracted from `aulo-telemetry` with its redaction and logging tests; the application name, filter variable and extra token patterns become settings. Then aulo moves onto it in its own pull request.

Progress: the crate is in pyrlyn/crates-packages#43 with aulo's redaction tests. `cox-telemetry` does no redaction, and rtok `src/otel` has nothing to share. On 2026-10-10 the package was trimmed to `src`, `tests` and `README.md`; local gates, the `otlp` tests, Rust 1.98 and `cargo publish --dry-run` pass.

Left: merge it; the creator enables GitHub Actions on pyrlyn/crates-packages (disabled since 2026-09-25, so neither CI nor `bump.yml` runs) and adds `telemetry-setup` to the `CARGO_REGISTRY_TOKEN` scope on crates.io; the coordinator runs `bump.yml` with `package=telemetry-setup`. Then aulo migrates: a shared crate is consumed as a registry version with a local `paths` override, never a bare path.

### X4. Extract gettext catalog loader — gettext-catalog

Depends on: nothing. Reuse: cox-i18n.

Done when: cox migrated; .po fixtures + plural test. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: a new crate `gettext-catalog` in pyrlyn/crates-packages, one pull request, split there into two tasks under the line cap: catalog, plural rules and placeholders; then the localizer. Locales, the default locale and constants such as `{brand}` are parameters. `.po` fixtures for en, ru and uk with plural tests. Gates and `cargo publish --dry-run` run locally. The cox migration waits for the crates.io release through `bump.yml`.

Progress: `gettext-catalog` is in pyrlyn/crates-packages#44 (tasks T22.1 and T22.2 there) with en, ru and uk `.po` fixtures and plural tests. Local gates and `cargo publish --dry-run` pass.

Left: merge it; the creator enables GitHub Actions on pyrlyn/crates-packages (disabled since 2026-09-25, so neither CI nor `bump.yml` runs) and adds `gettext-catalog` to the `CARGO_REGISTRY_TOKEN` scope on crates.io; the coordinator runs `bump.yml` with `package=gettext-catalog`. Then cox moves `cox-i18n` onto the registry version.

### X9. Extract ABI drift test helper (cbindgen + csbindgen regenerate & diff, BLESS env) — abi-drift

Depends on: nothing. Reuse: scull crates/scull-ffi/tests/bindings.rs; ketch-capi drift test.

Done when: scull uses it; diff shown on drift. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: a new crate `abi-drift` in pyrlyn/crates-packages, one pull request: a drift checker (unified diff on drift, bless from an environment variable) plus cbindgen and csbindgen render helpers. A fixture FFI crate carries a committed header and C# file. Gates and `cargo publish --dry-run` run locally. The scull migration waits for the crates.io release through `bump.yml`.

Progress: `abi-drift` is in pyrlyn/crates-packages#46; a stale fixture header fails with a unified diff. Local gates, `cargo publish --dry-run` and Rust 1.86 pass.

Left: merge it; the creator enables GitHub Actions on pyrlyn/crates-packages (disabled since 2026-09-25, so neither CI nor `bump.yml` runs) and adds `abi-drift` to the `CARGO_REGISTRY_TOKEN` scope on crates.io; the coordinator runs `bump.yml` with `package=abi-drift`. Then scull's `crates/scull-ffi/tests/bindings.rs` moves onto the registry version.

### X10. Consume text-sanitize from packages/crates (aulo S1 T1.11, in flight)

Depends on: nothing. Reuse: packages/crates text-sanitize (from cox-sanitize).

Done when: Mailune uses it for headers and plain text; fuzz target lives with the crate. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Already landed: `docs/text-sanitize.md` records that the crate is not on crates.io and not under `packages/crates` yet, so Mailune does not depend on it.

Execution plan: `packages/crates` worktree. Add `text-sanitize` if it is missing. Commit there. Do not push.

### X11. Extract SQLite change feed (PRAGMA data_version poller) — sqlite-change-feed

Depends on: nothing. Reuse: cox-store src/watch.rs.

Done when: Two-connection test sees writes from another process. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: a new crate `sqlite-change-feed` in pyrlyn/crates-packages, one pull request: `ChangeToken` over a Diesel `SqliteConnection` polling `PRAGMA data_version`. The cross-process test re-executes the test binary as a writer. Gates and `cargo publish --dry-run` run locally.

Progress: `sqlite-change-feed` is in pyrlyn/crates-packages#45. Its test spawns a writer process and sees the commit exactly once, and a second connection in the same process is covered too. Local gates, nextest and `cargo publish --dry-run` pass.

Left: merge it; the creator enables GitHub Actions on pyrlyn/crates-packages (disabled since 2026-09-25, so neither CI nor `bump.yml` runs) and adds `sqlite-change-feed` to the `CARGO_REGISTRY_TOKEN` scope on crates.io; the coordinator runs `bump.yml` with `package=sqlite-change-feed`.

### C3. Autocrypt headers

Depends on: C2. Reuse: the OpenPGP key type only if `mailune-crypto` can be called without editing it. Prefer a header codec in `mailune-mime`.

Done when: an Autocrypt header is parsed and gossip keys are collected from a message. No WKD network lookup.

Execution plan: `mailune-mime` only.

### C4. S/MIME verify and decrypt

Depends on: P1. Reuse: a maintained `cms` and `x509-cert` if they fit.

Done when: a fixture verifies a signature and decrypts with a supplied key. No keychain.

Execution plan: new crate `mailune-smime`.

### C5. S/MIME sign and encrypt

Depends on: C4. Reuse: the same crate.

Done when: a fixture signs and encrypts, and C4 verifies and decrypts it.

Execution plan: `mailune-smime` only.

### C7. DKIM ed25519, simple, and l=

Depends on: the rsa-sha256 relaxed/relaxed verifier already in `mailune-mime`.

Done when: ed25519, simple canonicalization, and the `l=` body-length tag verify against a supplied TXT record. No network.

Execution plan: `mailune-mime` `auth.rs` only. Extend the existing verifier.

### T4. Queue property tests

Depends on: the in-memory queue. Reuse: `proptest` from rust.md.

Done when: random ops against a model mailbox keep idempotency and undo invariants.

Execution plan: tests in `mailune-core` next to the queue. Do not add a production dependency.

### E1. mailune-server

Depends on: B8. Reuse: `axum` from `rust.md`.

Done when: one WebSocket JSON-RPC method from `mailune-rpc` answers on a bound ephemeral port in a test. Auth token is checked. No passkey yet if it needs a crate that is not already in the tree; say so in the commit.

Execution plan: new binary crate `mailune-server`. `anyhow` is allowed. Do not rewrite `mailune-rpc`.

### B2. UniFFI records

Depends on: B1, F6. Reuse: ketch-ffi and cox-ffi.

Done when: records, errors, one async function, and a callback interface compile, and each export forwards one call.

Execution plan: new crate `mailune-ffi`. Commit on `batch7-imap`. Read `crates/mailune-cli/tests/conventions.rs` before writing an export.

### B3. Swift package

Depends on: B2.

Done when: an XCFramework script targets macOS arm64 only, and a SwiftPM package named MailuneCore builds that slice. No Intel slice.

Execution plan: `desktop/macos` packaging only.

### M5. macOS host integrations

Depends on: M1, B2.

Done when: Keychain, notifications, network path, web auth, and open-URL sit behind protocols with fakes. Tests do not touch the real keychain. No source line contains `keyring::` or `Security.framework`.

Execution plan: `desktop/macos` only.

### T6. Performance budgets

Depends on: S12, B1. Reuse: `divan` and `hyperfine` if already listed.

Done when: cold open, a list page, and a search have a budget. `nextest` checks a small mailbox against a loose ceiling. The 100k run stays in the divan bench.

Execution plan: `mailune-store` benches and one test, on the S14 branch because the search being budgeted is the cached vector scan.

- Dependencies: B1 is done. S12 (storage benches) is not on `main`, so this task brings the 100k list-page and search benches itself; S12's insert bench is still open. S5 (FTS) is not on `main` either, so "a search" is `Store::nearest` until it lands.
- Budgets go in `docs/architecture.md`. Only the 50 ms list page at 100k is stated there; cold open and search get proposed numbers, flagged for the creator.
- `benches/budgets.rs` (divan) times them on a 100k fixture, seeded once into Cargo's target temp directory and reused. `tests/budgets.rs` runs a 300-message mailbox against ten times each budget, which holds even in a debug build. Both share `tests/support`.

Proposed budgets and why. The thresholds are from Nielsen Norman Group, "Response Times: The 3 Important Limits" (https://www.nngroup.com/articles/response-times-3-important-limits/): about 0.1 s feels instant, and about 1 s keeps the user's flow.

- **Search: 100 ms.** A typed query answered within the "instant" limit. It is looser than a list page, which has to keep up with scrolling, page after page.
- **Cold open (open the store and show the first page): 300 ms.** The store's share of a launch that should feel under a second; the rest is the native shell's.

Measured (Apple M3 Max, release, SQLCipher, 100k messages with 384 dimensions, medians):

| Measurement | Median | Budget |
| --- | --- | --- |
| Cold open to first page | 258 ms | 300 ms, met |
| List page | 220 ms | 50 ms, **missed**; the time is in `thread_page`'s query, not the vectors |
| Search, cached | 71 ms | 100 ms, met |
| First search after open, which fills the cache | 1.31 s | not budgeted |

Fixes for the list page and the first search are proposed in `ideas.md`.

Left: creator confirmation of the two proposed budgets.

### R8. Integration compose file

Depends on: the IMAP client already on this branch. P7's sync code is on another branch.

Done when: a compose file names Stalwart and Dovecot, and a test reads that file. `nextest` does not start Docker and does not open a socket. Do not edit `.github/workflows/ci.yml`.

Execution plan: `docker-compose.yml` plus one test in `mailune-cli`.

### E12. Server container

Depends on: E1. Another agent owns `docker-compose.yml`.

Done when: a Dockerfile builds the server binary on paper (the file exists and names the binary) and `docs/self-host.md` says how to run it. Do not start Docker in tests. Do not edit `docker-compose.yml`.

Execution plan: `Dockerfile` and `docs/self-host.md` only.

### B6. C ABI

Depends on: B1. The shared abi-drift crate is not in this repo.

Done when: a cbindgen header, a VAPI, and a meson file exist, and a test fails if the header drifts from the Rust records. Each export forwards one call.

Execution plan: new crate `mailune-capi`. Commit on `batch9-store`. Do not create a package outside this repo.

### M6. Apple on-device model

Depends on: M5, A5.

Done when: a prompt returns JSON from a scripted model. If the Foundation Models framework is missing, keep the protocol and say so in the commit.

Execution plan: `desktop/macos` only.

### L1. C library build

Depends on: B6. GTK 4 and `valac` are not installed on this machine.

Done when: meson builds the C library from `mailune-capi` and a C test links it. A Vala file is in the tree. The GTK target stays off, and the commit says `valac` and GTK 4 were absent. Do not install them.

Execution plan: `desktop/linux` only. Commit on `batch9-store`.

### L2. Payload schema

Depends on: L1.

Done when: a JSON schema lists the C record fields, and a Rust test fails if a field is missing. A Vala decoder file is checked in and not compiled.

Execution plan: `mailune-capi` and `desktop/linux` only.

### L3. Linux tokens

Depends on: L1. GTK 4 is not installed.

Done when: a CSS file defines named colours, type, and spacing, and one local icon is an SVG. A test reads the colour names. No remote image URL.

Execution plan: `desktop/linux` only. Commit on `batch9-store`. Do not install GTK.

### L4. Linux catalogues

Depends on: L1.

Done when: English and one other catalogue load by language, and a missing key falls back to English. A Rust test covers the fallback. Do not add the `gettext` crate.

Execution plan: `desktop/linux` only.

### R6. Linux CI

Depends on: L1.

Done when: a new workflow runs the meson C test. Vala stays off. Do not edit `.github/workflows/ci.yml` and do not change required checks.

Execution plan: `.github/workflows/linux.yml` only.

### L5. Linux host

Depends on: L2. GTK 4 and `valac` are not installed.

Done when: secrets, notifications, network path, and an OAuth redirect sit behind fakes. A test covers each. No `keyring` crate. No source line contains `keyring::` or `Security.framework`.

Execution plan: `desktop/linux` only. Commit on `batch9-store`. Do not install GTK.

### L6. Linux shell

Depends on: L3.

Done when: a UI description names three panes and a breakpoint. A test reads those names. The GTK target stays off.

Execution plan: `desktop/linux` only.

### L7. Linux thread list

Depends on: L6, L2.

Done when: a list description has one fixture row with a subject, and a test reads that subject.

Execution plan: `desktop/linux` only.

### I2. iOS host

Depends on: I1, M5.

Done when: keychain, background refresh, and notifications sit behind fakes. No source line contains `keyring::` or `Security.framework`.

Execution plan: `desktop/macos` only.

### I6. iOS model policy

Depends on: M6. The model catalog is on another branch.

Done when: embeddings are the default on-device job, and a larger job stays off unless a flag is set. The scripted model covers the test.

Execution plan: `desktop/macos` only. Do not edit `mailune-ai`.

### I7. iOS extensions

Depends on: I2. Reuse: NEW.

Done when: a share extension, a notification service, and a widget are present as sources. The notification preview decrypts locally and never sends mail to a cloud model. A test reads one fixture subject from each. If an extension host cannot launch, commit the sources and say why. No Intel slice.

Execution plan: `desktop/macos` only. Commit on `batch7-imap`. Do not edit `mailune-ai`, `.github/workflows/ci.yml`, or `swift.yml`.

### B5. C# core project

Depends on: B2. `mailune-ffi` is on another branch.

Done when: `Mailune.Core` builds under `dotnet test` with one record round-trip. Uniffi bindgen is not run. The commit says the ffi crate is absent.

Execution plan: `desktop/windows` only. Commit on `batch8-ai`.

### W1. WinUI solution

Depends on: B5.

Done when: `Mailune.App`, `Mailune.AppCore`, and `Mailune.Core` are a solution and `dotnet test` passes. Windows x64 and arm64 are named. If the WinUI target cannot restore on this Mac, keep the project files and say so.

Execution plan: `desktop/windows` only.

### W2. Windows tokens

Depends on: W1.

Done when: colors, type, and spacing are XAML resources, and one local icon is named. A test reads the color keys. No remote image URL.

Execution plan: `desktop/windows` only.

### W3. Windows catalogues

Depends on: W1.

Done when: English and one other catalogue load, and a missing key falls back to English.

Execution plan: `desktop/windows` only.

### L8. Linux reader

Depends on: L7, P3. GTK 4 is not installed.

Done when: a reader description shows a fixture body, says JavaScript is off, and blocks remote content. A test reads those flags. No remote image URL.

Execution plan: `desktop/linux` only. Commit on `batch9-store`. Do not install GTK.

### L9. Linux composer

Depends on: L6, P13.

Done when: a composer description has recipient, subject, and body, and send stays off until confirm. A test reads that.

Execution plan: `desktop/linux` only.

### L10. Linux AI surfaces

Depends on: L8, L9. The model crates are on another branch.

Done when: the reader description includes a fixture summary and one reply chip. A test reads them. No model download.

Execution plan: `desktop/linux` only. Do not edit `mailune-ai`.

### L11. Linux desktop integration

Depends on: L7.

Done when: a `.desktop` file names the app and a mailto handler. A test reads both.

Execution plan: `desktop/linux` only.

### L12. Linux UI scenario

Depends on: L7, B7.

Done when: a scenario names a fixture subject. A test reads it. AT-SPI is not run. The commit says GTK is absent.

Execution plan: `desktop/linux` only.

### L13. Flatpak manifest

Depends on: L1. The reusable Flatpak workflow is not in this repo.

Done when: a manifest names the app id and the binary. A test reads the app id. Do not install Flatpak and do not submit to Flathub.

Execution plan: `desktop/linux` only.

### W4. Windows host

Depends on: W1.

Done when: notifications, network status, and an OAuth redirect sit behind fakes. `dotnet test` covers each. No network.

Execution plan: `desktop/windows` only. Commit on `batch8-ai`.

### W5. Windows on-device model

Depends on: W4, A5.

Done when: a prompt returns JSON from a scripted model. If the Windows AI API cannot be called on this Mac, keep the protocol and say so.

Execution plan: `desktop/windows` only.

### W6. Windows shell

Depends on: W2.

Done when: a shell description names three panes and one accelerator. A test reads them. The WinUI XAML compiler may stay unused.

Execution plan: `desktop/windows` only.

### W7. Windows thread list

Depends on: W6, B7.

Done when: a list description has one fixture row with a subject. A test reads it.

Execution plan: `desktop/windows` only.

### W8. Windows reader

Depends on: W7, P3.

Done when: a reader description shows a fixture body and says remote content and JavaScript are off. A test reads those flags. No remote image URL.

Execution plan: `desktop/windows` only.

### W9. Windows composer

Depends on: W6, P13.

Done when: a composer description has recipient, subject, and body, and send stays off until confirm. A test reads that.

Execution plan: `desktop/windows` only.

### W10. Windows AI surfaces

Depends on: W8, W9. The model crates are on another branch.

Done when: the reader description includes a fixture summary and one reply chip. A test reads them.

Execution plan: `desktop/windows` only. Do not edit `mailune-ai`.

### W11. Windows integration

Depends on: W7.

Done when: a description names a toast action and a badge count. A test reads both.

Execution plan: `desktop/windows` only.

### W12. Windows UI scenario

Depends on: W7, B7.

Done when: a scenario names a fixture subject. A test reads it. UIA is not run. The commit says the WinUI compiler is absent.

Execution plan: `desktop/windows` only.

### R4. Windows CI

Depends on: W1.

Done when: a new workflow runs `dotnet test`. Do not edit `.github/workflows/ci.yml` and do not change required checks.

Execution plan: `.github/workflows/windows.yml` only.

### B4. Kotlin core

Depends on: B2. `mailune-ffi` is on another branch. The Android SDK is not installed.

Done when: a Gradle JVM test round-trips one record. cargo-ndk and UniFFI are not run. The commit says so. Do not install the Android SDK.

Execution plan: `desktop/android` only. Commit on `batch9-store`.

### D1. Android project

Depends on: B4.

Done when: `:core`, `:ui`, and `:app` exist. `gradle test` passes on the JVM module. Compose sources may be present and uncompiled if the SDK is missing.

Execution plan: `desktop/android` only.

### D2. Android tokens

Depends on: D1.

Done when: colors, type, and spacing are constants a test reads. One local icon name is present. No remote image URL.

Execution plan: `desktop/android` only.

### D3. Android catalogues

Depends on: D1.

Done when: English and one other catalogue load, and a missing key falls back to English.

Execution plan: `desktop/android` only.

### D4. Android host

Depends on: D1.

Done when: secrets, a sync request, a notification channel, and an OAuth redirect sit behind fakes. A test covers each. No `keyring` crate.

Execution plan: `desktop/android` only.

### D6. Android shell

Depends on: D2.

Done when: a shell description names a list and a detail pane, and a phone and a tablet width. A test reads them.

Execution plan: `desktop/android` only.

### D7. Android thread list

Depends on: D6, B7.

Done when: a list description has one fixture row with a subject. A test reads it.

Execution plan: `desktop/android` only.

### D8. Android reader

Depends on: D7, P3.

Done when: a reader description shows a fixture body and says remote content and JavaScript are off. A test reads those flags.

Execution plan: `desktop/android` only.

### D9. Android composer

Depends on: D6, P13.

Done when: a composer description has recipient, subject, and body, and send stays off until confirm. A test reads that.

Execution plan: `desktop/android` only.

### D10. Android AI surfaces

Depends on: D8, D9. The model crates are on another branch.

Done when: the reader description includes a fixture summary and one reply chip. A test reads them.

Execution plan: `desktop/android` only. Do not edit `mailune-ai`.

### D5. Gemini Nano

Depends on: D4, A5. ML Kit is not installed.

Done when: a platform model reports whether Gemini Nano is available and falls back when it is not. A test covers both. Do not add the ML Kit artifact if it cannot resolve offline. The commit says the SDK is absent.

Execution plan: `desktop/android` only. Commit on `batch9-store`. Do not edit `mailune-ai`.

### D11. Android widgets and share

Depends on: D7.

Done when: a widget, a share target, and a mailto intent filter are named in sources. A test reads one fixture subject from each. Glance may stay uncompiled.

Execution plan: `desktop/android` only.

### D12. Android UI tests

Depends on: D7, B7.

Done when: one scenario names a fixture subject, and a test reads it. Screenshot capture may stay off. The commit says the emulator is absent.

Execution plan: `desktop/android` only.

### R5. Android CI

Depends on: D1.

Done when: a new workflow runs `gradle test` for the JVM modules. The emulator is not required. Do not edit `.github/workflows/ci.yml` and do not change required checks.

Execution plan: `.github/workflows/android.yml` only.

### R18. Homebrew cask and registry entry

Depends on: M18. Reuse: homebrew-tap `Casks/ketch.rb`; ketch `tap.yml`; ketch-registry.

Done when: a cask installs the macOS arm64 build and the registry lists Mailune. The cask and the registry live in those repositories.

Execution plan: `packages/homebrew-tap` and `packages/ketch-registry` worktrees. arm64 only. Commit in each repo. Do not push.

### X5. Consume llm-wire and llm-http

Depends on: the aulo extraction of those crates. Reuse: `packages/crates` llm-wire and llm-http, extracted from cox-provider-http.

Done when: Mailune calls those crates and does not keep a private copy. The crates are not in `packages/crates` yet.

Execution plan: `packages/crates` worktree. Add `llm-wire` and `llm-http` if they are missing. Commit there. Do not push.

### X6. Consume llm-openai

Depends on: X5. Reuse: `packages/crates` llm-openai.

Done when: the OpenAI BYOK path uses that crate. It is not in `packages/crates` yet.

Execution plan: `packages/crates` worktree, crate `llm-openai`, after X5. Commit there. Do not push.

### X7. Consume llm-anthropic

Depends on: X5. Reuse: `packages/crates` llm-anthropic.

Done when: the Anthropic BYOK path uses that crate. It is not in `packages/crates` yet.

Execution plan: `packages/crates` worktree, crate `llm-anthropic`, after X5. Commit there. Do not push.

### X8. Extract llm-testkit

Depends on: X5. Reuse: cox-provider-testkit.

Done when: replay cassettes with secret redaction live in one shared crate and Mailune's scripted providers use it.

Execution plan: `packages/crates` worktree, crate `llm-testkit`, after X5. Commit there. Do not push.

### D13. Android release

Depends on: D1, R14. Reuse: the R14 Play workflow.

Done when: a signed AAB can be sent to the Play internal track and a note says whether F-Droid is feasible. Signing waits on R14.

Execution plan: `desktop/android` on `batch9-store` only. No Play upload. Do not install the Android SDK.

### I8. TestFlight pipeline

Depends on: I1, R13. Reuse: the R13 App Store Connect workflow, based on the pyrlyn/ci macos-sign action.

Done when: a TestFlight upload can run from that workflow. A real upload waits on R13.

Execution plan: `desktop/macos` plus one new `.github/workflows/testflight.yml`; the required checks are unchanged. `desktop/macos/scripts/ipa.sh` archives `MailuneIOS` for devices and exports an App Store Connect IPA. Without a team id and signing identity it says so and stops before calling Apple. The workflow is `workflow_dispatch` only and calls the R13 pyrlyn/ci `testflight.yml` reusable workflow, pinned by SHA, with `ipa.sh` as the build command. It is validated with actionlint. No real upload.

What landed so far: `.github/workflows/testflight.yml` (dispatch only, validated with actionlint 1.7.12) calls the R13 pyrlyn/ci `testflight.yml` pinned at `95d76fd`, with the App Store Connect secrets by name and `desktop/macos/scripts/ipa.sh` as its build command. `ipa.sh` archives MailuneIOS and exports an App Store Connect IPA with Xcode's cloud-managed signing (`desktop/macos/ExportOptions-AppStore.plist`). It stops with a message before calling Apple when the team id or the API key is missing. No upload was run.

Left: the R13 workflow cannot run this build yet. Its Build step runs before any secret is available and without mise, so `ipa.sh` gets neither the App Store Connect key for signing nor XcodeGen and translate-toolkit. R13 needs two things before Build: mise, and the API key written to `~/private_keys` (as its upload step already does) with the key id and issuer in the environment. Its `macos-sign` step also requires a Developer ID certificate, which a TestFlight upload does not use. Outside R13: the iOS app has no app icon yet, which App Store Connect rejects, and the `APPLE_TEAM_ID` variable and the secrets must be set.

### W13. Windows release

Depends on: W1, R11. Reuse: the R11 signing workflow.

Done when: an MSIX is signed and a winget manifest installs it. Signing waits on R11.

Execution plan: `desktop/windows` on `batch8-ai` only. No real code signing.

### B10. BoltFFI survey

From ideas. BoltFFI 0.31 generates Swift, Kotlin, C#, and WASM bindings from one tool. Crux has moved to it. Revisit after the UniFFI phase.

Done when: a note compares BoltFFI 0.31 with the UniFFI bindings already in the tree and says whether a switch is worth it.

Execution plan: `docs/boltffi.md` on `batch7-imap`. Do not replace UniFFI.

### B11. Shared view-model core

From ideas. A Crux-style pure UI core in Rust, with `mailune-app` view models as a reducer.

Done when: every shell can render the same state machine from that reducer.

Execution plan: a pure reducer in `mailune-app` on `batch8-ai`. Stay under 500 lines of code.

### P33. Calendar view

From ideas. Grow the scheduling assistant (A22) and Graph calendar access (P25) into a calendar view.

Done when: the view shows local ICS suggestions. A JMAP Calendars source is added only after that RFC is published.

Execution plan: a calendar view on `batch7-imap` from fixture ICS. No JMAP Calendars source until that RFC is published.

### P34. Shared inboxes

From ideas. Shared inboxes and comments, as in Spark and Missive, conflict with a no-server path unless they use JMAP Sharing (RFC 9670).

Done when: a design shows shared inboxes and comments on RFC 9670, or records that the RFC cannot carry them.

Execution plan: `docs/jmap-sharing.md` on `batch7-imap`. No Mailune server in the path.
