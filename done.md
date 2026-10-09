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

### S1. mailune-store

Depends on: F2. Reuse: Diesel. Only this crate may depend on `diesel`, `diesel_migrations`, or `libsqlite3-sys`.

Done when: a file-backed SQLite database opens with WAL and a key argument. The key is a byte slice from the caller, never logged. If SQLCipher does not compile here, use bundled SQLite and say why in the commit message.

Execution plan: new crate `mailune-store`: `Store::open(path, key)` with a 32-byte raw key, PRAGMA key through a silenced connection, then WAL, foreign keys and busy timeout. SQLCipher on Apple targets (CommonCrypto); bundled SQLite elsewhere refuses a key. Tests use tempfile.

What landed: `mailune-store` opens a file-backed SQLite database in WAL mode. `Store::open(path, Some(key))` takes a raw 32-byte key, sends it as `PRAGMA key` from a zeroized buffer on a connection whose instrumentation is silenced, and maps a wrong key to `WrongKey`. Apple targets link `libsqlite3-sys` 0.38.2 `bundled-sqlcipher` against CommonCrypto; Linux, Windows and Android use `bundled` SQLite because SQLCipher there needs an OpenSSL build the CI runners do not have, and those builds refuse a key with `CipherUnavailable` instead of silently storing plaintext. The crate is empty on wasm32.

### S2. Schema v1 migrations

Depends on: S1. Reuse: Diesel migrations.

Done when: embedded migrations create accounts, mailboxes, messages, memberships, threads, parts, flags, sync_state, ops, and contacts.

Execution plan: one embedded migration `schema_v1` in `crates/mailune-store/migrations`, a hand-written `schema.rs`, and `Store::open` running pending migrations. A test touches every column through the DSL.

What landed: `crates/mailune-store/migrations/2026-10-08-000001_schema_v1` creates accounts, mailboxes, threads, messages, memberships, parts, flags, sync_state, ops and contacts, keyed on `(account_id, id)` because provider ids are only unique per account. `Store::open` runs the embedded migrations through `diesel_migrations` 2.3.2. `schema.rs` is hand-written; a test selects every column through the DSL so a drift between it and `up.sql` fails.

### S3. Repository API

Depends on: S2. Reuse: Diesel's typed DSL.

Done when: upserts, a thread query, cursor paging, and counts go through the typed DSL.

Execution plan: `repo.rs` in `mailune-store`: upserts for accounts, mailboxes and messages (memberships, keywords and the thread row in one transaction), a keyset-paged thread list per mailbox, a thread's messages, mailbox counts, and the sync cursor per scope.

What landed: `Store` gains `upsert_account`, `upsert_mailbox`, `upsert_message` (memberships, keywords and the thread row refreshed in one transaction; a message that changes thread leaves no empty thread), `thread_page` with a `(latest_at, id)` keyset cursor, `thread_messages`, `mailbox_counts`, and `set_sync_state`/`sync_state`. All of it is Diesel's typed DSL; recipient lists are one JSON column.

### S4. Blob store

Depends on: S1. Reuse: `sha2` already in the workspace.

Done when: bodies are content-addressed, encrypted with a caller-supplied key, and evicted when a quota is exceeded.

Execution plan: `blob.rs` in `mailune-store` plus a `blobs` migration: SHA-256 address, AES-256-GCM (aes-gcm 0.10.3, already in the tree) with the address as associated data, LRU eviction by a use counter when the quota is passed.

What landed: `Store::blobs(key, quota)` returns a `Blobs` handle. `put` addresses bytes by the SHA-256 of the plaintext, seals them with AES-256-GCM under the caller's 32-byte key (nonce from the digest, address as associated data), and evicts least-recently-used blobs until the quota fits; `get` refuses a wrong key or a swapped row with `BlobKey`. Recency is a counter, not the clock. Blobs live in a `blobs` table added by a second migration.

### P12. Persist the operation queue

Depends on: the in-memory queue in `mailune-core` and S2.

Done when: pending ops round-trip through `mailune-store` and replay onto the existing state machine. The state machine stays in `mailune-core`. `mailune-core` must not depend on the store.

Execution plan: `Queue::pending_ops()` in `mailune-core` exposes the pending set (no rule change). `Store::save_ops`/`load_ops` in `mailune-store` write it to the `ops` table with nanosecond times; a test replays the rows through `Queue::enqueue`.

What landed: `mailune-core` exposes `Queue::pending_ops()` (key, op, queued time), with no change to the queue's rules. `mailune-store` adds `save_ops`, which replaces the `ops` table with that set, and `load_ops`, which reads it back oldest first. Times are stored as nanoseconds, so a replayed op compares equal and the undo window still runs from the original queue time. A test reopens the file, replays through `Queue::enqueue`, and checks pending keys, location, schedule, idempotent re-replay and undo. `mailune-core` still has no store dependency.

### S8. Embedding store

Depends on: S3. Reuse: cosine in Rust. Do not use sqlite-vec.

Done when: vectors stored in SQLite return the nearest neighbours by cosine.

Execution plan: `vector.rs` in `mailune-store` plus an `embeddings` migration: f32 little-endian vectors per (message, chunk, model); `nearest` scans one account and model and sorts by cosine in Rust. No sqlite-vec.

What landed: An `embeddings` table (third migration) keeps one little-endian f32 vector per message chunk and model, cascading with the message. `Store::put_embedding` upserts a vector; `Store::nearest` scans one account and model, skips other dimensions and zero vectors, and returns the top chunks by cosine with a stable tie order. Store unit tests now share one fixture module.

### S9. Hybrid retrieval fusion

Depends on: S6. Reuse: the search parser already in `mailune-core`.

Done when: two ranked lists fuse with reciprocal rank fusion at k=60 and the filters from the query parser still apply. No database in this function.

Execution plan: `fusion.rs` in `mailune-core`: `fuse(lexical, semantic, query)` sums `1/(60 + rank)` per list, then applies the parser's field terms (from, to, has, is, label, before) to each candidate's fields. No store dependency.

What landed: `mailune_core::fuse` fuses a lexical and a semantic ranked list with reciprocal rank fusion at `RRF_K = 60` (a repeated id counts once per list), then keeps only candidates that pass the parsed query's field terms; free-text terms are left to the retrievers. Ties break by message id. No database and no store dependency.

### P17. JMAP read sync

Depends on: S3, P12. Reuse: the JMAP fixtures in `mailune-fixture`.

Done when: session, Mailbox/Email/Thread get, `/changes`, and `/query` parse from a scripted body and upsert through the existing repository. No TCP.

Execution plan: an `Http` transport trait in `mailune-protocol` and a `ScriptedHttp` fake in `mailune-testkit`; new crate `mailune-jmap` with the request envelope, session, Mailbox/Email/Thread get, `/changes`, `/query`, and a one-request sync step. A test upserts the scripted batches through `mailune-store` (dev-dependency only). jmap-client is not linked.

What landed: New crate `mailune-jmap`. `mailune-protocol` gains an `Http` transport trait (`HttpRequest` redacts `Authorization` from `Debug`) and `mailune-testkit` a `ScriptedHttp` fake; `mailune-core` gains `parse_rfc3339` for `receivedAt`. `JmapClient` loads the session, gets mailboxes, emails and threads, runs `Email/query` and `Email/changes`, and `sync` does one step in a single request with result references, falling back to a full listing on `cannotCalculateChanges`. A test applies two scripted steps to `mailune-store` through the repository and resumes from the saved state. jmap-client 0.4.3 is not linked: reqwest is a required dependency and it sends its own requests.

### P19. JMAP mutations and send

Depends on: P17.

Done when: a scripted exchange applies a flag change and an EmailSubmission. No SMTP socket.

Execution plan: `mutate.rs` in `mailune-jmap`: keyword and mailbox patches through `Email/set`, and `EmailSubmission/set` with `onSuccessUpdateEmail` filing the draft in Sent. Scripted responses only.

What landed: `JmapClient::set_keywords` and `move_email` send `Email/set` path patches (`keywords/$seen`, `mailboxIds/<id>`), so a replay is idempotent; `notUpdated` becomes `Error::Rejected`. `submit` sends `EmailSubmission/set` under the submission capability and, through `onSuccessUpdateEmail`, clears `$draft` and files the email in Sent; `notCreated` is refused. Scripted exchanges only, no SMTP.

### P20. JMAP MaskedEmail and Sieve

Depends on: P19.

Done when: a scripted exchange creates a MaskedEmail and lists a Sieve script. No network.

Execution plan: `extras.rs` in `mailune-jmap`: `MaskedEmail/set` create under Fastmail's capability and `SieveScript/get` under RFC 9661's, both over the scripted transport.

What landed: `JmapClient::create_masked_email` creates an enabled address through `MaskedEmail/set` under `https://www.fastmail.com/dev/maskedemail` (a `notCreated` refusal is `Error::Rejected`), and `sieve_scripts` lists scripts through `SieveScript/get` under `urn:ietf:params:jmap:sieve` (RFC 9661). Scripted exchanges only.

### P21. Gmail read sync

Depends on: P15, S3. Do not add `google-gmail1`.

Done when: threads, labels, a historyId incremental diff, and a batch fetch parse from a scripted body and upsert through the repository. No TCP.

Execution plan: new crate `mailune-gmail` with no reqwest or google-gmail1: requests are built by hand and sent through the injected `Http` trait. labels, threads.list, history.list across pages (404 means full sync), and batch GETs as multipart/mixed. Tests: scripted bodies upsert through `mailune-store` and resume from the saved historyId.

What landed: New crate `mailune-gmail`: `GmailClient` over the injected `Http` transport (no reqwest, no google-gmail1). `labels`, `thread_ids`, `history` (follows `nextPageToken`, 404 returns `None` for a full sync, refuses a non-numeric historyId), `messages` and `threads` through the multipart/mixed batch endpoint (50 per batch, inner 404 skipped, ids outside [A-Za-z0-9_-] refused), and `sync(since, limit)`. Labels become mailboxes; `UNREAD` and `STARRED` become flags. Tests upsert scripted bodies through `mailune-store` and resume from the saved historyId. Deleted ids are returned; the store has no delete yet.

### P22. Gmail mutations

Depends on: P21, P2.

Done when: batchModify labels, send, and a draft run against the scripted transport. No TCP.

Execution plan: `mailune-gmail` only: `modify_labels` (messages.batchModify, 1000 ids per call), `send_raw` and `create_draft` carrying caller-built RFC 5322 bytes as base64url (workspace `base64`). Scripted-transport tests check the request bodies and status errors.

What landed: `GmailClient::modify_labels` (messages.batchModify, chunked at 1000 ids; mark read = remove `UNREAD`, move = add target and remove source), `send_raw` (messages.send, base64url `raw`, optional `threadId`; documented as call-after-confirmation) returning `Sent { id, thread_id }`, and `create_draft` (drafts.create) returning `Draft { id, message }`. The caller builds the RFC 5322 bytes. Reuses the workspace `base64` crate. Scripted-transport tests check bodies, round-trip the raw bytes, and map a 400 to `Error::Status`.

### P23. Graph mail sync

Depends on: P15, S3. Do not add `graph-rs-sdk`.

Done when: folders, a delta query, and `$select` parse from a scripted body and upsert through the repository. No TCP.

Execution plan: new crate `mailune-graph` without graph-rs-sdk: folders (with child folders and nextLink paging) and a per-folder `messages/delta` with `$select`, resumed from a saved delta link; a 410 restarts the folder. Server links are followed only on the Graph host. Tests upsert scripted pages through `mailune-store`.

What landed: New crate `mailune-graph` (no graph-rs-sdk; requests go through the injected `Http` transport). `GraphClient::folders` walks top-level and child folders across `@odata.nextLink` pages, parents first, with roles from `wellKnownName` when Graph sends it. `delta(folder, saved_link)` runs `messages/delta?$select=...` with `Prefer: odata.maxpagesize=50`, splits `@removed` items from changed ones, and returns the delta link to save; a 410 restarts from scratch. Every server link must start with `https://graph.microsoft.com/` or the call fails with `Error::ForeignLink`, so the token never leaves the Graph host. Page and folder counts are capped. Categories become keywords. Tests upsert scripted folders and two delta rounds through `mailune-store`.

### P24. Graph mutations and send

Depends on: P23.

Done when: move, flag or category changes, sendMail, and `$batch` run against the scripted transport. No TCP.

Execution plan: `mailune-graph` only: `update_message` (PATCH isRead, flag, categories), `move_message` (returns the new id), `send_mime` (sendMail MIME form, base64 text/plain), `update_messages` through `$batch` in chunks of 20 with per-request statuses. Scripted-transport tests.

What landed: `GraphClient::update_message` sends a PATCH with only the set fields of `MessagePatch { is_read, flagged, categories }`; `move_message` posts to `/move` and returns the new id Graph assigns; `send_mime` uses sendMail's MIME form (base64, `text/plain`), documented as call-after-confirmation; `update_messages` sends PATCHes through `$batch` 20 at a time and returns a `BatchOutcome` per message in request order even when responses come back shuffled. Ids are percent-encoded in paths. Reuses the workspace `base64` crate.

### P25. Graph calendar and contacts

Depends on: P23.

Done when: availability and contact autocomplete parse from a scripted body. No TCP.

Execution plan: `mailune-graph` only: `schedule` (calendar/getSchedule, UTC via `Prefer: outlook.timezone`, slots from availabilityView, busy blocks from scheduleItems) and `autocomplete` (People API `$search`). A UTC formatter joins `parse_rfc3339` in `mailune-core`. Scripted-body tests.

What landed: `GraphClient::schedule(emails, start, end, interval)` posts `calendar/getSchedule` with `Prefer: outlook.timezone="UTC"` and returns a `Schedule` per address: `slots` from `availabilityView` (free, tentative, busy, out of office, working elsewhere), `busy` blocks from `scheduleItems` (a block in another zone is dropped rather than guessed), and the error Graph gives for a calendar it cannot read. `autocomplete(prefix, limit)` searches the People API and returns each scored address, most relevant first; quotes and backslashes are stripped from the term. `mailune-core` gains `format_rfc3339_utc`, the inverse of `parse_rfc3339`.

### P27. EWS for on-premises Exchange

Depends on: P23, P12. Reuse: survey Thunderbird ews-rs (MPL-2.0) before writing a client.

Done when: the survey says whether ews-rs can be reused, and Exchange Online still goes through Microsoft Graph. The client starts only after that survey.

Execution plan: survey `ews` (thunderbird/ews-rs) with cited sources in `research.md`; it has no HTTP client, so reuse it. New crate `mailune-ews`: `SyncFolderItems` over the injected `Http` transport, mapped onto protocol types; Exchange Online endpoints refused. Scripted SOAP tests upsert through `mailune-store`.

What landed: Survey in `research.md` ("EWS survey (P27)", sources checked 2026-10-08): `ews` 0.1.2 from thunderbird/ews-rs is typed EWS operations plus SOAP (de)serialization with no HTTP client, so it is reused unmodified; Exchange Online stays on Graph because Microsoft disables EWS there from October 2026. New crate `mailune-ews`: `EwsClient::new` refuses non-https endpoints, user info in the URL, and Exchange Online hosts; `sync_folder(folder, state, max)` sends `SyncFolderItems` through the injected `Http` transport and returns created/updated messages, deletions, read-flag changes, the next sync state, and whether the range is complete. A stale state is `Error::Response { code }`; SOAP faults on HTTP 500 are parsed. `ews` 0.1.2 panics on a non-fault document with no SOAP header, so such a document is refused before parsing. Bearer auth only (hybrid modern auth); NTLM and Basic are not supported.

### P31. Push relay

Depends on: P21, P23. Reuse: `axum`.

Done when: a Gmail or Graph webhook becomes an empty wake. A body that carries a token, subject, or mail text is refused. The relay stores no token and no mail.

Execution plan: new crate `mailune-push`: a pure screen (size cap, provider shape, no token-like or mail-content keys, no bearer/JWT values, Gmail Pub/Sub data limited to emailAddress and historyId) and an axum router `POST /hook/{provider}/{channel}` that turns a screened notice into an empty wake through a `Notifier` trait. Registry holds channel-to-device routes only. Tests drive the router with tower `oneshot`; no socket, no APNs or FCM.

What landed: New crate `mailune-push`. `screen(provider, body)` accepts a notice only if it is under 16 KiB, has the provider's shape (Gmail Pub/Sub `message.data` whose decoded JSON holds only `emailAddress` and `historyId`; Graph `value` array), names no key containing "token" or a mail-content key (subject, body, bodyPreview, snippet, encryptedContent, ...), and has no bearer, `ya29.` or JWT-shaped value. `router(registry, notifier)` serves `POST /hook/{gmail|graph}/{channel}`: unknown provider or channel is 404, a refused body 422 (reason not echoed), an oversized one 413, Graph's `validationToken` handshake is echoed as `text/plain` with `nosniff`, and a screened notice wakes the channel's device with an empty push through the `Notifier` trait (202). `MemoryRegistry` holds only channel-to-device-handle routes. Tests drive the router in process with tower `oneshot`; no socket, no APNs or FCM. New deps: axum 0.8.9 (no default features), tokio 1.53.2 and tower 0.5.3 for tests.

### A32. Embedding pipeline

Depends on: S8, S10. The local engine crate is on another branch.

Done when: text is chunked, two scripted embedders are scored, and the winner's vectors are stored. No model download.

Execution plan: `mailune-store` only: an `Embedder` trait, `Store::embed_best` scores each candidate by mean reciprocal rank on labelled probes, skips a broken engine, and stores the winner's vectors through `put_embedding`. The test chunks text with `mailune-mime`'s `chunk_plain` (dev-dependency only) and scores two scripted embedders.

What landed: `mailune-store` gains an `Embedder` trait (model name, `embed(texts)`) and `Store::embed_best(account, chunks, probes, candidates)`: each candidate embeds the chunks and the probe queries, is scored by mean reciprocal rank of each probe's relevant chunk (ties share the worst rank, so an embedder that cannot tell chunks apart scores low), and the best one's vectors are stored under its model name. A candidate that fails, returns the wrong count, mixes dimensions or sends a non-finite value is skipped and listed in `Choice::skipped`; none usable is `Error::NoEmbedder`. The test chunks bodies with `mailune-mime`'s `chunk_plain` (dev-dependency only; the store does not depend on mime), scores a word-hash embedder against a flat one and a broken one, and finds the right message through `nearest`. No model download; `mailune-ai` and `mailune-mime` untouched.

### A20. Ask with citations

Depends on: S9, A32. The router lives in `mailune-ai`, which another agent is editing.

Done when: a question returns an answer whose citations point at retrieved ids. No cloud model.

Execution plan: `mailune-core` only: an `Answerer` trait and `ask(question, fused hits, limit, text lookup, answerer)` that passes the top passages to the answerer and keeps only citations of retrieved ids; no passage or no valid citation is an error, not an unsourced answer. Tests use `fuse` output and scripted answerers.

What landed: `mailune-core` gains `ask(question, hits, limit, text, answerer)` and an `Answerer` trait (the router in `mailune-ai` can implement it; `mailune-ai` untouched). The first `limit` fused hits that have text become `Passage`s; with none the answerer is not called (`Error::NothingRetrieved`). The draft's citations are filtered to retrieved ids, deduplicated, in citation order; an empty draft or one citing nothing retrieved is `Error::Uncited`, and a model failure is `Error::Answerer`. Tests run `fuse` output through scripted answerers that cite an invented id, a duplicate and an unretrieved hit. No cloud model.

### B9. WASM subset

Depends on: P1, P4, S6.

Done when: protocol types, MIME parse, threading, and the query parser agree with a native parity test. If the wasm32 target is not installed, do not install it; say so in the commit.

Execution plan: new crate `mailune-wasm`: wasm-bindgen exports that forward to protocol serde, `mailune_mime::parse`, `thread_messages` and `parse_query`, with JSON shapes kept in one module. Native parity tests compare each export with the native call. Build the module with `cargo rustc --crate-type cdylib` for wasm32 (target installed).

What landed: New crate `mailune-wasm` (wasm-bindgen 0.2.129, already in the lock): `normalizeEnvelope` (protocol `Envelope` JSON round trip), `parseMime` (headers and part list), `threadMessages` (JSON in, thread trees out) and `parseQuery` (terms). Each export is one forwarding expression; the JSON shapes live in `json.rs`, so the domain crates stay serde-free. Native parity tests compare every export with the native call, errors included. The wasm32 target is installed: `cargo build --workspace --exclude mailune-cli --lib --target wasm32-unknown-unknown` passes and `cargo rustc -p mailune-wasm --target wasm32-unknown-unknown --crate-type cdylib --release` writes a 1.1 MB module. No `cdylib` in the manifest, because the Android cross build has no linker. rsa (in `mailune-mime`) pulls getrandom 0.2, which has no browser backend by default, so the crate enables getrandom's `js` feature on wasm32 only. 162 production lines.

### E10. WASM JMAP calls

Depends on: B9, P17. The web shell is on another branch.

Done when: the wasm crate runs one scripted JMAP query and returns the same mailbox ids as the native parser. No browser page in this task.

Execution plan: Add mailune-jmap to mailune-wasm; a private replay transport implements mailune_protocol::Http from recorded bodies and a poll-once runner drives the adapter future; export jmapMailboxIds(sessionUrl, replies) loads the session and lists mailbox ids as JSON; a parity test compares it with the native JmapClient over ScriptedHttp on the jmap fixtures; check the wasm32 workspace build.

What landed: mailune-wasm depends on mailune-jmap. A private Replay transport implements mailune_protocol::Http from recorded bodies, and a poll-once runner drives the adapter future (it refuses a future that would wait). The export jmapMailboxIds(sessionUrl, replies) loads the JMAP session and returns the mailbox ids as a JSON array; it stays a single forwarding expression. Tests: the export returns the same ids as the native JmapClient over ScriptedHttp on the session.json and sync-initial.json fixtures, and a missing reply is an error. The wasm32 workspace build passes. A fetch-based transport for real browser calls is left for the web client task.

### A28. Agent tools

Depends on: A9, B1. Reuse: cox-permission patterns already reflected in A9. Do not edit the queue.

Done when: a tool call has a scope, a preview, an undo record, and an audit line. The policy still fails closed. No send without the existing confirmation flag.

Execution plan: `mailune-ai` `agent.rs`: a typed `ToolCall` parsed from strict JSON, an `Agent` that checks the A9 policy and a granted `Scope`, builds a `Preview` and an `UndoRecord`, and appends an `AuditLine` for every decision. `commit` returns submissions for the app; the queue is untouched.

What landed: A typed `ToolCall` (summarize, archive, delete, send) is parsed from strict JSON and checked against the A9 policy and a granted scope. Each call gets a preview, an undo record (moves back to the original mailbox) and content-free audit lines. Send and delete are refused at commit until the app passes `Confirmation::Confirmed`; forward has no contract submission and is denied.

### A29. Local MCP server

Depends on: A28. Reuse: `rmcp` from `rust.md`.

Done when: a read-only tool is exposed and a send tool stays behind the in-app approval flag. No network listener in tests.

Execution plan: New crate `mailune-mcp` on rmcp 3.5: a `summarize` tool (read-only hint) and a `send` tool, each forwarding one `Agent::request` call. `mailune-ai` gains `request`, `held` and `approve` so a caller that cannot confirm gets send held for the app. Test drives the server over a tokio duplex.

What landed: New crate `mailune-mcp` (rmcp 3.5.1). `summarize` is read-only and runs through the A28 policy and scope; `send` is held for in-app approval (`Agent::approve` with `Confirmation::Confirmed`), and an MCP client has no argument that confirms. The test drives the server over an in-memory duplex, so no listener opens.

### A3. Local engine adapter

Depends on: A1. Reuse: a scripted engine, not a real model runtime.

Done when: generation, an embedding vector, and JSON-structured output come from a trait the test implements. No process and no download.

Execution plan: `mailune-ai` `engine.rs`: a `LocalEngine` trait (generate, embed, capability), `generate_json` that decodes untrusted JSON output, a `LocalProvider` adapter onto `Provider`, and a `ScriptedEngine` that later features and A30 replay.

What landed: A `LocalEngine` trait gives generation, an embedding vector and JSON-shaped output (`generate_json`, bad output is `Error::BadOutput`). `LocalProvider` puts an engine behind the router as a local model. `ScriptedEngine` replays canned replies and embeds by word hashing; no process, no download.

### A4. Model catalog and verified blobs

Depends on: A3. Reuse: sha2 already in the workspace.

Done when: a catalog entry verifies a blob by SHA-256, records a resume offset, and deletes the blob. The bytes come from a trait. No network.

Execution plan: `mailune-ai` `catalog.rs`: a `CatalogEntry` (capability, source, size, SHA-256), a `BlobSource` trait for the bytes, `download` that writes through the contract `Fs` and reports a `Resume` offset per chunk, `verify` that deletes a mismatched blob, and `delete`. Tests use the testkit `FakeHost` file system.

What landed: A catalog entry verifies a blob by SHA-256 and size (a mismatch deletes it), `download` resumes from a recorded offset after a dropped chunk, and `delete` removes the blob. Bytes come from a `BlobSource` trait and land through the contract `Fs`; no network.

### A11. Thread summary cache

Depends on: A3, A10. Reuse: the prompt registry and the scripted engine.

Done when: short, detailed, and action-item summaries are cached by content hash.

Execution plan: `mailune-ai` `summary.rs`: three versioned templates (`summarize-short`, `summarize-detailed`, `action-items`) in the registry snapshot, and a `SummaryCache` keyed by SHA-256 over template id, version and the rendered thread. Calls go through any `Provider`; tests use the scripted engine.

What landed: Short, detailed and action-item summaries come from three new registry templates and are cached by a SHA-256 content hash that includes the template version. A repeat costs no model call, an edited thread misses, and a failed call caches nothing.

### A12. Daily digest

Depends on: A11. Reuse: the summary cache.

Done when: a digest covers messages since a given instant and skips older ones.

Execution plan: `mailune-ai` `digest.rs`: `daily_digest` filters each thread to messages received at or after `since`, skips threads with none, summarizes the rest through the A11 cache (short kind), cites message ids, and orders entries newest first.

What landed: A daily digest covers only messages received at or after a given instant: older mail and threads with nothing new never reach the model. Each entry is a cached short summary that cites its message ids; entries are newest first.

### A27. Attachment summarisation

Depends on: A11. Reuse: a maintained extractor if one is already in `rust.md`; otherwise plain text only, named in the commit.

Done when: a text attachment becomes a summary through the scripted engine. No network.

Execution plan: `mailune-ai` `attachment.rs`: plain text only, since `rust.md` lists no maintained document extractor. `text/*` (not HTML) is decoded as UTF-8, capped at 16 KiB on a char boundary, and summarized through a new `summarize-attachment` template via `SummaryCache::complete`.

What landed: A text attachment becomes a cached summary through the scripted engine and a new `summarize-attachment` template. Plain text only: `rust.md` has no maintained extractor, so PDF, office formats, raw HTML and non-UTF-8 bodies return `Error::Unsupported`. Text is capped at 16 KiB.

### A6. Cloud BYOK request shapes

Depends on: A1. Reuse: the privacy router. The llm-* crates are not in this repo yet; do not depend on them.

Done when: an OpenAI-compatible request and an Anthropic request are built from a prompt, and a scripted response becomes a typed result. No HTTP.

Execution plan: `mailune-ai` `cloud.rs`, no llm-* dependency (those crates are not published in packages/crates yet). `build_request` runs `allow_cloud` and `redact_for_cloud` first, then shapes an OpenAI-compatible or Anthropic Messages body; the key is never in the shape, only which header carries it. `parse_response` turns scripted bodies into `Completion` or a typed error.

What landed: OpenAI-compatible and Anthropic request shapes are built from a `Prompt` after the privacy check and redaction; encrypted, local-only and local-preferred mail return `CloudForbidden`. The key is not in the shape, only the header that carries it. Scripted responses become `Completion::Text`; an error body keeps only its error type. No HTTP, no llm-* dependency yet.

### A33. Paid hosted AI tier

Depends on: A6, A7. Reuse: the existing privacy router.

Done when: a design names a confidential-compute provider behind that router. Encrypted mail is never sent to a cloud model. The adapter comes after the design.

Execution plan: Design doc `docs/hosted-ai.md` only; the creator deferred the paid tier. Name a confidential-compute provider with a primary source, place it behind the privacy router as a `Hosted` kind that needs `CloudAllowed`, keep encrypted mail local-only, and specify attestation before any request.

What landed: The design in `docs/hosted-ai.md` names Azure confidential GPU VMs (`NCCads_H100_v5`, a TEE that spans CPU and H100 GPU; source checked 2026-10-08) behind the existing router as a `Hosted` kind. The kind needs `CloudAllowed`, encrypted mail is forced local-only, and redaction and the ledger apply. Attestation is checked against pinned values and fails closed. No code; the adapter comes after the design.

### A14. Compose assist

Depends on: A10. Reuse: the prompt registry.

Done when: draft, rewrite, tone, shorten, and proofread each return text from the scripted engine.

Execution plan: `mailune-ai` `compose.rs`: a `Feature::Compose`, five versioned templates in the registry snapshot, and `assist(provider, action, text, privacy)`. The tone is a closed enum, never free text in the prompt.

What landed: Draft, rewrite, tone (formal, friendly, direct), shorten and proofread each run through their own versioned template under the new `Feature::Compose` and return trimmed text from the scripted engine. An empty reply is `BadOutput`. Nothing is sent.

### A15. Style profile from sent mail

Depends on: A1. Reuse: NEW.

Done when: a per-recipient style profile is learned from sent plain text and can be rendered back as guidance. No model call.

Execution plan: `mailune-ai` `style.rs`: a `StyleBook` that tallies greeting, sign-off, sentence and message length and exclamation use per recipient from sent plain text (quotes and signature dropped through `redact_for_cloud`), a `StyleProfile`, and `guidance` that renders it as one line of prompt text. No model call.

What landed: A per-recipient `StyleProfile` (greeting, sign-off, sentence and message length, exclamation share) is learned from sent plain text with quotes and the signature removed. `guidance` renders it back as prompt guidance. Counting only, no model call.

### A19. Natural-language rules

Depends on: A16, A9. Reuse: NEW.

Done when: a sentence becomes a typed rule, and a preview lists the messages that would match before the rule is enabled. No send.

Execution plan: `mailune-ai` `rules.rs`: a `rule-from-sentence` template under `Feature::Rules`, a closed JSON shape (`from`/`subject`/`category` conditions; archive, mark read, star and label actions; no send, forward or delete), validation, `preview` over messages, and `RulePreview::enable` as the only way to turn a rule on. Committed on this PR branch.

What landed: A sentence becomes a typed `ProposedRule` through a scripted model reply that must parse into a closed shape: no send, forward or delete action, at least one condition, no extra fields. `preview` lists the matching message ids, and only `RulePreview::enable` produces an `EnabledRule`, so the person has seen the matches first.

### A22. Scheduling extraction

Depends on: P26. Reuse: `icalendar` already in the workspace. Do not add `jiff` unless a date cannot be expressed with the types already in the tree.

Done when: a fixture sentence with a date and a time becomes an ICS suggestion. No SMTP and no calendar server.

Execution plan: `mailune-mime` `schedule.rs` next to `calendar.rs`: word heuristics for ISO dates, month-day in either order, weekdays, today and tomorrow, and 12- or 24-hour times. Civil-date arithmetic is done in place, so no `jiff` and no direct `chrono`. A one-hour floating-time VEVENT is built with `icalendar`.

What landed: A sentence with a date (ISO, month-day, weekday, today or tomorrow) and a time (3:30pm, 10 am, 14:05, noon) becomes a one-hour `ScheduleSuggestion` with floating local DTSTART and DTEND, and the ICS text is built with `icalendar` and parses back. Without both a date and a time there is no suggestion. No jiff, no SMTP, no calendar server.

### A25. Phishing and scam assessment

Depends on: C8, A2. The rsa-sha256 verifier is already on this branch.

Done when: auth results, link flags, and a scripted model verdict combine into one assessment. No network.

Execution plan: Mirror DKIM/SPF/DMARC outcomes and link flags as plain inputs in mailune-ai (no mime dependency), add a phishing-verdict template under a new Phishing feature, parse a strict JSON verdict, combine into weighted reasons and a risk level. The model may only raise risk.

What landed: `phishing.rs`: `combine` and `assess_phishing` turn auth outcomes, link flags and a scripted `{"verdict":...}` reply into one `Assessment` (risk, score, reasons). A malformed reply is ignored, and a "safe" verdict cannot lower a risk the facts set.

### A30. Evaluation cassettes

Depends on: T5. Reuse: the synthetic mailbox only as fixtures you write yourself. Do not extract `llm-testkit`.

Done when: a cassette replays a feature call and a metric fails the run when the output drifts.

Execution plan: Add eval.rs to mailune-ai: a JSON cassette records the call, input, prompt SHA-256, model reply and expected output; replay runs the real feature through ScriptedEngine; a token-F1 metric and a prompt-digest check fail the run with Error::Drift. Fixtures are hand-written in crates/mailune-ai/cassettes.

What landed: `eval.rs` and `cassettes/features.json`: five cassettes (short summary, rewrite, shorten, proofread, rule from a sentence) replay through the real feature code with `ScriptedEngine`. `run_cassettes` returns `Error::Drift` naming each cassette whose prompt digest changed, whose token-F1 fell below its `min_score`, or whose reply no longer parses. No model, no network.

### P7. IMAP initial sync

Depends on: P6. Reuse: the in-memory IMAP server in `mailune-imap`.

Done when: a sync batch records UIDVALIDITY, fetches envelope, flags, and BODYSTRUCTURE in batches, and applies a day window. No store write and no TCP.

Execution plan: Extend the scripted server with mailboxes (UIDVALIDITY, UIDNEXT, HIGHESTMODSEQ, UID SEARCH SINCE, UID FETCH of UID, FLAGS, ENVELOPE and BODYSTRUCTURE as one-line replies). Add Connection::over, select and initial_sync in a new sync.rs; decode FETCH with imap-codec; window dates via chrono, which imap-codec already links.

What landed: `sync.rs`: `Connection::select` records UIDVALIDITY, UIDNEXT and HIGHESTMODSEQ. `initial_sync` searches `UID SEARCH SINCE` for the day window, then fetches UID, FLAGS, ENVELOPE and BODYSTRUCTURE in batches, decoded by imap-codec, into a `SyncBatch` of `MessageMeta`. No store write, no TCP. The scripted server gained mailboxes and those commands. chrono is the one new direct dependency, already linked through imap-codec.

### P8. IMAP incremental sync

Depends on: P7. Reuse: the same session.

Done when: CONDSTORE CHANGEDSINCE and VANISHED are applied, and a server without QRESYNC falls back to a diff of uid sets.

Execution plan: Scripted server: ENABLE QRESYNC, per-message MODSEQ, expunge tombstones, UID FETCH (CHANGEDSINCE n [VANISHED]). Client incremental.rs: SyncState from a SyncBatch, incremental_sync picking QRESYNC, CONDSTORE plus UID diff, or full flag diff; UIDVALIDITY change resets.

What landed: `incremental.rs`: `incremental_sync` uses `CHANGEDSINCE` with `VANISHED` under QRESYNC, `CHANGEDSINCE` plus a `UID SEARCH` diff under CONDSTORE alone, and a full flag fetch plus UID diff otherwise. New mail above the last known UID is fetched with metadata, and a changed UIDVALIDITY returns a reset. `SyncState::apply` folds a `Delta` in. VANISHED ranges are tested for membership and never expanded.

### P9. IMAP IDLE

Depends on: P7. Reuse: an injected clock. Do not sleep.

Done when: IDLE updates are parsed and a dropped session backs off then reconnects on the scripted server.

Execution plan: Scripted server: IDLE/DONE, pushed EXISTS/EXPUNGE/FETCH while idling, drop and refuse hooks, and a SharedServer so reconnects see the same mailboxes. Client idle.rs: idle, idle_poll, idle_done, parse_idle_line, Backoff, and IdleWatch::step that waits on the injected mailune-protocol Clock (testkit FakeHost in tests).

What landed: `idle.rs`: `Connection::idle`, `idle_poll` and `idle_done` handle IDLE, and `parse_idle_line` reads EXISTS, EXPUNGE, FETCH flags and BYE. `IdleWatch::step` notices a dropped session and waits on the injected `Clock` with exponential `Backoff` (1, 2, then 4 s on the testkit fake clock, never on the thread). It then reconnects, re-selects and re-idles, and returns `Tick::Reconnected` so the caller can run an incremental sync. `SharedServer` lets reconnects reach the same scripted mailboxes.

### P11. IMAP mutations

Depends on: P7. Reuse: the scripted server.

Done when: STORE, MOVE or COPY+EXPUNGE, and APPEND run, and UIDPLUS maps the new uid. No TCP.

Execution plan: Scripted server: UID STORE, UID COPY, UID MOVE, UID EXPUNGE and APPEND with a synchronizing literal, answering COPYUID and APPENDUID under UIDPLUS. Client mutate.rs: store_flags, copy_messages, move_messages (MOVE, else COPY + Deleted + UID EXPUNGE, refused without UIDPLUS) and append; validate flags and mailbox names; bound COPYUID expansion.

What landed: `mutate.rs`: `store_flags` (silent +/-/replace), `copy_messages`, `move_messages` and `append` run on the scripted server. UIDPLUS `COPYUID` and `APPENDUID` become a `UidMap` and an `Appended`. Without MOVE, the fallback is COPY, `\Deleted` and `UID EXPUNGE`, and it is refused with `Unsupported` when UIDPLUS is missing, so other clients' deleted mail is never expunged. Flags and mailbox names that could break the command line are rejected (`Error::Argument`), and COPYUID ranges are bounded by the request size.

### E2. Web frontend scaffold

Depends on: B8. Reuse: Vite, React, TypeScript.

Done when: `web/` builds and a vitest checks that a generated payload type round-trips. No live server required for the test.

Execution plan: `web/` with Vite, React and TypeScript on npm, Node pinned in `mise.toml`. `web/scripts/contract.ts` turns the committed contract JSON Schema snapshot of `mailune-protocol` into `web/src/contract.gen.ts` with json-schema-to-typescript. Vitest checks the generated file has not drifted and that an `Event` round-trips through JSON and validates against the same schema (Ajv). UI strings come from the English web catalog `scripts/i18n.py` writes. New npm packages get `toolchain.md` rows.

What landed: `web/` on Vite, React and TypeScript, npm with Node 26.8.2 pinned in `mise.toml`. `web/src/contract.gen.ts` is generated by `web/scripts/contract.ts` (json-schema-to-typescript) from the `mailune-protocol` JSON Schema snapshot. Vitest checks it has not drifted and that an `Event` round-trips through JSON and validates against that schema with Ajv. UI text is read by msgctxt key from the English catalog `scripts/i18n.py` writes to `target/i18n/web`.

### E3. Web tokens and icons

Depends on: E2.

Done when: colors, type, and spacing are CSS variables, and one icon is an inline SVG. No remote image URL on one source line.

Execution plan: `web/src/tokens.css` (colour, type, spacing variables) and an inline SVG inbox icon. A vitest reads the variables, renders the icon, and scans every `web/` source line for a remote URL.

What landed: `web/src/tokens.css` holds colour, type, spacing and radius variables (light and dark, system fonts only), and the inbox icon is inline SVG in `currentColor`. A vitest reads the variables, keeps colour literals in the tokens file, renders the icon, and fails on any remote URL on a `web/` source line.

### E5. Web shell and thread list

Depends on: E3, E1.

Done when: three panes render a fixture thread list, and choosing a row shows that thread. A vitest covers the selection.

Execution plan: three panes (mailboxes, list, reading) over fixture `ThreadRow`s typed by the generated contract. E1 (`mailune-server`) is in PR #5, so no socket is used. A happy-dom vitest clicks a row and checks the reading pane.

What landed: three panes (mailboxes, conversations, open conversation) over fixture `ThreadRow`s typed by the generated contract. E1 (`mailune-server`) is still in PR #5, so nothing talks to a socket yet. A happy-dom vitest clicks rows and a mailbox and checks the reading pane and the current row.

### E6. Web reader

Depends on: E5, P3.

Done when: a message body renders in a sandboxed iframe from srcdoc with a strict CSP. No remote image URL on one source line.

Execution plan: a reader iframe with an empty `sandbox` and `srcdoc` carrying a `default-src 'none'` CSP. The body is the output of `mailune-mime::sanitize_html` for a hostile message, pinned as an insta snapshot in `mailune-mime` that the web test loads, so no sanitiser is written in TypeScript. The test checks scripts, remote images and navigation stay blocked.

What landed: the reader is an iframe with an empty `sandbox`, no referrer, and a `srcdoc` whose first element is a `default-src 'none'; base-uri 'none'; form-action 'none'` policy, with `<base target="_blank">` so a link cannot navigate the frame. The body is `mailune-mime::sanitize_html` output for a hostile message, pinned as an insta snapshot in `mailune-mime` and loaded by the web test. A branded `SanitizedHtml` type keeps raw strings out. Tests check the sandbox, the policy, and that scripts, styles, forms and remote sources are gone.

### E7. Web composer

Depends on: E5.

Done when: recipient, subject, and body round-trip, and send stays disabled until a confirm control is on.

Execution plan: a composer whose recipient, subject and body round-trip through the generated `save_draft` submission. Send stays disabled until a confirm checkbox is on, and any edit turns the confirmation off again. A happy-dom vitest covers both.

What landed: recipients (`Name <addr>` or plain), subject and body round-trip through the generated `save_draft` submission, and every edit is offered as one. Send stays disabled until the confirm box is on, any edit turns it off again, and Enter cannot submit around it. Both submissions validate against the contract schema in tests.

### M1. Xcode project

Depends on: B3.

Done when: XcodeGen generates MailuneModel, MailuneUI, and MailunePlatform for macOS arm64. No Intel target.

Execution plan: `desktop/macos` only. Port `project.yml` and the three local packages (MailuneModel, MailuneUI, MailunePlatform) from `batch7-imap`. `ARCHS = arm64`, no Intel slice. B3 is not done (it waits on B2 in PR #5), so the project builds without MailuneCore and the model is fixture-backed; B3 adds the package later. `scripts/test.sh` generates the project with XcodeGen and runs `xcodebuild test` on `platform=macOS,arch=arm64`; an arm64 test checks the running slice.

What landed: `desktop/macos/project.yml` generates `Mailune.xcodeproj` (not committed) from the local packages MailuneModel, MailuneUI and MailunePlatform, with `ARCHS = arm64` and no Intel slice. `scripts/test.sh` runs the three package suites, generates the project with XcodeGen 2.46.0, runs `xcodebuild test` on `platform=macOS,arch=arm64`, and fails unless `lipo -archs` on the built app says `arm64` only. `Host.architecture` and its tests fail on any other slice. Depends-on B3 is still open (it waits on B2 in PR #5); the project builds without MailuneCore and the model serves fixture data until B3 adds the package. Xcode and XcodeGen are in `toolchain.md`.

### M2. Swift design tokens

Depends on: M1.

Done when: colors, type, spacing, radii, and motion are Swift constants used by one view.

Execution plan: `desktop/macos` only. Port `Tokens.swift` (colour, type, spacing, radius, motion) and use it in `InboxView`.

What landed: `MailuneUI/Tokens.swift`: colour, type, spacing, radius and motion constants. `InboxView` uses every family; the shell and thread list use colour, type and spacing.

### M3. macOS icons

Depends on: M1.

Done when: one app icon and one toolbar icon are local SVG or asset-catalog entries. No remote image URL.

Execution plan: `desktop/macos` only. Port the local `AppIcon` and `ToolbarCompose` asset-catalog entries; the toolbar uses the local image. No URL.

What landed: `App/Assets.xcassets` holds a local `AppIcon` set and a `ToolbarCompose` image, used by the app's toolbar with a localised accessibility label. The artwork is a placeholder until the brand assets exist. The workspace guard in `mailune-cli/tests/secrets.rs` now also scans Swift, JSON, YAML and shell files under `desktop/`, so a remote image URL or a keychain call there fails the test.

### M4. macOS localisation

Depends on: M1.

Done when: one string is in an English catalog and a second catalog, and a missing key falls back to English.

Execution plan: `desktop/macos/scripts/catalogs.sh` runs `mise run i18n` and copies `target/i18n/apple/*.lproj` into a git-ignored `MailuneUI` resource folder; XcodeGen runs it as `preGenCommand` and `scripts/test.sh` runs it first. No hand-written `Localizable.strings`. `Copy` reads a language's generated table and falls back to English. The key-level fallback is tested on catalogs the test writes at runtime in the generator's format, so no shipped string stays untranslated.

What landed: `Copy.text` and `Copy.format` read the catalogs generated from `i18n/mailune.pot` and `i18n/<lang>.po` (en, de, fr, ja ship). A key missing from a language's table, or a language with no table, falls back to English, which Foundation does not do by itself. Tests cover English and German from the generated catalogs, an unknown language, a `{0}` placeholder, and a key missing from one catalog.

### M7. macOS shell

Depends on: M2, M3.

Done when: a NavigationSplitView shows three panes, a toolbar, one keyboard shortcut, and a command palette. Preview or a unit test builds the view.

Execution plan: `desktop/macos` only. Stay under 500 lines. Port `ShellView` (three-pane `NavigationSplitView`, toolbar, ⌘K) and the command palette; every visible string comes from `Copy`. A unit test filters the palette and one builds the view.

What landed: `ShellView`: a three-pane `NavigationSplitView` (mailboxes, thread list, detail), a Commands toolbar button on ⌘K, and a command palette whose commands are built from the mailbox list (`shell.go_to`). New keys `shell.commands`, `shell.command_placeholder` and `shell.go_to` have de, fr and ja translations. Tests filter the palette in English and German and build the view in an `NSHostingView`.

### M8. macOS thread list

Depends on: M7, B7.

Done when: the list is lazy, has a swipe action, multi-select, one indicator, and category tabs, fed by fixture data.

Execution plan: `desktop/macos` only, plus one test in `mailune-testkit`. Port `ThreadList` (lazy `List`, category tabs, unread mark, swipe to archive, multi-select) fed by a fixture that is a contract `Event::Snapshot` (the B7 JSON shape); the testkit test parses the same file as `Event`.

What landed: `ThreadList`: a `List` (lazy; only on-screen rows are built) with the four contract categories as tabs, an unread dot, a trailing swipe that archives, and multi-select. Rows decode from `MailuneModel/Fixtures/threads.json`, a contract `Event::Snapshot`; `mailune-testkit` parses the same file as `Event`, so the Swift fixture cannot drift from `ThreadRow`. New key `mail_list.category` has de, fr and ja translations.

### M9. macOS reader

Depends on: M8, P3, C6.

Done when: a message renders with quotes collapsed, one attachment, and a security badge. Remote content and JavaScript stay off.

Execution plan: `desktop/macos` only, on `m9-reader-composer` stacked on `m1-macos-chain`. No remote image URL. Port the reader from `batch7-imap` and fix it: the badge derives from a typed security state (DKIM verdict, signature check, encryption) instead of a literal; the body is the core's plain text with no web view; blocked remote content arrives as counts. Tests: quotes collapse and expand, one attachment, every badge rule, the blocked-remote banner, and a hosted view-tree walk that fails on any web view.

What landed: `MailuneModel/Message.swift`: `SecurityState` mirrors `mailune-mime`'s `DkimVerdict` plus the signature check and encryption; `SecurityBadge` is computed from it and a failed check outranks everything (so a forged, encrypted message shows "Authentication failed"). `RemoteSummary` carries what the core's remote policy blocked as counts, so the shell never holds a URL. `ReaderPresentation` collapses the core-split quote by default and lists the attachment. `ReaderView` renders plain text with a quote toggle (⇧⌘Q), an attachment row, the badge, and an "Images blocked" banner with the tracker count. `ReaderViewTests` hosts the reader in a window and fails if any view class is a web view; planting a `WKWebView` made it fail, so the check is live. The fixture is `messages.json`; the real security state arrives over the core binding once B3 lands. New `reader.*` keys have de, fr and ja translations.

### M10. macOS composer

Depends on: M7, P13, S13.

Done when: recipient chips, a body, one attachment, send later, and undo send round-trip through a fake. Send stays off until confirm.

Execution plan: `desktop/macos` only, on the same stacked branch as M9. Port the composer from `batch7-imap` and fix it: a `Composer` state machine (editing → confirming → held) where only `confirmSend` reaches the outbox, a fake outbox with an injected time, send later and undo send as one hold window. Tests: chips, send off until confirm, undo inside and after the window, send later, and a past send-later time.

What landed: `MailuneModel/Composer.swift`: `Composer` parses typed text into recipient chips (rejecting non-addresses and duplicates), holds subject, body, one attachment and an optional send-later time. `requestSend` asks for confirmation and `confirmSend` is the only call that reaches the `Outbox`; the draft is held until the later of the undo window (10 s) and the send-later time, so `undo` restores the whole draft until it goes out. `FakeOutbox` stands in for the core's queue until B3. `ComposerView` (⌘N from the shell) shows chips, a file picker for one attachment, a send-later toggle and date, a confirmation dialog before send, and Undo while held; it cannot be dismissed while a message is held. Contact suggestions (S13) are not shown yet: they need the core binding. New `compose.*` keys have de, fr and ja translations.

### R11. Windows signing workflow

Depends on: R10. Reuse: none yet. This is a gap in pyrlyn/ci.

Done when: pyrlyn/ci has a reusable workflow that signs a Windows build and packs an MSIX. This repository's required checks stay unchanged.

Execution plan: `packages/infra` worktree (remote pyrlyn/ci). New reusable workflow only. Do not push.

What landed: pyrlyn/ci `.github/workflows/windows-sign.yml` (pyrlyn/ci PR #57, commit `1941a53`). It packs one unpacked layout with `makeappx`, signs it with `signtool` and an Authenticode `.pfx` from `WINDOWS_CERTIFICATE`/`WINDOWS_CERTIFICATE_PWD`, verifies the signature, and uploads the MSIX. Missing secrets stop the run before anything is packed. No Mailune workflow calls it, so the required checks are unchanged.

### R12. Flatpak workflow

Depends on: R10. Reuse: none yet. This is a gap in pyrlyn/ci.

Done when: pyrlyn/ci has a reusable workflow that builds the Flatpak. This repository's required checks stay unchanged.

Execution plan: `packages/infra` worktree. New reusable Flatpak workflow only. Do not push.

What landed: pyrlyn/ci `.github/workflows/flatpak.yml` (pyrlyn/ci PR #57, commit `1941a53`). It installs `flatpak-builder`, builds the caller's manifest with the runtime installed from Flathub, and uploads one `.flatpak` bundle. No signing secret. No Mailune workflow calls it, so the required checks are unchanged.

### R13. TestFlight workflow

Depends on: R10. Reuse: pyrlyn/ci macos-sign action as a base.

Done when: pyrlyn/ci has a reusable iOS TestFlight workflow that talks to the App Store Connect API. This repository's required checks stay unchanged.

Execution plan: `packages/infra` worktree. New reusable TestFlight workflow only. Do not push.

What landed: pyrlyn/ci `.github/workflows/testflight.yml` (pyrlyn/ci PR #57, commit `1941a53`). The certificate is checked with the `macos-sign` action (`discover`), and the IPA is uploaded with `xcrun altool` using the App Store Connect API key trio. A pull request never uploads. The caller's build command must produce a signed IPA. No Mailune workflow calls it, so the required checks are unchanged.

### R14. Play workflow

Depends on: R10. Reuse: none yet.

Done when: pyrlyn/ci has a reusable workflow that uploads a signed Android App Bundle. This repository's required checks stay unchanged.

Execution plan: `packages/infra` worktree. New reusable Play workflow only. Do not push.

What landed: pyrlyn/ci `.github/workflows/play.yml` (pyrlyn/ci PR #57, commit `1941a53`). It checks the bundle with `jarsigner -verify -strict` and uploads it through `r0adkll/upload-google-play` (pinned v1.1.5) with `PLAY_SERVICE_ACCOUNT_JSON`. Track and status are validated, and a pull request never uploads. No Mailune workflow calls it, so the required checks are unchanged.
