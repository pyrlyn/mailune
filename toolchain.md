# Toolchain

Only what the project uses today. Rows are added in the task that adds the program or package (planned choices are in `docs/architecture.md`).

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| git | system | Version control | https://github.com/git/git |
| rust | mise | Compiler, cargo, rustfmt, clippy | https://github.com/rust-lang/rust |
| cargo-nextest | global | Workspace test runner | https://github.com/nextest-rs/nextest |
| dotnet | system | C# core and the Windows shell | https://github.com/dotnet/sdk |

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| anyhow | local | https://github.com/dtolnay/anyhow | Errors in `mailune-cli` only |
| clap | local | https://github.com/clap-rs/clap | `account add` and `account list` |
| quick-xml | local | https://github.com/tafia/quick-xml | Mozilla autoconfig XML |
| sha2 | local | https://github.com/RustCrypto/hashes | PKCE S256 challenge and model-blob verification |
| base64 | local | https://github.com/marshallpierce/rust-base64 | PKCE base64url verifier and challenge |
| insta | local | https://github.com/mitsuhiko/insta | Committed JSON Schema snapshot |
| schemars | local | https://github.com/GREsau/schemars | JSON Schema for the contract |
| serde | local | https://github.com/serde-rs/serde | Contract serialization and cloud request bodies |
| serde_json | local | https://github.com/serde-rs/json | Schema snapshot value, scenario replay JSON, and structured model output |
| cargo_metadata | local | https://github.com/oli-obk/cargo_metadata | Crate-boundary test over `cargo metadata` |
| syn | local | https://github.com/dtolnay/syn | `//!` header check and forward-only FFI scaffold |
| thiserror | local | https://github.com/dtolnay/thiserror | One error enum per library crate |
| encoding_rs | local | https://github.com/hsivonen/encoding_rs | Canonical charset name for a MIME part |
| mail-builder | local | https://github.com/stalwartlabs/mail-builder | MIME build (text, HTML, attachment, reply headers) |
| mail-parser | local | https://github.com/stalwartlabs/mail-parser | RFC 5322 / MIME parse |
| ammonia | local | https://github.com/rust-ammonia/ammonia | HTML sanitizer for the message body |
| html2text | local | https://github.com/jugglerchris/rust-html2text | Plain text from sanitized HTML |
| rsa | local | https://github.com/RustCrypto/RSA | DKIM rsa-sha256 verification |
| rand | local | https://github.com/rust-random/rand | DKIM test keys and OpenPGP key generation |
| imap-codec | local | https://github.com/duesee/imap-codec | IMAP LIST, commands, and responses |
| icalendar | local | https://github.com/hoodie/icalendar | iCalendar REQUEST and REPLY |
| pgp | local | https://github.com/rpgp/rpgp | OpenPGP sign, encrypt, decrypt, and verify |
| libfuzzer-sys | local | https://github.com/rust-fuzz/libfuzzer | libFuzzer harness for the fuzz targets |
| rmcp | local | https://github.com/modelcontextprotocol/rust-sdk | Local MCP server for read-only tools and gated send |
| tokio | local | https://github.com/tokio-rs/tokio | MCP duplex and the JSON-RPC server runtime |
| axum | local | https://github.com/tokio-rs/axum | WebSocket JSON-RPC server |
| tokio-tungstenite | local | https://github.com/snapview/tokio-tungstenite | WebSocket client for the server test |
| futures-util | local | https://github.com/rust-lang/futures-rs | Send and receive on the test WebSocket |
| whisper-rs | local | https://codeberg.org/tazz4843/whisper-rs | Local speech-to-text for the composer |
| figment | local | https://github.com/SergioBenitez/Figment | Layered TOML load for typed config |
| toml | local | https://github.com/toml-rs/toml | Unknown config keys keep their source line |

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| @playwright/test | local | https://github.com/microsoft/playwright | Browser test that opens the shell and selects a thread |

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| MSTest.Sdk | local | https://github.com/microsoft/testfx | Tests for the C# core |
| Microsoft.WindowsAppSDK | local | https://github.com/microsoft/WindowsAppSDK | WinUI 3 shell for Windows x64 and arm64 |
