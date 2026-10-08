# Roadmap

Approved work that is not yet in `plan.md`. Approved by the creator on 2026-10-08 together with the blueprint (name Mailune, license as cox, shared-crate extractions, web mode A in v1 with WASM later, push relay, EWS for on-premises later, hosted paid tier later). A phase moves into `plan.md` when the previous one is nearly done. Each line: id, size (S/M/L), depends on, title. Reuse notes and done criteria are in the blueprint and are copied into the card when a task is planned.

## Ph1 Core mail over IMAP and SMTP

- P7 · M · after P6, S3 · IMAP initial sync: UIDVALIDITY, batched envelope/flags/BODYSTRUCTURE, day window. Reuse: NEW.
- P8 · L · after P7 · IMAP incremental sync: CONDSTORE/QRESYNC (CHANGEDSINCE, VANISHED) + diff fallback. Reuse: NEW.
- P9 · M · after P7 · IMAP IDLE push with reconnect and backoff. Reuse: NEW.
- P10 · S · after P7, S4 · Lazy body and attachment fetch (BODY.PEEK partial, BINARY). Reuse: NEW.
- P11 · M · after P7 · IMAP mutations: STORE flags, MOVE (or COPY+EXPUNGE), APPEND, UIDPLUS mapping. Reuse: NEW.
- P12 · L · after S2, F7 · Persist the operation queue (replay, idempotency, conflict policy, undo window) in the store. The in-memory state machine is already in mailune-core. Reuse: NEW.
- R8 · M · after P7, R1 · Integration env: Stalwart + Dovecot in docker compose (PR subset, nightly full). Reuse: NEW.
- S1 · M · after F2, C1 · mailune-store: Diesel + bundled SQLCipher, WAL, embedded migrations, key from SecretStore. Reuse: cox-store; rtok src/store/mod.rs.
- S2 · M · after S1, F7 · Schema v1 migrations: accounts, mailboxes, messages, memberships, threads, parts, flags, sync_state, ops, contacts. Reuse: cox-store migrations layout; optional prisma-toolkit Diesel generator for models.rs/schema.rs.
- S3 · M · after S2 · Repository API in typed Diesel DSL: upserts, thread queries, cursor paging, counts. Reuse: cox-store models.rs/schema.rs patterns.
- S4 · M · after S1 · Blob store for bodies/attachments: content-addressed, encrypted, quota + eviction. Reuse: NEW; sha2 (rust.md).
- S5 · S · after S3 · FTS5 index over subject, addresses, body text (via sql_query inside store crate). Reuse: cox-store src/fts.rs; rtok FTS5 migrations.
- S7 · S · after S3, X11 · Change feed → typed invalidation events for view models. Reuse: X11 sqlite-change-feed (cox-store watch.rs).
- S12 · S · after S3 · Storage benchmarks: 100k insert, list page, search. Reuse: divan (rust.md).
- T4 · M · after P12, P8 · Sync property tests: random ops vs model mailbox, convergence invariants. Reuse: proptest (rust.md).

## Ph2 macOS on the FFI

- B2 · M · after B1, F6 · mailune-ffi: UniFFI 0.32 records, errors, async functions, callback interfaces. Reuse: ketch-ffi (callbacks.rs, records.rs, error.rs); cox-ffi (staticlib, bundled uniffi-bindgen).
- B3 · S · after B2 · Swift packaging: XCFramework (macOS arm64, iOS, simulator) + SwiftPM MailuneCore. Reuse: ketch scripts/xcframework.sh; cross-code tools/build-xcframework.sh.
- M1 · S · after B3 · Xcode project via XcodeGen + SPM packages (MailuneModel, MailuneUI, MailunePlatform), arm64, macOS 26. Reuse: cox desktop/macos layout; ketch desktop/macos.
- M2 · S · after M1 · Design tokens → Swift (colors, type, spacing, radii, motion). Reuse: cox/ketch desktop/design (Style Dictionary); weft-swiftui theme.rs; research/mail-app theme.slint + brand tokens.css.
- M3 · S · after M1 · Icon set: apple variant SVGs → asset catalog. Reuse: research/mail-app ui/icons/apple + tools/gen-icons.py.
- M4 · S · after M1 · Localisation: .po → .xcstrings. Reuse: forks/translate; research/mail-app lang/*.po.
- M5 · M · after M1, B2 · Host integrations: Keychain, UserNotifications, NWPathMonitor, ASWebAuthenticationSession, open URL. Reuse: cox-ffi AppHost Swift side.
- M7 · M · after M2, M3 · Shell: NavigationSplitView three panes, toolbar, keyboard shortcuts, command palette. Reuse: research/mail-app app.slint, drawer.slint.
- M8 · M · after M7, B7 · Thread list: virtualised, swipe actions, multi-select, indicators, category tabs. Reuse: research/mail-app mail_list.slint.
- M9 · L · after M8, P3, C6 · Reader: locked-down WKWebView (no JS, no remote), quotes collapse, attachments, security badges. Reuse: research/mail-app thread.slint.
- M10 · L · after M7, P13, S13 · Composer: recipient chips, rich text, attachments, send later, undo send. Reuse: research/mail-app compose.slint, sheets.slint (schedule).
- M12 · M · after M7 · Settings: accounts, appearance, notifications, reading, compose, sync. Reuse: research/mail-app settings.slint.
- M13 · M · after M12, P14, P15 · Onboarding and account setup (autoconfig + OAuth). Reuse: research/mail-app auth.slint (AuthShell).
- M14 · M · after M8 · macOS integration: mailto handler, Dock badge, Share extension, Spotlight (opt-in). Reuse: NEW.
- M16 · S · after M9, M10 · Accessibility audit: VoiceOver, keyboard-only, contrast. Reuse: scull macOS accessibility code.
- M17 · M · after M8, B7 · XCUITests on contract scenarios with fake core. Reuse: ketch macOS UI tests.
- M18 · S · after M13, R10 · Release: Developer ID, notarise, DMG, Sparkle appcast. Reuse: pyrlyn/ci release-apple-desktop.yml (used by ketch).
- R3 · S · after M1, R1 · Swift CI: xcodebuild test (macOS + iOS sim). Reuse: pyrlyn/ci setup-xcode; cox desktop-macos action.
- R18 · S · after M18 · Homebrew cask + ketch-registry entry. Reuse: homebrew-tap Casks/ketch.rb; ketch tap.yml; ketch-registry.
- T6 · S · after S12, B1 · Performance budgets: cold start, 100k sync, list scroll. Reuse: divan, hyperfine (rust.md).

## Ph3 AI v1 (local-first)

- A3 · M · after A1 · Local engine adapter: generation, embeddings, JSON-structured output. Reuse: runa-engine (embed.rs, structured.rs, prompt cache).
- A4 · M · after A3 · Model manager: catalog, SHA-256 verified resumable downloads, storage, delete. Reuse: aulo-models downloader; runa-fit memory planner; hf-hub (rust.md).
- A6 · M · after A1, X6, X7, C1 · Cloud BYOK providers: OpenAI-compatible (Ollama, LM Studio, Foundry Local) and Anthropic. Reuse: X5–X7 llm-* crates; runa-cloud routing.rs.
- A11 · M · after A2, A3, A9, A10, P3 · Thread summary: short, detailed, action items; cached by content hash. Reuse: NEW.
- A12 · M · after A11 · Daily digest (“Today” brief) since last open. Reuse: NEW.
- A13 · S · after A11 · Smart reply suggestions (3 options). Reuse: NEW.
- A14 · M · after A2, A10 · Compose assist: draft from prompt, rewrite, tone, shorten, proofread. Reuse: NEW.
- A20 · M · after S9, A32, A2 · Ask your inbox: RAG with citations. Reuse: research/noema; S9.
- A23 · S · after A2 · Translation of messages and drafts with language detection. Reuse: NEW.
- A30 · M · after X8, T5 · AI evaluation harness: per-feature metrics, cassettes, CI regression gate. Reuse: X8 llm-testkit (cox-provider-testkit).
- A32 · M · after A3, S8, S10 · Background embedding pipeline with model choice by eval. Reuse: research/noema (bge-m3 / Qwen3-Embedding GGUF candidates).
- M6 · M · after M5, A5 · Apple Foundation Models implementation of PlatformModel (guided generation → JSON). Reuse: NEW.
- M11 · M · after M8, S9, A20 · Search UI with filter tokens and Ask mode with linked citations. Reuse: research/mail-app search.slint.
- M15 · S · after M11 · App Intents / Shortcuts: summarise, search, compose. Reuse: NEW.
- M19 · M · after M9, M10, A11, A13, A14, A16 · AI surfaces: summary card, smart replies, triage tabs, compose assist, privacy panel + ledger view. Reuse: research/mail-app sparkle icon, category tabs.
- S8 · M · after S3 · Embedding store: vectors in SQLite + cosine KNN. Reuse: rtok src/store/embed.rs (avoids sqlite-vec, which bundled Diesel SQLite cannot load).
- S9 · M · after S5, S6, S8 · Hybrid retrieval: FTS5 BM25 + vector KNN fused with RRF (k=60), filters, citations. Reuse: research/noema retrieval design.
- X5 · S · Consume llm-wire + llm-http from packages/crates (aulo S1 T1.1, T1.2, in flight). Reuse: packages/crates llm-wire, llm-http (extracted from cox-provider-http).
- X6 · S · after X5 · Consume llm-openai from packages/crates (aulo S1 T1.3, in flight). Reuse: packages/crates llm-openai.
- X7 · S · after X5 · Consume llm-anthropic from packages/crates (aulo S1 T1.4, in flight). Reuse: packages/crates llm-anthropic.
- X8 · M · after X5 · Extract llm-testkit (replay/scripted cassettes with secret redaction). Reuse: cox-provider-testkit.

## Ph4 Providers

- P17 · L · after S3, P12 · JMAP read sync: session, Mailbox/Email/Thread get, /changes, /query. Reuse: NEW; jmap-client.
- P18 · S · after P17 · JMAP push: EventSource / WebSocket (RFC 8887). Reuse: NEW.
- P19 · M · after P17 · JMAP mutations and EmailSubmission send. Reuse: NEW.
- P20 · M · after P19 · JMAP extras: MaskedEmail create from composer, Sieve script list/edit (RFC 9661). Reuse: NEW.
- P21 · L · after P15, S3 · Gmail API read sync: threads/labels, historyId incremental, batch fetch. Reuse: NEW (thin reqwest; google-gmail1 rejected as heavy).
- P22 · M · after P21, P2 · Gmail API mutations: batchModify labels, send, drafts. Reuse: NEW.
- P23 · L · after P15, S3 · Microsoft Graph mail sync: folders, delta queries, $select. Reuse: NEW (no official Rust SDK; graph-rs-sdk stale).
- P24 · M · after P23 · Graph mutations and send: move, flags/categories, sendMail, $batch. Reuse: NEW.
- P25 · M · after P23 · Graph calendar and contacts read (availability, autocomplete). Reuse: NEW.

## Ph5 Other platforms

- B4 · M · after B2 · Kotlin packaging: cargo-ndk (arm64-v8a, x86_64) + AAR with UniFFI Kotlin. Reuse: cross-code wry-jni (cargo-ndk), wry-kotlin Gradle.
- B5 · S · after B2 · C# packaging: uniffi-bindgen-cs (pinned fork for 0.32) → Mailune.Core project. Reuse: ketch scripts/csharp.sh, uniffi.toml, desktop/windows KetchCore.csproj.
- B6 · M · after B1, X9 · mailune-capi: JSON-record C ABI for Vala, cbindgen header, VAPI, meson, drift test. Reuse: ketch-capi (abi.rs, cbindgen.toml, vapi, meson.build, tests/capi.vala); scull-ffi event queue; slint-bindings sb_last_error.
- B9 · L · after P1, P4, S6 · mailune-wasm: subset build (protocol, MIME, threading, query parser) + engine-parity test. Reuse: weft crates/weft-wasm + moon wasm task; slint-flutter native/rust/wasm.rs envelope.
- D1 · S · after B4 · Gradle project: Compose + Material 3; :core (AAR), :ui, :app. Reuse: cross-code wry-kotlin Gradle setup.
- D2 · S · after D1 · Tokens → Compose theme; material icon variant → VectorDrawables. Reuse: research/mail-app icons/material + theme.slint.
- D3 · S · after D1 · Localisation: .po → strings.xml. Reuse: forks/translate.
- D4 · M · after D1 · Host: Keystore-wrapped secrets, WorkManager sync, notification channels, Custom Tabs OAuth. Reuse: NEW.
- D5 · M · after D4, A5 · Gemini Nano via ML Kit GenAI as PlatformModel (capability checks, fallback). Reuse: NEW.
- D6 · M · after D2 · Adaptive shell: ListDetailPaneScaffold for phone/tablet/foldable. Reuse: research/mail-app breakpoints.
- D7 · M · after D6, B7 · Thread list. Reuse: research/mail-app mail_list.slint.
- D8 · M · after D7, P3 · Reader (sandboxed WebView). Reuse: research/mail-app thread.slint.
- D9 · M · after D6, P13 · Composer. Reuse: research/mail-app compose.slint.
- D10 · M · after D8, D9, A11, A14 · AI surfaces. Reuse: M19 spec.
- D11 · S · after D7 · Widgets (Glance), share target, mailto intent filter. Reuse: NEW.
- D12 · M · after D7, B7 · Compose UI tests + screenshot tests on contract scenarios. Reuse: NEW.
- D13 · M · after D1, R14 · Release: signed AAB to Play internal track; F-Droid feasibility note. Reuse: R14 new reusable workflow.
- E1 · M · after B8 · mailune-server: axum WebSocket JSON-RPC over mailune-app, token/passkey auth, single user. Reuse: rtok src/web (axum WS, rust-embed SPA); aulo-server auth/token.rs.
- E2 · S · after B8 · Frontend scaffold: TypeScript + Vite + React with generated types. Reuse: rtok web/ (Vite, vitest, Playwright); weft render-react.
- E3 · S · after E2 · Tokens → CSS variables; web icon set. Reuse: research/mail-app docs/brand/tokens.css; @pyrlyn/brand build.mjs.
- E4 · S · after E2 · Localisation: .po → JSON catalogs. Reuse: forks/translate.
- E5 · M · after E3, E1 · Shell + thread list. Reuse: research/mail-app app.slint, mail_list.slint.
- E6 · M · after E5, P3 · Reader in sandboxed iframe (srcdoc + strict CSP). Reuse: research/mail-app thread.slint.
- E7 · M · after E5 · Composer. Reuse: research/mail-app compose.slint.
- E8 · M · after E6, E7, A11, A14 · AI surfaces. Reuse: M19 spec.
- E9 · M · after E5 · PWA: offline cache of recent threads, Web Push from server. Reuse: NEW.
- E10 · M · after B9, E5, P17 · WASM JMAP-direct mode (mode B). Reuse: B9; weft engine-parity test.
- E11 · M · after E5, B7 · Playwright e2e on contract scenarios. Reuse: rtok web Playwright setup.
- E12 · S · after E1 · Container image + self-hosting docs. Reuse: NEW.
- I1 · M · after M7 · iOS target sharing MailuneModel/MailuneUI; compact stack + iPad split layouts. Reuse: M1 packages; research/mail-app phone/tablet layouts.
- I2 · M · after I1, M5 · Host: Keychain access group, BGAppRefresh/BGProcessing sync, notifications. Reuse: M5 Swift host.
- I3 · M · after I1, M8 · Mobile thread list: swipe actions, pull to refresh, selection mode. Reuse: research/mail-app mail_list.slint (phone).
- I4 · M · after I3, M9 · Mobile reader. Reuse: research/mail-app thread.slint.
- I5 · M · after I1, M10 · Mobile composer. Reuse: research/mail-app compose.slint.
- I6 · S · after M6, A4 · On-device AI on iOS: Foundation Models + model-size policy (embeddings only by default). Reuse: M6.
- I7 · M · after I2 · Extensions: share, notification service (local decrypt/preview), widgets. Reuse: NEW.
- I8 · M · after I1, R13 · TestFlight pipeline. Reuse: R13 new reusable workflow.
- I9 · S · after I3, B7 · XCUITests on contract scenarios. Reuse: M17.
- I10 · S · after I4, I5, M19 · AI surfaces (mobile variants). Reuse: M19.
- L1 · S · after B6 · Meson + Vala + GTK4/libadwaita project linked to mailune-capi. Reuse: ketch-capi meson.build, vapi, tests/capi.vala.
- L2 · M · after L1 · Payload decoding (json-glib) generated from payloads schema. Reuse: ketch-capi schema/payloads.schema.json approach.
- L3 · S · after L1 · Tokens → GTK CSS (libadwaita named colours); adwaita icon variant. Reuse: @pyrlyn/brand CSS output; research/mail-app icons/adwaita.
- L4 · S · after L1 · Localisation: gettext .po used natively. Reuse: research/mail-app lang/*.po.
- L5 · M · after L2 · Host: Secret Service (via keyring), GNotification, GNetworkMonitor, OAuth via portal + loopback. Reuse: NEW.
- L6 · M · after L3 · Shell: AdwNavigationSplitView three panes, adaptive breakpoints. Reuse: research/mail-app app.slint.
- L7 · M · after L6, L2 · Thread list (GtkListView + factories). Reuse: research/mail-app mail_list.slint.
- L8 · M · after L7, P3 · Reader (WebKitGTK, JS off, remote blocked). Reuse: research/mail-app thread.slint.
- L9 · M · after L6, P13 · Composer. Reuse: research/mail-app compose.slint.
- L10 · M · after L8, L9, A11, A14 · AI surfaces (bundled llama.cpp or local endpoint; no OS model). Reuse: M19 spec.
- L11 · S · after L7 · Desktop integration: .desktop, mailto handler, background portal. Reuse: NEW.
- L12 · M · after L7, B7 · UI tests via AT-SPI on contract scenarios. Reuse: NEW.
- L13 · M · after L1, R12 · Flatpak manifest + Flathub submission prep. Reuse: R12 new reusable workflow.
- R4 · S · after W1 · Windows .NET CI. Reuse: pyrlyn/ci ci-dotnet.yml.
- R5 · M · after D1 · Android CI: Gradle build, unit + emulator tests. Reuse: NEW.
- R6 · M · after L1 · Linux GTK CI: meson build, Vala tests. Reuse: NEW.
- R7 · S · after E2 · Web CI: vitest + Playwright. Reuse: rtok web workflows.
- R11 · M · after R10 · pyrlyn/ci: reusable Windows signing + MSIX workflow. Reuse: NEW (gap in pyrlyn/ci; listed in docs/centralization-candidates.md style).
- R12 · M · after R10 · pyrlyn/ci: reusable Flatpak workflow. Reuse: NEW (gap in pyrlyn/ci).
- R13 · M · after R10 · pyrlyn/ci: reusable iOS TestFlight workflow (App Store Connect API). Reuse: pyrlyn/ci macos-sign action as a base.
- R14 · M · after R10 · pyrlyn/ci: reusable Android Play workflow (signed AAB). Reuse: NEW.
- W1 · S · after B5 · WinUI 3 solution: Mailune.App, Mailune.AppCore, Mailune.Core; x64 + arm64. Reuse: ketch desktop/windows (Ketch.App, Ketch.AppCore).
- W2 · S · after W1 · Tokens → XAML resources; fluent icon variant. Reuse: ketch desktop/design (XAML tokens); research/mail-app icons/fluent.
- W3 · S · after W1 · Localisation: .po → .resw. Reuse: forks/translate (po2resx path).
- W4 · M · after W1 · Host: notifications (AppNotificationManager), network status, OAuth loopback/broker. Reuse: NEW.
- W5 · M · after W4, A5 · Windows AI APIs as PlatformModel (Phi Silica → Aion Instruct transition); Foundry Local fallback. Reuse: NEW.
- W6 · M · after W2 · Shell: NavigationView three panes, Mica, accelerators. Reuse: research/mail-app app.slint.
- W7 · M · after W6, B7 · Thread list. Reuse: research/mail-app mail_list.slint.
- W8 · M · after W7, P3 · Reader (locked-down WebView2). Reuse: research/mail-app thread.slint.
- W9 · M · after W6, P13 · Composer. Reuse: research/mail-app compose.slint.
- W10 · M · after W8, W9, A11, A14 · AI surfaces. Reuse: M19 spec.
- W11 · S · after W7 · Integration: default mail app, toasts with actions, jump list, badge. Reuse: NEW.
- W12 · M · after W7, B7 · UI tests (UIA/Appium) on contract scenarios. Reuse: pyrlyn/ci ci-dotnet.yml.
- W13 · M · after W1, R11 · Release: MSIX, code signing, winget manifest. Reuse: R11 new reusable workflow.

## Ph6 Trust, agents, AI v2, push relay

- C7 · S · after P1 · DKIM remainder: ed25519, simple canonicalization, and the `l=` body-length tag. rsa-sha256 relaxed/relaxed already verifies in `mailune-mime`. Reuse: the verifier already there.
- A15 · M · after A14, S3 · Style profile learned locally from Sent (per-recipient tier). Reuse: NEW.
- A19 · M · after A16, A9 · Natural-language rules → typed rule DSL, preview matches before enabling. Reuse: NEW.
- A22 · M · after A2, P26, P25 · Scheduling extraction: dates, times, intents → ICS suggestion with availability. Reuse: NEW; jiff (rust.md).
- A25 · M · after C7, C8, A2 · Phishing and scam assessment (auth results + links + model). Reuse: NEW.
- A27 · M · after A11 · Attachment summarisation (text/PDF extraction). Reuse: NEW (crate survey needed).
- A28 · L · after A9, P12, B1 · Agent tools layer: typed tools with scopes, preview, undo, audit log. Reuse: cox-permission; cox-tools patterns.
- A29 · M · after A28 · Local MCP server with per-folder/label scopes, read-only default, in-app send approval. Reuse: cox-mcp (rmcp); weft packages/mcp; rtok-mcp host config install.
- A31 · S · after A1 · Voice dictation in composer (whisper-rs). Reuse: cox-voice; runa-media ASR.
- C3 · M · after C2 · Autocrypt headers and gossip, WKD key discovery. Reuse: NEW.
- C4 · M · after P1 · S/MIME verify and decrypt (cms + x509-cert). Reuse: NEW.
- C5 · M · after C4, P2 · S/MIME sign and encrypt. Reuse: NEW.
- P31 · M · after P21, P23 · Push relay service: Gmail Pub/Sub and Graph webhooks to empty APNs/FCM wakes. Reuse: NEW (Mimestream Private Push pattern); axum from rtok src/web.
- P32 · S · after P31, P16 · Relay client: register devices, wake triggers sync. Reuse: NEW.

## Later

- A33 · M · after A6, A7 · Paid hosted AI tier (later): confidential-compute provider behind the same router. Reuse: NEW; design first, then a provider adapter next to BYOK.
- P27 · L · after P23, P12 · EWS for on-premises Exchange (later; Exchange Online stays on Graph). Reuse: NEW; survey Thunderbird ews-rs (MPL-2.0) before writing a client.
