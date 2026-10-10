# Architecture

The source for this plan, with the feature matrix, the reuse inventory and a filterable task list, is the Mailune blueprint artifact. This file is the part that agents need while they work.

## Principles

- **Dependency direction.** Dependencies flow one way: `mailune-protocol` (contract), then `mailune-core` and `mailune-ai` (pure domain), then adapters, then `mailune-app` (assembly), then surfaces.
- **I/O behind traits.** All network, filesystem and process access goes through traits defined in `mailune-protocol`. Each heavy dependency is owned by one crate, and a `cargo metadata` test checks this.
- **Offline-first.** The encrypted local SQLite database is the source of truth for the UI. Servers are the source of truth for mail state.
- **No server in the path.** The device talks to providers directly. The optional push relay only wakes devices.
- **Prior art.**
  - Delta Chat core: one Rust core behind a C FFI and JSON-RPC.
  - Bitwarden sdk-internal: UniFFI for mobile, WASM for the web.
  - Crux: a pure UI-state core. On its own it does not fit a sync-heavy client.

## Crates

| Crate | Role | Owns |
| --- | --- | --- |
| `mailune-protocol` | Contract: ids, records, `Submission`, `Event`, I/O and host traits | serde, schemars |
| `mailune-core` | Pure: sync state machines, JWZ threading, op queue, scheduling, rules DSL | — |
| `mailune-ai` | Pure: feature router, privacy policy, prompt registry, tool permissions | — |
| `mailune-imap`, `-smtp`, `-jmap`, `-gmail`, `-graph` | Protocol adapters | imap stack (spike P0), lettre, jmap-client, reqwest |
| `mailune-mime` | Adapter: MIME parse and build, HTML sanitising, HTML to text, DKIM/DMARC checks | mail-parser, mail-builder, ammonia, html2text, mail-auth |
| `mailune-store` | Adapter: SQLite with SQLCipher, FTS5, vectors, blob store | diesel, libsqlite3-sys (bundled-sqlcipher) |
| `mailune-ai-local` | Adapter: local generation and embeddings | runa-engine (llama-cpp-2) |
| `mailune-ai-cloud` | Adapter: bring-your-own-key providers | packages/crates llm-http, llm-openai, llm-anthropic |
| `mailune-crypto` | Adapter: OpenPGP, S/MIME, Autocrypt | pgp, cms, x509-cert |
| `mailune-auth` | Adapter: OAuth PKCE, account autoconfig | oauth2 or the cox-mcp auth code |
| `mailune-config` | Adapter: typed layered TOML config and its committed JSON Schema | figment, toml |
| `mailune-app` | Assembly: owns config and the runtime, folds Events into view models | tokio |
| `mailune-ffi`, `-capi`, `-rpc`, `-wasm`, `-cli`, `-mcp`, `-server` | Surfaces that only forward calls | uniffi, cbindgen, axum, wasm-bindgen, clap, rmcp |
| `mailune-testkit` | Fakes for every trait, scenario builder | wiremock |

## Data flow

1. The UI sends a typed `Submission`, for example `Archive(thread_ids)`, `Send(draft)` or `Summarize(thread)`.
2. The core applies the change to the local store in a transaction, so the UI updates optimistically, and records an operation in the persistent op queue.
3. The change feed (`PRAGMA data_version`) notifies `mailune-app`, which emits Events and fresh paged snapshots to the UI.
4. The adapter replays the operation against the server. On success the op is marked done. On a conflict the local change is rolled back and a `Notice` event is emitted.

**Runtime and calls**
- FFI calls are async and never hold a core lock.
- The C ABI uses a wakeup callback and a polled event queue (the scull-ffi model).
- Every exported function catches panics.

**Host callbacks**
- `SecretStore`, `Notifier`, `NetworkState`, `AuthSession`.
- `PlatformModel`, `BackgroundScheduler`.

## Sync model

| Protocol | Incremental sync | Push | Mutations |
| --- | --- | --- | --- |
| IMAP | CONDSTORE/QRESYNC, with a UID diff fallback; UIDVALIDITY resets trigger a resync | IDLE | STORE, MOVE, APPEND, with UIDPLUS mapping |
| JMAP | `/changes` with state strings | EventSource or WebSocket (RFC 8887) | Email/set, EmailSubmission/set |
| Gmail API | `history.list` from a historyId | push relay (Pub/Sub), otherwise polling | batchModify, send, drafts |
| Graph | delta queries per folder | push relay (webhooks), otherwise polling | move, PATCH, sendMail, `$batch` |
| EWS (later, on-premises only) | SyncFolderItems | streaming notifications | — |

**Op queue**
- Ops are idempotent and carry stored server ids.
- Flags merge per keyword. For moves, the server wins.
- Send goes through the queue too: send-later, an undo-send delay, retries.

**Snooze, reminders and reply-later**
- They sync across devices through IMAP keywords plus METADATA, JMAP keywords or Gmail labels where the server supports it.
- Otherwise they stay local, and the UI says so.

**Mobile background sync**
- iOS uses BGAppRefresh and processing tasks. Android uses WorkManager.
- Sync order: the visible mailbox first, then inbox headers, then bodies on Wi-Fi.

## AI pipeline

**Privacy classes**
- Each account, thread and feature has one class:
  - `local-only`: forced for encrypted mail.
  - `local-preferred`: the default.
  - `cloud-allowed`: bring-your-own-key.
- The router also checks device capability, the models installed, battery and budget.

**Where requests run**
1. Heuristics.
2. The platform model, called through a host callback: Apple Foundation Models, Gemini Nano, Windows AI.
3. Bundled llama.cpp.
4. A cloud provider with the user's own key.
5. Later, a paid hosted confidential-compute tier behind the same router.

**Ledger and redaction**
- Every cloud call writes a ledger row: feature, provider, bytes sent, message ids.
- Quoted text and signatures are redacted before anything goes to the cloud.

**Retrieval**
- Chunks of 400–600 tokens, with provenance.
- FTS5 BM25 and vector KNN results fused with RRF (k = 60).
- Answers must cite message ids.

**Safety**
- Mail content is untrusted data.
- The model only proposes typed tool calls. A policy outside the model approves them and fails closed.
- Send, delete and forward need confirmation in the app. Agent actions are logged and can be undone.

**Model choice**
- An evaluation harness (A30) on a synthetic corpus (T5) picks the smallest model that passes for each feature.

## Security

**Storage and secrets**
- The SQLCipher key and OAuth tokens live only in the OS keychain. Android uses Keystore through a host callback.

**Rendering mail**
- HTML is sanitised with ammonia.
- It is then rendered in a sandboxed web view: no JavaScript, and no remote content by default.
- Tracker pixels are detected.
- Headers are cleaned of escape sequences and bidi control characters.
- Links show their real target, with lookalike detection.

**Authentication and encryption**
- DKIM, SPF, DMARC and ARC are checked with mail-auth and shown as sender badges.
- OpenPGP uses rPGP, with Autocrypt and WKD. S/MIME is built on RustCrypto `cms`.
- No AGPL or LGPL crates: no sieve-rs, no sequoia.

**Hardening**
- Every parser of untrusted input has a cargo-fuzz target.
- The threat model lives in `docs/threat-model.md` (task C11).

## Bindings and web

| Target | Mechanism | Template in the workspace |
| --- | --- | --- |
| macOS, iOS | UniFFI → Swift, XCFramework | ketch `crates/ketch-ffi`, `scripts/xcframework.sh` |
| Android | UniFFI → Kotlin, cargo-ndk, AAR | cross-code wry-jni, wry-kotlin |
| Windows | UniFFI → C# via the pinned uniffi-bindgen-cs fork | ketch `desktop/windows`, `scripts/csharp.sh` |
| Linux | C ABI with JSON records, cbindgen header, VAPI, meson | ketch `crates/ketch-capi`; scull-ffi drift test |
| Web v1 | `mailune-server`: JSON-RPC over WebSocket, generated TS types | rtok `src/web` |
| Web later | WASM subset with JMAP over fetch and SQLite on OPFS | weft `crates/weft-wasm`; slint-flutter `wasm.rs` |

## Testing

**Code-level tests**
- Unit tests with nextest, rstest and insta.
- Property tests with proptest: sync convergence and op-queue idempotency.
- Scenario replay through `mailune-app` with fake I/O.

**Protocol and integration**
- A scripted IMAP server with provider quirks.
- HTTP fixtures for JMAP, Gmail and Graph.
- Stalwart and Dovecot in docker compose.

**Security and AI**
- Nightly fuzzing.
- AI evaluation with replay cassettes.

**Front ends**
- Contract scenarios that every front end's fake core replays.
- Per-platform UI tests: XCUITest, Compose, UIA, AT-SPI, Playwright.

**Architecture checks**
- The dependency-graph test, the forward-only FFI test and the ABI drift test.

**Performance budgets**

All at 100k messages with one 384-dimension embedding each, on the store as production opens it (SQLCipher on Apple targets). `mailune-store/benches/budgets.rs` measures them; `mailune-store/tests/budgets.rs` checks a small mailbox against ten times each budget on every `nextest` run.

- Thread-list page (50 threads): up to 50 ms.
- Cold open (open the store and show the first list page): up to 300 ms. *Proposed, awaiting creator confirmation.*
- Search (top 10 by embedding, vectors already in memory): up to 100 ms. *Proposed, awaiting creator confirmation.* The first search after open fills the vector cache and is not covered by this budget.
