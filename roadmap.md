# Roadmap

Approved work that is not yet in `plan.md`. Approved by the creator on 2026-10-08 together with the blueprint (name Mailune, license as cox, shared-crate extractions, web mode A in v1 with WASM later, push relay, EWS for on-premises later, hosted paid tier later). A phase moves into `plan.md` when the previous one is nearly done. Each line: id, size (S/M/L), depends on, title. Reuse notes and done criteria are in the blueprint and are copied into the card when a task is planned.

## Ph2 macOS on the FFI

- R18 · S · after M18 · Homebrew cask + ketch-registry entry. Reuse: homebrew-tap Casks/ketch.rb; ketch tap.yml; ketch-registry.

## Ph3 AI v1 (local-first)

- X5 · S · Consume llm-wire + llm-http from packages/crates (aulo S1 T1.1, T1.2, in flight). Reuse: packages/crates llm-wire, llm-http (extracted from cox-provider-http).
- X6 · S · after X5 · Consume llm-openai from packages/crates (aulo S1 T1.3, in flight). Reuse: packages/crates llm-openai.
- X7 · S · after X5 · Consume llm-anthropic from packages/crates (aulo S1 T1.4, in flight). Reuse: packages/crates llm-anthropic.
- X8 · M · after X5 · Extract llm-testkit (replay/scripted cassettes with secret redaction). Reuse: cox-provider-testkit.

## Ph5 Other platforms

- D13 · M · after D1, R14 · Release: signed AAB to Play internal track; F-Droid feasibility note. Reuse: R14 new reusable workflow.
- I8 · M · after I1, R13 · TestFlight pipeline. Reuse: R13 new reusable workflow.
- R11 · M · after R10 · pyrlyn/ci: reusable Windows signing + MSIX workflow. Reuse: NEW (gap in pyrlyn/ci; listed in docs/centralization-candidates.md style).
- R12 · M · after R10 · pyrlyn/ci: reusable Flatpak workflow. Reuse: NEW (gap in pyrlyn/ci).
- R13 · M · after R10 · pyrlyn/ci: reusable iOS TestFlight workflow (App Store Connect API). Reuse: pyrlyn/ci macos-sign action as a base.
- R14 · M · after R10 · pyrlyn/ci: reusable Android Play workflow (signed AAB). Reuse: NEW.
- W13 · M · after W1, R11 · Release: MSIX, code signing, winget manifest. Reuse: R11 new reusable workflow.

## Later

- A33 · M · after A6, A7 · Paid hosted AI tier (later): confidential-compute provider behind the same router. Reuse: NEW; design first, then a provider adapter next to BYOK.
- P27 · L · after P23, P12 · EWS for on-premises Exchange (later; Exchange Online stays on Graph). Reuse: NEW; survey Thunderbird ews-rs (MPL-2.0) before writing a client.
