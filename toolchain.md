# Toolchain

Only what the project uses today. Rows are added in the task that adds the program or package (planned choices are in `docs/architecture.md`).

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| git | system | Version control | https://github.com/git/git |
| rust | mise | Compiler, cargo, rustfmt, clippy | https://github.com/rust-lang/rust |
| cargo-nextest | global | Workspace test runner | https://github.com/nextest-rs/nextest |
| translate-toolkit | mise (`pipx:`) | `i18n/*.po` → native catalogs (`scripts/i18n.py`); its venv Python runs the script | https://github.com/translate/translate |
| node | mise | Runs Vite, Vitest and TypeScript for `web/` | https://github.com/nodejs/node |
| npm | with node | Installs `web/` packages from `web/package-lock.json` | https://github.com/npm/cli |

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| anyhow | local | https://github.com/dtolnay/anyhow | Errors in `mailune-cli` only |
| clap | local | https://github.com/clap-rs/clap | `account add` and `account list` |
| quick-xml | local | https://github.com/tafia/quick-xml | Mozilla autoconfig XML |
| sha2 | local | https://github.com/RustCrypto/hashes | PKCE S256 challenge; blob-store content addresses |
| sha2 | local | https://github.com/RustCrypto/hashes | PKCE S256 challenge; model blob digests |
| base64 | local | https://github.com/marshallpierce/rust-base64 | PKCE base64url verifier and challenge |
| insta | local | https://github.com/mitsuhiko/insta | Committed JSON Schema snapshot; the sanitised-HTML fixture the web reader renders |
| schemars | local | https://github.com/GREsau/schemars | JSON Schema for the contract |
| serde | local | https://github.com/serde-rs/serde | Contract serialization |
| serde_json | local | https://github.com/serde-rs/json | Schema snapshot value and scenario replay JSON |
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
| chrono | local | https://github.com/chronotope/chrono | IMAP SEARCH SINCE dates for the sync window |
| icalendar | local | https://github.com/hoodie/icalendar | iCalendar REQUEST and REPLY |
| pgp | local | https://github.com/rpgp/rpgp | OpenPGP sign, encrypt, decrypt, and verify |
| libfuzzer-sys | local | https://github.com/rust-fuzz/libfuzzer | libFuzzer harness for the fuzz targets |
| diesel | local | https://github.com/diesel-rs/diesel | Typed SQLite access in `mailune-store` |
| diesel_migrations | local | https://github.com/diesel-rs/diesel | Embedded schema migrations in `mailune-store` |
| libsqlite3-sys | local | https://github.com/rusqlite/rusqlite | Bundled SQLCipher on Apple targets, bundled SQLite elsewhere |
| zeroize | local | https://github.com/RustCrypto/utils | Wipes the SQLCipher key text after the PRAGMA |
| tempfile | local | https://github.com/Stebalien/tempfile | Scratch directories for store tests |
| aes-gcm | local | https://github.com/RustCrypto/AEADs | Seals blob-store bodies with a caller-supplied key |
| ews | local | https://github.com/thunderbird/ews-rs | Typed EWS operations and SOAP for on-premises Exchange (MPL-2.0, unmodified) |
| axum | local | https://github.com/tokio-rs/axum | Push relay webhook router |
| tokio | local | https://github.com/tokio-rs/tokio | rmcp runtime in `mailune-mcp`; push relay router tests (dev) |
| tower | local (dev) | https://github.com/tower-rs/tower | `ServiceExt::oneshot` drives the relay router without a socket |
| wasm-bindgen | local | https://github.com/wasm-bindgen/wasm-bindgen | JavaScript exports of `mailune-wasm` |
| getrandom (0.2, `js`) | local (wasm32) | https://github.com/rust-random/getrandom | Browser randomness for rsa's rand in the wasm32 build |
| rmcp | local | https://github.com/modelcontextprotocol/rust-sdk | `mailune-mcp` local MCP server |

npm (`web/package.json`):

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| react | local | https://github.com/facebook/react | Web client UI |
| react-dom | local | https://github.com/facebook/react | Renders the web client into the page |
| typescript | local (dev) | https://github.com/microsoft/TypeScript | Type checks `web/` |
| vite | local (dev) | https://github.com/vitejs/vite | Dev server and production build of `web/` |
| @vitejs/plugin-react | local (dev) | https://github.com/vitejs/vite-plugin-react | JSX and fast refresh in Vite |
| vitest | local (dev) | https://github.com/vitest-dev/vitest | Web tests |
| happy-dom | local (dev) | https://github.com/capricorn86/happy-dom | In-memory DOM for component tests; it opens no socket |
| json-schema-to-typescript | local (dev) | https://github.com/bcherny/json-schema-to-typescript | Generates `web/src/contract.gen.ts` from the contract JSON Schema |
| ajv | local (dev) | https://github.com/ajv-validator/ajv | Validates web payloads against the contract JSON Schema in tests |
| @types/react | local (dev) | https://github.com/DefinitelyTyped/DefinitelyTyped | React types |
| @types/react-dom | local (dev) | https://github.com/DefinitelyTyped/DefinitelyTyped | React DOM types |
| @types/node | local (dev) | https://github.com/DefinitelyTyped/DefinitelyTyped | Node types for the generator script and tests |
