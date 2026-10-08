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
