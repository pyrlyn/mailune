# Mailune

GitHub: https://github.com/pyrlyn/mailune

A local-first, AI-first mail client: one Rust core (IMAP/SMTP, JMAP, Gmail API, Microsoft Graph, sync, storage and search, AI orchestration, crypto) behind native SwiftUI, WinUI 3, GTK4/Vala and Jetpack Compose shells and a web client. See `research.md` and `docs/architecture.md`.

| # | Status | Priority | Complexity | Readiness | Agent |
| --- | --- | --- | --- | --- | --- |
| C1 | todo | P0 | 2 | 40% | |
| C11 | in progress | P0 | 2 | 10% | Cursor / grok 4.7 |
| F4 | todo | P0 | 3 | 0% | |
| F5 | todo | P0 | 2 | 0% | |
| F10 | todo | P0 | 2 | 0% | |
| R1 | in progress | P0 | 2 | 10% | Cursor / grok 4.7 |
| R17 | todo | P0 | 2 | 40% | |
| X1 | todo | P0 | 3 | 0% | |
| X2 | todo | P0 | 2 | 0% | |
| X3 | todo | P0 | 3 | 0% | |
| X4 | todo | P0 | 2 | 0% | |
| X9 | todo | P0 | 2 | 0% | |
| X10 | todo | P0 | 2 | 0% | |
| X11 | todo | P0 | 2 | 0% | |
| A2 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A7 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| A9 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A10 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| A16 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A17 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A18 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| A21 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| A24 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| B1 | in progress | P1 | 4 | 0% | Cursor / grok 4.7 |
| B7 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| B8 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| C2 | in progress | P1 | 4 | 0% | Cursor / grok 4.7 |
| C7 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| C10 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P3 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P5 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P6 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| P13 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P14 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P15 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P16 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P26 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P29 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| P30 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| R9 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| S10 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| S11 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| T2 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| T3 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |

### C1. SecretStore integration: tokens, passwords, DB key; Android via host callback

Depends on: F8, X2. Reuse: X2 secret-store; cox no_real_keychain_in_tests.rs guard.

Done when: Secrets never in config or logs (tests); test suite never touches real keychain. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Already landed: `mailune-app` reads a bearer token through `SecretStore` and logs `Secret(redacted)` only. Passwords, the database key, and the Android host callback are still open. `X2` (the shared keychain crate) is still open.

### C11. Threat model and trust boundaries document (core, AI, MCP)

Depends on: F7. Reuse: aulo conventions (untrusted model output, fail closed); cox permission design.

Done when: docs/threat-model.md reviewed by creator. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: Write `docs/threat-model.md` for the core, AI pipeline, and MCP. Trust boundaries follow the architecture (untrusted mail, model output, MCP clients; fail closed; encrypted mail never leaves the device for a cloud model). Reuse the shape of aulo's untrusted-output rules and cox's permission design. Do not edit other tasks' files.

### F4. mailune-config: typed TOML, layering, committed JSON Schema

Depends on: F2, X1. Reuse: X1 layered-config (from cox-config/src/load.rs, rtok src/config/validate.rs).

Done when: Unknown keys reported with file:line; schema staleness test; only this module imports figment/toml. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### F5. Telemetry: tracing, rotating logs, secret redaction, optional OTLP (off by default)

Depends on: F2, X3. Reuse: X3 telemetry-setup (from aulo-telemetry, cox-telemetry).

Done when: Logs redact tokens (test); no println! in libraries. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### F10. Gettext catalogs for core-originated strings

Depends on: F2, X4. Reuse: X4 gettext-catalog (from cox-i18n); research/mail-app lang/*.po.

Done when: en + ru + de/fr/ja load; missing key falls back to msgid. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### R1. Core CI: pyrlyn/ci ci-rust.yml matrix + changes.yml + pipeline.yml

Depends on: F2. Reuse: pyrlyn/ci ci-rust.yml, changes.yml, pipeline.yml; packages/crates path-gates.

Done when: Required checks green on PR. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Execution plan: Add GitHub Actions reused from `pyrlyn/ci` (`ci-rust.yml`, `changes.yml`, `pipeline.yml`) and the packages/crates path-gates pattern. Matrix is arm64 macOS only plus Linux/Windows x86_64. Do not edit protocol sources or `docs/threat-model.md`.

### R17. Brand entry (pyrlyn/brand brands/mailune) and landing docs/site.md

Depends on: F1. Reuse: apps/brand, apps/landing CONTENT_CONTRACT.md.

Done when: Landing build lists Mailune. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

Already landed: `docs/site.md` describes the landing. `brands/mailune` is not in the brand repo: an incomplete folder would fail the token build (`build.mjs` lists brands, and each one needs tokens, logos, exports, and a rebuilt `dist/`).

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

Already landed: `docs/text-sanitize.md` records that the crate is not on crates.io and not under `packages/crates` yet, so Mailune does not depend on it.

### X11. Extract SQLite change feed (PRAGMA data_version poller) — sqlite-change-feed

Depends on: nothing. Reuse: cox-store src/watch.rs.

Done when: Two-connection test sees writes from another process. Workspace checks (nextest, clippy, fmt under `mise exec`) are green.

### A2. Privacy policy router

Depends on: A1. Reuse: runa-fit hardware probe.

Done when: a feature's privacy class, a capability probe, a fallback chain, and a budget pick a provider. Encrypted mail stays on a local model.

Execution plan: `mailune-ai` only. No network.

### A7. Data-flow ledger

Depends on: A2. Reuse: cox usage-ledger convention.

Done when: an in-memory ledger records feature, provider, bytes, message ids, and retention class.

Execution plan: `mailune-ai` only. No store.

### A9. Prompt-injection guard and tool permissions

Depends on: A1. Reuse: cox-permission; aulo fail-closed conventions.

Done when: untrusted model output cannot call a tool the policy did not allow, and a failed check denies the call.

Execution plan: `mailune-ai` only. Policy code is not inside a prompt.

### A10. Prompt registry

Depends on: A1. Reuse: NEW.

Done when: versioned prompt templates have an insta snapshot. No gettext.

Execution plan: `mailune-ai` only.

### A16. Triage heuristics

Depends on: A1. Reuse: existing `Category` on the protocol.

Done when: header heuristics assign Primary, Social, Promotions, or Updates without calling a model.

Execution plan: `mailune-ai` only.

### A17. Priority score and needs-reply

Depends on: A16. Reuse: NEW.

Done when: a priority score and a needs-reply flag come from flags and headers.

Execution plan: `mailune-ai` only.

### A18. First-time sender screener

Depends on: A16. Reuse: NEW.

Done when: an address with no prior inbound message is marked as a first-time sender.

Execution plan: `mailune-mime` only.

### A21. Follow-up detector

Depends on: A2. Reuse: NEW.

Done when: a sent message with no later inbound from that address is awaiting a reply.

Execution plan: `mailune-core` only.

### A24. Extraction cards

Depends on: A2, P1. Reuse: NEW.

Done when: plain text yields an OTP, a tracking number, an invoice total, or a flight code when the fixture contains one.

Execution plan: `mailune-mime` only. Heuristics, no model.

### B1. View models for list, thread, composer, settings

Depends on: F9. Reuse: cox-app; view-model shapes from the protocol `Event`.

Done when: folding events updates a thread list, an open thread, a composer draft, and a settings snapshot. No printing.

Execution plan: `mailune-app` only. Keep `ready()`.

### B7. Contract scenario JSON

Depends on: T1. Reuse: ketch contract JSON.

Done when: a testkit scenario fold is written as JSON another front end can replay.

Execution plan: `mailune-testkit` only.

### B8. JSON-RPC surface

Depends on: F7. Reuse: rtok JSON-RPC shape.

Done when: typed JSON-RPC methods forward to a handler trait and return `Event`s. No axum, no socket.

Execution plan: new crate `mailune-rpc`.

### C2. OpenPGP encrypt, sign, decrypt, verify

Depends on: P1, C1. Reuse: `pgp` from rust.md.

Done when: a test generates a key, signs and encrypts a message, then decrypts and verifies it. Key bytes are not in `Debug`. No keychain.

Execution plan: new crate `mailune-crypto`.

### C7. Authentication-Results and DKIM verify

Depends on: P1. Reuse: `mail-auth` if it is the maintained verifier.

Done when: `Authentication-Results` becomes a badge, and DKIM verifies against a DNS answer supplied by a trait. No network.

Execution plan: `mailune-mime` only.

### C10. Fuzz targets for MIME, autoconfig, and search

Depends on: P1, P14, S6. Reuse: `libfuzzer-sys` from rust.md.

Done when: fuzz targets exist for MIME parse, autoconfig XML, and the search query parser, and `cargo test --workspace` does not build them.

Execution plan: `fuzz/` excluded from workspace members. Do not edit the parsers except to expose a public entry that already exists.

### P3. HTML policy

Depends on: P1. Reuse: ammonia. `text-sanitize` is not available; do not vendor it.

Done when: HTML is sanitized, `cid:` is rewritten, remote images are stripped, and a plain-text alternative is produced. Nothing is fetched.

Execution plan: `mailune-mime` only.

### P5. IMAP connection state machine

Depends on: P0, F8. Reuse: `imap-codec` 1.0.0, already chosen.

Done when: a connection state machine speaks CAPABILITY and LOGIN to the in-memory scripted server. No TCP and no TLS socket.

Execution plan: new crate `mailune-imap`.

### P6. IMAP LIST to mailbox roles

Depends on: P5. Reuse: `imap-codec`.

Done when: a LIST response maps SPECIAL-USE and common names onto `MailboxRole`.

Execution plan: `mailune-imap` only.

### P13. SMTP send behind a fake transport

Depends on: P2. Reuse: `lettre` if it is the maintained SMTP crate.

Done when: a message is handed to a fake transport, including a send-later delay decision. No socket. Saving to Sent is a result value, not a store write.

Execution plan: new crate `mailune-smtp`.

### P14. Autoconfig lookup behind a fetch trait

Depends on: F8. Reuse: the XML and JSON parsers already in `mailune-auth`.

Done when: a fetch trait returns fixture bytes for ISPDB, and SRV/MX heuristics fill host, port, and security. No live DNS.

Execution plan: `mailune-auth` only.

### P15. OAuth loopback and refresh

Depends on: F8, C1. Reuse: PKCE S256 already in `mailune-auth`; cox-mcp loopback shape.

Done when: a listener trait receives a redirect and a refresh rotates the token in memory. No TCP bind and no keychain.

Execution plan: `mailune-auth` only.

### P16. Sync scheduler

Depends on: F8. Reuse: `NetworkState` on the protocol.

Done when: a per-account scheduler pauses when offline, metered, or low-battery, and orders work by priority. No sleeping thread.

Execution plan: `mailune-core` only.

### P26. iCalendar parse

Depends on: P1. Reuse: NEW. Survey a maintained iCalendar crate before writing a parser.

Done when: a REQUEST or REPLY fixture becomes a typed invite. No SMTP.

Execution plan: `mailune-mime` or a new `mailune-cal` if the MIME crate would mix two jobs. Prefer `mailune-mime` if the file stays small.

### P29. New-mail notification policy

Depends on: P16, A17. Reuse: NEW.

Done when: VIP, quiet hours, and a priority flag decide notify or suppress.

Execution plan: `mailune-core` only. Take a plain score input so this file does not depend on `mailune-ai`.

### P30. Snooze, reminder, and reply-later ops

Depends on: the in-memory queue. Reuse: the queue already in `mailune-core`.

Done when: snooze, reminder, and reply-later are scheduled ops with an undo window. No METADATA protocol.

Execution plan: `mailune-core` only.

### R9. Nightly fuzz workflow

Depends on: C10. Reuse: cox nightly fuzz workflow.

Done when: a nightly workflow runs the fuzz targets for a short time. Do not change the required PR checks in `ci.yml`.

Execution plan: `.github/workflows/` only, plus the fuzz crate from C10.

### S10. Chunker

Depends on: P3. Reuse: NEW.

Done when: plain text splits into 400–600 token windows, quotes and a signature are stripped, and each chunk carries a provenance span.

Execution plan: `mailune-mime` only. A token is a whitespace word if no tokenizer crate is already in the repo.

### S11. Export eml and mbox

Depends on: P1. Reuse: NEW.

Done when: one message writes as `.eml` and a list writes as mbox.

Execution plan: `mailune-mime` only.

### T2. Scripted IMAP server

Depends on: P5, T1. Reuse: `imap-codec`; provider quirk rows already in `mailune-mime`.

Done when: an in-memory server answers greeting, CAPABILITY, LOGIN, SELECT, and a fixture FETCH, with one Gmail quirk and one Dovecot quirk.

Execution plan: `mailune-imap` only. No TCP.

### T3. JMAP, Gmail, and Graph fixtures

Depends on: T1. Reuse: static fixtures. Prefer fixtures over `wiremock`.

Done when: a JMAP Email/get, a Gmail history list, and a Graph delta page parse into protocol envelopes or thread rows. No network.

Execution plan: new crate `mailune-fixture`.
