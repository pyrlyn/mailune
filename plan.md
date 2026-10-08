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
| A3 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A4 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A6 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A11 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A12 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A13 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| A14 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A15 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A22 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| A23 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| A30 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| C3 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| C4 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| C5 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| C7 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| P7 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P8 | in progress | P1 | 4 | 0% | Cursor / grok 4.7 |
| P9 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P10 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| P11 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| P12 | in progress | P1 | 4 | 0% | Cursor / grok 4.7 |
| S1 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| S2 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| S3 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| S4 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| S5 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| S7 | in progress | P1 | 2 | 0% | Cursor / grok 4.7 |
| S8 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| S9 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |
| T4 | in progress | P1 | 3 | 0% | Cursor / grok 4.7 |

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

### A3. Local engine adapter

Depends on: A1. Reuse: a scripted engine, not a real model runtime.

Done when: generation, an embedding vector, and JSON-structured output come from a trait the test implements. No process and no download.

Execution plan: `mailune-ai` only.

### A4. Model catalog and verified blobs

Depends on: A3. Reuse: sha2 already in the workspace.

Done when: a catalog entry verifies a blob by SHA-256, records a resume offset, and deletes the blob. The bytes come from a trait. No network.

Execution plan: `mailune-ai` only.

### A6. Cloud BYOK request shapes

Depends on: A1. Reuse: the privacy router. The llm-* crates are not in this repo yet; do not depend on them.

Done when: an OpenAI-compatible request and an Anthropic request are built from a prompt, and a scripted response becomes a typed result. No HTTP.

Execution plan: `mailune-ai` only.

### A11. Thread summary cache

Depends on: A3, A10. Reuse: the prompt registry and the scripted engine.

Done when: short, detailed, and action-item summaries are cached by content hash.

Execution plan: `mailune-ai` only.

### A12. Daily digest

Depends on: A11. Reuse: the summary cache.

Done when: a digest covers messages since a given instant and skips older ones.

Execution plan: `mailune-ai` only.

### A13. Smart reply suggestions

Depends on: A11. Reuse: the scripted engine.

Done when: a thread yields three reply suggestions.

Execution plan: `mailune-ai` only.

### A14. Compose assist

Depends on: A10. Reuse: the prompt registry.

Done when: draft, rewrite, tone, shorten, and proofread each return text from the scripted engine.

Execution plan: `mailune-ai` only.

### A15. Style profile from sent mail

Depends on: A1. Reuse: NEW.

Done when: a per-recipient style profile is learned from sent plain text and can be rendered back as guidance. No model call.

Execution plan: `mailune-ai` only.

### A22. Scheduling extraction

Depends on: P26. Reuse: `icalendar` already in the workspace. Do not add `jiff` unless a date cannot be expressed with the types already in the tree.

Done when: a fixture sentence with a date and a time becomes an ICS suggestion. No SMTP and no calendar server.

Execution plan: `mailune-mime` only, next to the calendar module.

### A23. Language detection

Depends on: A2. Reuse: NEW. Survey a maintained detector before writing a table.

Done when: a message is labelled with a language. No translation call.

Execution plan: `mailune-ai` only.

### A30. Evaluation cassettes

Depends on: T5. Reuse: the synthetic mailbox only as fixtures you write yourself. Do not extract `llm-testkit`.

Done when: a cassette replays a feature call and a metric fails the run when the output drifts.

Execution plan: `mailune-ai` only.

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

### P7. IMAP initial sync

Depends on: P6. Reuse: the in-memory IMAP server in `mailune-imap`.

Done when: a sync batch records UIDVALIDITY, fetches envelope, flags, and BODYSTRUCTURE in batches, and applies a day window. No store write and no TCP.

Execution plan: `mailune-imap` only.

### P8. IMAP incremental sync

Depends on: P7. Reuse: the same session.

Done when: CONDSTORE CHANGEDSINCE and VANISHED are applied, and a server without QRESYNC falls back to a diff of uid sets.

Execution plan: `mailune-imap` only.

### P9. IMAP IDLE

Depends on: P7. Reuse: an injected clock. Do not sleep.

Done when: IDLE updates are parsed and a dropped session backs off then reconnects on the scripted server.

Execution plan: `mailune-imap` only.

### P10. Lazy body fetch

Depends on: P7. Reuse: imap-codec.

Done when: BODY.PEEK partial and BINARY requests return the requested bytes from the scripted server.

Execution plan: `mailune-imap` only.

### P11. IMAP mutations

Depends on: P7. Reuse: the scripted server.

Done when: STORE, MOVE or COPY+EXPUNGE, and APPEND run, and UIDPLUS maps the new uid. No TCP.

Execution plan: `mailune-imap` only.

### P12. Persist the operation queue

Depends on: the in-memory queue in `mailune-core` and S2.

Done when: pending ops round-trip through `mailune-store` and replay onto the existing state machine. The state machine stays in `mailune-core`. `mailune-core` must not depend on the store.

Execution plan: a load/save API on `mailune-store`, called by a test. Do not edit the queue's decision rules except to expose the pending set if it is still private.

### S1. mailune-store

Depends on: F2. Reuse: Diesel. Only this crate may depend on `diesel`, `diesel_migrations`, or `libsqlite3-sys`.

Done when: a file-backed SQLite database opens with WAL and a key argument. The key is a byte slice from the caller, never logged. If SQLCipher does not compile here, use bundled SQLite and say why in the commit message.

Execution plan: new crate `mailune-store`. No network and no keychain.

### S2. Schema v1 migrations

Depends on: S1. Reuse: Diesel migrations.

Done when: embedded migrations create accounts, mailboxes, messages, memberships, threads, parts, flags, sync_state, ops, and contacts.

Execution plan: `mailune-store` only. No raw SQL outside migration files.

### S3. Repository API

Depends on: S2. Reuse: Diesel's typed DSL.

Done when: upserts, a thread query, cursor paging, and counts go through the typed DSL.

Execution plan: `mailune-store` only.

### S4. Blob store

Depends on: S1. Reuse: `sha2` already in the workspace.

Done when: bodies are content-addressed, encrypted with a caller-supplied key, and evicted when a quota is exceeded.

Execution plan: `mailune-store` only.

### S5. FTS5 index

Depends on: S3. Reuse: FTS5.

Done when: subject, addresses, and body text are searchable. FTS5 virtual tables go through `sql_query` inside this crate only, with a comment that Diesel cannot model them.

Execution plan: `mailune-store` only.

### S7. Change feed

Depends on: S3. Reuse: `PRAGMA data_version`. The shared sqlite-change-feed crate does not exist yet; do not create it outside this repo.

Done when: a second connection in the same process observes a write as a typed invalidation.

Execution plan: `mailune-store` only.

### S8. Embedding store

Depends on: S3. Reuse: cosine in Rust. Do not use sqlite-vec.

Done when: vectors stored in SQLite return the nearest neighbours by cosine.

Execution plan: `mailune-store` only.

### S9. Hybrid retrieval fusion

Depends on: S6. Reuse: the search parser already in `mailune-core`.

Done when: two ranked lists fuse with reciprocal rank fusion at k=60 and the filters from the query parser still apply. No database in this function.

Execution plan: `mailune-core` only. Do not depend on `mailune-store`.

### T4. Queue property tests

Depends on: the in-memory queue. Reuse: `proptest` from rust.md.

Done when: random ops against a model mailbox keep idempotency and undo invariants.

Execution plan: tests in `mailune-core` next to the queue. Do not add a production dependency.
