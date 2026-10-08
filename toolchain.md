# Toolchain

Only what the project uses today. Rows are added in the task that adds the program or package (planned choices are in `docs/architecture.md`).

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| git | system | Version control | https://github.com/git/git |
| rust | mise | Compiler, cargo, rustfmt, clippy | https://github.com/rust-lang/rust |
| cargo-nextest | global | Workspace test runner | https://github.com/nextest-rs/nextest |

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| anyhow | local | https://github.com/dtolnay/anyhow | Errors in `mailune-cli` only |
| clap | local | https://github.com/clap-rs/clap | `account add` and `account list` |
| insta | local | https://github.com/mitsuhiko/insta | Committed JSON Schema snapshot |
| schemars | local | https://github.com/GREsau/schemars | JSON Schema for the contract |
| serde | local | https://github.com/serde-rs/serde | Contract serialization |
| serde_json | local | https://github.com/serde-rs/json | Schema snapshot value |
| cargo_metadata | local | https://github.com/oli-obk/cargo_metadata | Crate-boundary test over `cargo metadata` |
| syn | local | https://github.com/dtolnay/syn | `//!` header check and forward-only FFI scaffold |
| thiserror | local | https://github.com/dtolnay/thiserror | One error enum per library crate |
