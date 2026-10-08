# Mailune — instructions for agents

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too; on conflict, ask the creator.

**What this is.** A local-first, AI-first mail client. One Rust core owns protocols (IMAP/SMTP, JMAP, Gmail API, Microsoft Graph), sync, storage and search, AI orchestration and crypto. Native shells sit on top: SwiftUI (macOS, iOS), WinUI 3 (Windows), GTK4 + libadwaita in Vala (Linux), Jetpack Compose (Android), and a TypeScript web client served by `mailune-server`.

**Read first.**

- `plan.md`: active tasks.
- `roadmap.md`: approved tasks by phase.
- `docs/architecture.md`: crate map, data flow, sync model, AI pipeline, security, testing.
- `research.md`: the competitive research and its sources.

Do not add work that is not in `plan.md` or `roadmap.md`; propose it in `ideas.md`.

## Decisions (creator, 2026-10-08)

- **Name and license.** The project is named Mailune. It is licensed like cox: `GPL-3.0-or-later OR LicenseRef-Mailune-Royalty-Free`, with a commercial license on request (`LICENSE`, `LICENSE-ROYALTY-FREE.md`, `PRICING.md`).
- **Shared code.** Code that Mailune shares with other projects is extracted into `packages/crates` (roadmap X tasks). Crates that are already being extracted there are consumed, not duplicated: llm-wire, llm-http, llm-openai, llm-anthropic, text-sanitize, config-schema.
- **Web.** v1 ships the self-hosted `mailune-server`: the native core behind JSON-RPC over WebSocket. The WASM JMAP-direct mode comes later and is already in the roadmap.
- **Push.** An optional push relay is built. It never sees tokens or mail content.
- **Exchange.** Exchange Online is reached through Microsoft Graph. EWS for on-premises Exchange comes later.
- **AI and monetisation.**
  - AI runs locally by default. Cloud AI is bring-your-own-key and opt-in per feature.
  - A paid hosted AI tier comes later.
  - Encrypted mail is never sent to a cloud model.

## Rules

The workspace `rust.md` applies in full. The rules below are the ones that shape this project most.

**Architecture and crates**

- Crates are named `crates/mailune-<role>`.
- Dependencies point one way: contract → pure domain → adapters → assembly (`mailune-app`) → surfaces. A `cargo metadata` test enforces this.
- Network, filesystem and process I/O sit behind traits in `mailune-protocol`. Each heavy dependency has one owning crate.
- Surfaces (`mailune-ffi`, `mailune-capi`, `mailune-rpc`, `mailune-wasm`, `mailune-cli`, `mailune-mcp`) only forward calls. A syn test checks this.

**Code style**

- Every file starts with a `//!` header.
- No `unwrap`, `expect` or `panic!` outside tests.
- One `thiserror` enum per crate. `anyhow` is allowed only in binaries.
- Tests live in `mod tests` at the bottom of the file.

**Data and untrusted input**

- No raw SQL. Use Diesel. FTS5 statements through `sql_query` are allowed only inside `mailune-store`, with a comment saying why.
- Mail content, model output, MCP clients and plugins are untrusted.
- The model only proposes typed tool calls. A policy outside the model approves them and fails closed.
- Send, delete and forward always need confirmation in the app.

**Secrets and tests**

- Secrets live only in the OS keychain.
- Tests never touch the real keychain or the network.

**Platforms and dependencies**

- macOS builds target arm64 only.
- Bumping a pinned version needs the creator's permission.
- A new maintained dependency gets a row in `toolchain.md` and in the workspace `rust.md` in the same change.

## Commands

The workspace is created in task F2. Until then there is nothing to build. Once it exists:

```bash
mise exec -- cargo nextest run --workspace
mise exec -- cargo clippy --workspace --all-targets -- -D warnings
mise exec -- cargo fmt --all -- --check
```

Localisation: `i18n/mailune.pot` and `i18n/<lang>.po` are the only source. Add a string there with a msgctxt key, never in a native catalog.

```bash
mise run i18n          # native catalogs into target/i18n
mise run i18n --check  # validate keys, placeholders and plural forms
mise run i18n:test
```
