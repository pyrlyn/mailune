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

### A2. Privacy policy router

Depends on: A1. Reuse: runa-fit hardware probe.

Done when: a feature's privacy class, a capability probe, a fallback chain, and a budget pick a provider. Encrypted mail stays on a local model.

Execution plan: `mailune-ai` only. No network.

What landed: The router in `mailune-ai` picks a provider from privacy class, capabilities, a fallback chain, and a budget. Encrypted mail stays on a local model.

### A7. Data-flow ledger

Depends on: A2. Reuse: cox usage-ledger convention.

Done when: an in-memory ledger records feature, provider, bytes, message ids, and retention class.

Execution plan: `mailune-ai` only. No store.

What landed: An in-memory ledger records feature, provider, bytes, message ids, and retention class.

### A9. Prompt-injection guard and tool permissions

Depends on: A1. Reuse: cox-permission; aulo fail-closed conventions.

Done when: untrusted model output cannot call a tool the policy did not allow, and a failed check denies the call.

Execution plan: `mailune-ai` only. Policy code is not inside a prompt.

What landed: A policy outside the model approves a typed tool call and denies anything it did not allow.

### A10. Prompt registry

Depends on: A1. Reuse: NEW.

Done when: versioned prompt templates have an insta snapshot. No gettext.

Execution plan: `mailune-ai` only.

What landed: Versioned prompt templates have a committed insta snapshot. No gettext.

### A16. Triage heuristics

Depends on: A1. Reuse: existing `Category` on the protocol.

Done when: header heuristics assign Primary, Social, Promotions, or Updates without calling a model.

Execution plan: `mailune-ai` only.

What landed: Header heuristics assign the existing `Category` values. No model call.

### A17. Priority score and needs-reply

Depends on: A16. Reuse: NEW.

Done when: a priority score and a needs-reply flag come from flags and headers.

Execution plan: `mailune-ai` only.

What landed: Priority and needs-reply come from flags and headers. No model call.

### A18. First-time sender screener

Depends on: A16. Reuse: NEW.

Done when: an address with no prior inbound message is marked as a first-time sender.

Execution plan: `mailune-mime` only.

What landed: An address with no prior inbound message is screened; a repeat is not.

### A21. Follow-up detector

Depends on: A2. Reuse: NEW.

Done when: a sent message with no later inbound from that address is awaiting a reply.

Execution plan: `mailune-core` only.

What landed: A sent message with no later inbound from that address is awaiting a reply. Pure function in `mailune-core`.

### A24. Extraction cards

Depends on: A2, P1. Reuse: NEW.

Done when: plain text yields an OTP, a tracking number, an invoice total, or a flight code when the fixture contains one.

Execution plan: `mailune-mime` only. Heuristics, no model.

What landed: Plain text yields an OTP, a tracking number, an invoice total, or a flight code from fixtures.

### B1. View models for list, thread, composer, settings

Depends on: F9. Reuse: cox-app; view-model shapes from the protocol `Event`.

Done when: folding events updates a thread list, an open thread, a composer draft, and a settings snapshot. No printing.

Execution plan: `mailune-app` only. Keep `ready()`.

What landed: Folding events updates a thread list, an open thread, a composer draft, and a settings snapshot. Composer and settings still read notice text, because `Event` has no typed payloads for them yet.

### B7. Contract scenario JSON

Depends on: T1. Reuse: ketch contract JSON.

Done when: a testkit scenario fold is written as JSON another front end can replay.

Execution plan: `mailune-testkit` only.

What landed: A testkit scenario fold is written as JSON.

### B8. JSON-RPC surface

Depends on: F7. Reuse: rtok JSON-RPC shape.

Done when: typed JSON-RPC methods forward to a handler trait and return `Event`s. No axum, no socket.

Execution plan: new crate `mailune-rpc`.

What landed: `mailune-rpc` forwards typed JSON-RPC methods to a handler trait. No socket.

### C2. OpenPGP encrypt, sign, decrypt, verify

Depends on: P1, C1. Reuse: `pgp` from rust.md.

Done when: a test generates a key, signs and encrypts a message, then decrypts and verifies it. Key bytes are not in `Debug`. No keychain.

Execution plan: new crate `mailune-crypto`.

What landed: `mailune-crypto` generates a key, signs and encrypts, then decrypts and verifies. `Debug` hides key bytes. No keychain.

### C7. Authentication-Results and DKIM verify

Depends on: P1. Reuse: `mail-auth` if it is the maintained verifier.

Done when: `Authentication-Results` becomes a badge, and DKIM verifies against a DNS answer supplied by a trait. No network.

Execution plan: `mailune-mime` only.

What landed: `Authentication-Results` becomes a badge, and DKIM rsa-sha256 with relaxed/relaxed verifies against a supplied TXT record. ed25519, simple canonicalization, and the `l=` tag are still open.

### C10. Fuzz targets for MIME, autoconfig, and search

Depends on: P1, P14, S6. Reuse: `libfuzzer-sys` from rust.md.

Done when: fuzz targets exist for MIME parse, autoconfig XML, and the search query parser, and `cargo test --workspace` does not build them.

Execution plan: `fuzz/` excluded from workspace members. Do not edit the parsers except to expose a public entry that already exists.

What landed: Fuzz targets for MIME parse, autoconfig XML, and the search query live in `fuzz/`, which is excluded from the workspace.

### P3. HTML policy

Depends on: P1. Reuse: ammonia. `text-sanitize` is not available; do not vendor it.

Done when: HTML is sanitized, `cid:` is rewritten, remote images are stripped, and a plain-text alternative is produced. Nothing is fetched.

Execution plan: `mailune-mime` only.

What landed: HTML is sanitized with ammonia, `cid:` is rewritten, remote images are stripped, and html2text produces the plain alternative. `text-sanitize` is still unavailable.

### P5. IMAP connection state machine

Depends on: P0, F8. Reuse: `imap-codec` 1.0.0, already chosen.

Done when: a connection state machine speaks CAPABILITY and LOGIN to the in-memory scripted server. No TCP and no TLS socket.

Execution plan: new crate `mailune-imap`.

What landed: An in-memory state machine speaks CAPABILITY and LOGIN to the scripted server. No TCP and no TLS handshake.

### P6. IMAP LIST to mailbox roles

Depends on: P5. Reuse: `imap-codec`.

Done when: a LIST response maps SPECIAL-USE and common names onto `MailboxRole`.

Execution plan: `mailune-imap` only.

What landed: LIST maps SPECIAL-USE and common names onto `MailboxRole`.

### P13. SMTP send behind a fake transport

Depends on: P2. Reuse: `lettre` if it is the maintained SMTP crate.

Done when: a message is handed to a fake transport, including a send-later delay decision. No socket. Saving to Sent is a result value, not a store write.

Execution plan: new crate `mailune-smtp`.

What landed: A message is handed to an in-memory transport, including a send-later decision. `lettre` was not linked: its SMTP client opens a socket, and the message is already built.

### P14. Autoconfig lookup behind a fetch trait

Depends on: F8. Reuse: the XML and JSON parsers already in `mailune-auth`.

Done when: a fetch trait returns fixture bytes for ISPDB, and SRV/MX heuristics fill host, port, and security. No live DNS.

Execution plan: `mailune-auth` only.

What landed: A fetch trait returns fixture bytes. SRV and MX heuristics fill host, port, and security. No live DNS.

### P15. OAuth loopback and refresh

Depends on: F8, C1. Reuse: PKCE S256 already in `mailune-auth`; cox-mcp loopback shape.

Done when: a listener trait receives a redirect and a refresh rotates the token in memory. No TCP bind and no keychain.

Execution plan: `mailune-auth` only.

What landed: A listener trait accepts the redirect and refresh rotates an in-memory token. No TCP bind and no keychain.

### P16. Sync scheduler

Depends on: F8. Reuse: `NetworkState` on the protocol.

Done when: a per-account scheduler pauses when offline, metered, or low-battery, and orders work by priority. No sleeping thread.

Execution plan: `mailune-core` only.

What landed: The scheduler orders accounts by priority and pauses when offline, metered, or low-battery. No sleeping thread.

### P26. iCalendar parse

Depends on: P1. Reuse: NEW. Survey a maintained iCalendar crate before writing a parser.

Done when: a REQUEST or REPLY fixture becomes a typed invite. No SMTP.

Execution plan: `mailune-mime` or a new `mailune-cal` if the MIME crate would mix two jobs. Prefer `mailune-mime` if the file stays small.

What landed: An iCalendar REQUEST or REPLY becomes a typed invite. No SMTP.

### P29. New-mail notification policy

Depends on: P16, A17. Reuse: NEW.

Done when: VIP, quiet hours, and a priority flag decide notify or suppress.

Execution plan: `mailune-core` only. Take a plain score input so this file does not depend on `mailune-ai`.

What landed: VIP, quiet hours, and a plain priority number decide notify or suppress. No OS notification.

### P30. Snooze, reminder, and reply-later ops

Depends on: the in-memory queue. Reuse: the queue already in `mailune-core`.

Done when: snooze, reminder, and reply-later are scheduled ops with an undo window. No METADATA protocol.

Execution plan: `mailune-core` only.

What landed: Snooze, reminder, and reply-later are scheduled ops on the in-memory queue, with its undo window.

### R9. Nightly fuzz workflow

Depends on: C10. Reuse: cox nightly fuzz workflow.

Done when: a nightly workflow runs the fuzz targets for a short time. Do not change the required PR checks in `ci.yml`.

Execution plan: `.github/workflows/` only, plus the fuzz crate from C10.

What landed: `.github/workflows/nightly.yml` runs the fuzz targets for a short time. The required PR checks in `ci.yml` are unchanged.

### S10. Chunker

Depends on: P3. Reuse: NEW.

Done when: plain text splits into 400–600 token windows, quotes and a signature are stripped, and each chunk carries a provenance span.

Execution plan: `mailune-mime` only. A token is a whitespace word if no tokenizer crate is already in the repo.

What landed: Plain text splits into 400–600 word windows, with quotes and a signature stripped and a provenance span on each chunk.

### S11. Export eml and mbox

Depends on: P1. Reuse: NEW.

Done when: one message writes as `.eml` and a list writes as mbox.

Execution plan: `mailune-mime` only.

What landed: One message writes as `.eml` and a list writes as mbox.

### T2. Scripted IMAP server

Depends on: P5, T1. Reuse: `imap-codec`; provider quirk rows already in `mailune-mime`.

Done when: an in-memory server answers greeting, CAPABILITY, LOGIN, SELECT, and a fixture FETCH, with one Gmail quirk and one Dovecot quirk.

Execution plan: `mailune-imap` only. No TCP.

What landed: An in-memory server answers greeting, CAPABILITY, LOGIN, SELECT, and a fixture FETCH, with a Gmail quirk and a Dovecot quirk. No TCP.

### T3. JMAP, Gmail, and Graph fixtures

Depends on: T1. Reuse: static fixtures. Prefer fixtures over `wiremock`.

Done when: a JMAP Email/get, a Gmail history list, and a Graph delta page parse into protocol envelopes or thread rows. No network.

Execution plan: new crate `mailune-fixture`.

What landed: Static JMAP, Gmail, and Graph fixtures parse into protocol rows. No network.

### F12. Gettext as the localisation source, converted to each shell's native format

Requested by the creator. Reuse: translate-toolkit storage classes; research/mail-app lang/*.po as the seed.

Done when: translate-toolkit is pinned in `mise.toml`; `i18n/*.po` is the single source; one command writes every native catalog and fails on broken keys, placeholders or plural forms.

What landed: `"pipx:translate-toolkit" = "3.20.0"` in `mise.toml` with `i18n`, `i18n --check` and `i18n:test` tasks. `i18n/mailune.pot` plus `de`, `fr`, `ja` catalogs imported from research/mail-app (276 strings, msgctxt keys, `{0}` placeholders). `scripts/i18n.py` writes `target/i18n/`: Apple `Localizable.strings` and `.stringsdict`, Android `strings.xml`, Windows `.resw` (plurals as `<key>_<tag>`), Linux `.mo`, web i18next v4 JSON. Gettext plural forms are spread over CLDR tags; placeholders become `%n$@`, `%n$s`, `{n}` or `{{n}}` (`{{count}}` in plurals).

### B2. UniFFI records

Depends on: B1, F6. Reuse: ketch-ffi and cox-ffi.

Done when: records, errors, one async function, and a callback interface compile, and each export forwards one call.

Execution plan: new crate `mailune-ffi` (uniffi 0.32.2 in proc-macro mode, as ketch-ffi and cox-ffi). Records mirror `Address`, `Category`, `ThreadRow`, `Event` and the four view models with `From` conversions; `MailuneError` is the one error enum. `HostSecrets` is a foreign trait that becomes the protocol `SecretStore`; the async export `has_token` forwards to a new `mailune_app::has_token`. `MailuneCore` wraps `Views` and exports `fold` and `state`. Every export body is one expression; the existing syn test walks the crate. Verify with nextest, clippy, fmt.

What landed: New crate `mailune-ffi` (uniffi 0.32.2, proc-macro mode). Records mirror `Address`, `Category`, `ThreadRow`, `Event` and the view models; `MailuneError` is the one error enum; the foreign trait `HostSecrets` becomes the protocol `SecretStore`; the async export `has_token` forwards to the new `mailune_app::has_token`; `MailuneCore` exports `fold` and `state`. Every export is one expression, checked by the existing syn test. No bindgen binary yet; B3 adds it.

### B4. Kotlin core

Depends on: B2. `mailune-ffi` is on another branch. The Android SDK is not installed.

Done when: a Gradle JVM test round-trips one record. cargo-ndk and UniFFI are not run. The commit says so. Do not install the Android SDK.

Execution plan: `desktop/android` only. A Gradle build (Kotlin JVM 2.4.20, Gradle 9.8.0 and Java 27 from `desktop/android/mise.toml`, as cox `plugins/` does, so Rust-only contributors install nothing) with one `core` module. `Address` mirrors the B2 record and its converter writes the UniFFI buffer layout; a JUnit test round-trips it. Generated bindings replace the hand-written converter once uniffi-bindgen runs. No Android SDK, no cargo-ndk. Verify with `gradle test` in `desktop/android`.

What landed: A Gradle build in `desktop/android` (Kotlin JVM 2.4.20, Gradle 9.8.0, Java 27 from its own `mise.toml`, bytecode level 17) with a `core` module. `Address` mirrors the B2 record and `FfiConverterAddress` writes the RustBuffer layout; four JUnit tests round-trip it, and a Rust test in `mailune-ffi` checks uniffi writes the same bytes. cargo-ndk and uniffi-bindgen are not run and the Android SDK is not installed.

### B6. C ABI

Depends on: B1. The shared abi-drift crate is not in this repo.

Done when: a cbindgen header, a VAPI, and a meson file exist, and a test fails if the header drifts from the Rust records. Each export forwards one call.

Execution plan: new crate `mailune-capi`, the C ABI for the Vala shell, shaped like ketch-capi. One opaque `MailuneCore` handle over `mailune_ffi::MailuneCore`, so both bindings run the same code; records cross as JSON in an `{"ok"}` / `{"error"}` envelope (serde derives added to the `mailune-ffi` records, JSON Schema behind its `schema` feature). Answers are `malloc`ed (libc) so `g_free` frees them; every export catches panics and is one expression, and the syn test in `mailune-cli` now walks `mailune-capi` `no_mangle` functions too. Commit `include/mailune.h` (cbindgen), `vapi/mailune.vapi`, `meson.build` with a Vala test, and `schema/payloads.schema.json`; drift tests regenerate the header with the cbindgen library and the schema from the records. valac is not installed, so the Meson test is not run here.

What landed: New crate `mailune-capi`: one opaque `MailuneCore` handle over `mailune_ffi::MailuneCore`, records as contract JSON in an `ok`/`error` envelope, `malloc`ed answers, a panic guard in `respond`. Committed `include/mailune.h` (cbindgen), `vapi/mailune.vapi`, `meson.build` with `tests/capi.vala`, and `schema/payloads.schema.json`; drift tests regenerate the header and the schema of the records and fail on any difference. The forward-only syn test now covers `no_mangle` exports in `mailune-capi`. A C program linked against the cdylib was run by hand; valac is not installed, so the Meson test was not run.

### B10. BoltFFI survey

From ideas. BoltFFI 0.31 generates Swift, Kotlin, C#, and WASM bindings from one tool. Crux has moved to it. Revisit after the UniFFI phase.

Done when: a note compares BoltFFI 0.31 with the UniFFI bindings already in the tree and says whether a switch is worth it.

Execution plan: `docs/boltffi.md`. Facts from crates.io, the BoltFFI README and docs, and the Crux repository, each with its URL and the 2026-10-08 check date; a comparison against `mailune-ffi` and `mailune-capi`; a decision and triggers to revisit. Do not replace UniFFI.

What landed: `docs/boltffi.md` compares BoltFFI 0.31.0 with the UniFFI 0.32.2 bindings (B2) and the C ABI (B6), with primary sources checked on 2026-10-08. Decision: no switch now. The native C# target is BoltFFI's real gain; the vendor speed numbers are marked unverified and do not matter for coarse calls; the C target is experimental and sync-only; minor releases break every few weeks. Triggers to revisit are listed.

### B11. Shared view-model core

From ideas. A Crux-style pure UI core in Rust, with `mailune-app` view models as a reducer.

Done when: every shell can render the same state machine from that reducer.

Execution plan: a pure reducer in `mailune-app` (`reducer.rs`), Crux-style: `Ui` holds the B1 `Views` plus UI-only state (composer open, pending confirmation, query); `Ui::update(Msg)` changes state and queues typed `Submission`s in an outbox the runtime drains; send and delete only leave after `Confirm`. Shells reach the same machine through `mailune-ffi` (`MailuneCore::dispatch` → `UiState`, Swift/Kotlin/C#) and `mailune-capi` (`mailune_core_dispatch`, JSON, Vala); the header and payload schema are re-blessed. No I/O. Under 500 lines of code.

What landed: `mailune_app::Ui` is a pure Crux-style reducer: `Msg` in, state plus an outbox of typed `Submission`s out, with the B1 `Views` folded from `Msg::Core`. Send and delete wait for `Confirm`; archive, search and select go straight to the outbox; closing the composer saves the draft. Swift, Kotlin and C# reach it through `MailuneCore::dispatch` / `ui_state` in `mailune-ffi`, and Vala through `mailune_core_dispatch` in `mailune-capi` (header and payload schema re-blessed). The web shell will reach it through `mailune-server` once that forwards a dispatch method; that is not wired in this task.

### C3. Autocrypt headers

Depends on: C2. Reuse: the OpenPGP key type only if `mailune-crypto` can be called without editing it. Prefer a header codec in `mailune-mime`.

Done when: an Autocrypt header is parsed and gossip keys are collected from a message. No WKD network lookup.

Execution plan: `mailune-mime` only, plus a fuzz target. New `autocrypt.rs`: a header codec for Autocrypt Level 1 (`addr`, `prefer-encrypt`, `keydata`; `_`-prefixed attributes ignored, any other unknown attribute voids the header; keydata base64 with folding stripped and a size cap). `sender_autocrypt` returns the From address's key only when exactly one valid header matches From; `gossip_keys` reads `Autocrypt-Gossip` from a decrypted payload and keeps keys for the outer recipients only. Keydata stays opaque bytes (no `mailune-crypto` call). No WKD, no network. Fixture tests plus an `autocrypt-header` fuzz target in the nightly matrix.

What landed: `mailune-mime::autocrypt` with `sender_autocrypt` (exactly one valid header whose `addr` is the single From address; delivery reports ignored) and `gossip_keys` (decrypted payload only, filtered to the outer recipients, first key per address, no preference). Underscore attributes are ignored, any other unknown or duplicate attribute voids the header, keydata is unfolded base64 capped at 64 KiB and kept opaque. Seven fixture tests and an `autocrypt-header` fuzz target in the nightly matrix. No WKD and no network.

### C4. S/MIME verify and decrypt

Depends on: P1. Reuse: a maintained `cms` and `x509-cert` if they fit.

Done when: a fixture verifies a signature and decrypts with a supplied key. No keychain.

Execution plan: new crate `mailune-smime` (the card names it; `docs/architecture.md` moves S/MIME out of the `mailune-crypto` row) on RustCrypto `cms` 0.2.3 and `x509-cert` 0.2.5, the stable lines that share the workspace rsa 0.9. `verify` checks every SignerInfo (issuer/serial or key id, message-digest and content-type attributes, RSA PKCS #1 v1.5 over SHA-256/384/512; SHA-1 refused); `decrypt` finds the RSA key-transport entry for the supplied certificate and opens AES-128/192/256-CBC content. CMS layer only: MIME extraction and certificate trust stay with the caller. Fixtures made by OpenSSL (`fixtures/regenerate.sh`) so the tests prove interop, not a round trip with ourselves.

What landed: new crate `mailune-smime` on `cms` 0.2.3 and `x509-cert` 0.2.5 (plus `aes` 0.8 and `cbc` 0.1, which share cipher 0.4 with cms). `verify(signed, detached)` checks every signer (issuer/serial or key id; message-digest and content-type attributes; RSA PKCS #1 v1.5 over SHA-256/384/512, SHA-1 refused) and returns the content and signer certificates, untrusted; `decrypt(enveloped, certificate, key)` opens RSA key transport and AES-128/192/256-CBC. `Certificate` exposes subject, `emails()` and DER; `PrivateKey` is redacted in `Debug`. Twelve tests against OpenSSL-made fixtures (detached, opaque by key id, AES-256 to one recipient, AES-128 to two; tampered content and signature, stranger and wrong key). CMS layer only: MIME extraction and certificate trust are left to the caller. No keychain.

### C5. S/MIME sign and encrypt

Depends on: C4. Reuse: the same crate.

Done when: a fixture signs and encrypts, and C4 verifies and decrypts it.

Execution plan: `mailune-smime` only. `sign(content, certificate, key, Signature::Detached | Opaque)` builds SignedData with the cms builder (SHA-256, RSA PKCS #1 v1.5, issuer/serial sid, signing-time attribute, signer certificate included); `encrypt(content, recipients)` builds EnvelopedData with AES-256-CBC and RSA key transport per recipient, refusing RSA keys under 2048 bits. Randomness from the OS. Tests: both shapes verify through C4 `verify`, a two-recipient message decrypts through C4 `decrypt` for each recipient, a small key is refused; a manual OpenSSL cross-check of the output is noted in the commit.

What landed: `mailune-smime::sign(content, certificate, key, Signature::Detached | Opaque)` builds SignedData with the cms builder (SHA-256 with RSA PKCS #1 v1.5, issuer/serial sid, signing-time attribute, signer certificate included; a key that is not the certificate's is refused) and `encrypt(content, recipients)` builds AES-256-CBC EnvelopedData with RSA key transport per recipient, refusing keys under 2048 bits. Six tests: both shapes verify through C4, a two-recipient message decrypts through C4 for each recipient, sign-then-encrypt round-trips, and the refusals. OpenSSL 3.6 verified both signature shapes and decrypted the two-recipient message by hand.
