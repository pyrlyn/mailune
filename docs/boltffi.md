# BoltFFI survey

Checked 2026-10-08. This note compares BoltFFI 0.31 with the UniFFI bindings already in this tree. It does not switch the binding tool and it does not add a dependency.

## Sources

- BoltFFI repository: https://github.com/boltffi/boltffi — README on the default branch, fetched 2026-10-08.
- BoltFFI 0.31.0: tag `v0.31.0`, published 2026-09-28. Release notes: https://github.com/boltffi/boltffi/releases/tag/v0.31.0
- UniFFI pin in this workspace: `uniffi` 0.32.2 (`Cargo.toml`). Surface: `crates/mailune-ffi`.

The releases page fetched the same day also lists `v0.25.0` (2026-05-11) among the twenty most recent tags. That page is not a full history.

## What this tree binds today

`mailune-ffi` is a UniFFI 0.32 surface. `uniffi::setup_scaffolding!` and `#[uniffi::export]` forward into `api`. The exported shape is small:

- `ThreadRecord`, a record of two strings and a bool.
- `FfiError`, an error enum with one string field.
- `ThreadSink`, a callback interface.
- `core_ready`, an async function whose future finishes immediately.
- `publish_threads`, a function that hands records to that callback.

`desktop/macos/scripts/xcframework.sh` builds one `aarch64-apple-darwin` static library and runs the crate's `uniffi-bindgen` for Swift. It does not build iOS or Intel. The crate type is `lib` plus `staticlib`. The bindgen binary is behind the `bindgen` feature so the static library does not compile the CLI.

## What BoltFFI 0.31 is

BoltFFI generates bindings from `#[data]` on types and `#[export]` on functions (`boltffi` crate, `boltffi_cli`). The README (fetched 2026-10-08) lists Swift, Kotlin, Java, C#, WASM/TypeScript, and Python as full support. C is experimental and synchronous. Kotlin Multiplatform is experimental and does not cover iOS or macOS. `boltffi pack all` is described as producing an Apple xcframework, Android and Java libraries, a WASM package, a C# package, and a Python wheel.

The 0.31.0 notes (published 2026-09-28) highlight an experimental C backend and faster `#[data]` macro expansion. They also record fixes for class ownership, synchronous callbacks that take an owned class, callback exceptions, Swift `try` on unit results, and several Kotlin, Java, C#, and WASM fixes. Async and callbacks are already part of the tool; 0.31 does not introduce them.

The README publishes a speed table against UniFFI (noop, echo, counter, struct batches). Those figures are the project's own benchmark. This repository has not reproduced them.

## Comparison

| | UniFFI 0.32.2, as wired here | BoltFFI 0.31.0 |
| --- | --- | --- |
| Attributes | `#[uniffi::export]`, `uniffi::Record`, `uniffi::Error`, callback interface | `#[data]`, `#[export]` |
| Records, errors, async, callbacks | Used by `mailune-ffi` today | Documented; 0.31 fixes callback and Swift error paths |
| Swift packaging | One arm64 macOS static library and `uniffi-bindgen` | `boltffi pack` builds an xcframework and other targets together |
| C ABI for Vala | Not this crate (`cbindgen` is the planned C surface) | Experimental C target in 0.31 |
| Extra runtimes | None in this crate | Java, Python, and WASM ship from the same pack command |

A switch would rewrite the five exports onto another attribute set, replace `xcframework.sh`, and take a new CLI. The current surface does not use a UniFFI feature that BoltFFI 0.31 lacks: records, an error enum, one callback, and one immediately-ready async function are all inside what both tools describe.

The speed claim matters if a shell made thousands of FFI calls per frame. Thread rows and a readiness check are not that workload. The 0.31 headline (experimental C, faster macros) does not change the macOS shell.

`boltffi pack all` also emits Android, JVM, WASM, C#, and Python. This tree's macOS script is deliberately arm64-only. Pulling the pack command in would widen the build past that script for no gain on the exports that exist.

## Decision

Stay on UniFFI. BoltFFI 0.31 is a maintained generator with the languages the later shells need, and it is reasonable to look at again when a second language binding is actually being generated. It is not worth switching the working arm64 Swift surface for it.
