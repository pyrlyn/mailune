# Toolchain

Only what the project uses today. Rows are added in the task that adds the program or package (planned choices are in `docs/architecture.md`).

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| git | system | Version control | https://github.com/git/git |
| rust | mise | Compiler, cargo, rustfmt, clippy | https://github.com/rust-lang/rust |
| cargo-nextest | global | Workspace test runner | https://github.com/nextest-rs/nextest |
| translate-toolkit | mise (`pipx:`) | `i18n/*.po` → native catalogs (`scripts/i18n.py`); its venv Python runs the script | https://github.com/translate/translate |
| java | mise (`desktop/android/mise.toml`) | Runs Gradle for the Android shell | https://openjdk.org |
| gradle | mise (`desktop/android/mise.toml`) | Builds and tests `desktop/android` | https://github.com/gradle/gradle |
| kotlin | mise (`desktop/android/mise.toml`) | Kotlin compiler for the Android shell | https://github.com/JetBrains/kotlin |
| meson | system | Builds and runs the Vala test of `mailune-capi` | https://github.com/mesonbuild/meson |
| vala | system (with GLib, GObject and json-glib) | Compiles `crates/mailune-capi/tests/capi.vala` against the VAPI | https://gitlab.gnome.org/GNOME/vala |
| openssl | system | Regenerates the `mailune-smime` interop fixtures (`fixtures/regenerate.sh`); not needed to build or test | https://github.com/openssl/openssl |

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| anyhow | local | https://github.com/dtolnay/anyhow | Errors in the `mailune-cli` and `mailune-server` binaries only |
| clap | local | https://github.com/clap-rs/clap | `account add` and `account list` |
| quick-xml | local | https://github.com/tafia/quick-xml | Mozilla autoconfig XML |
| sha2 | local | https://github.com/RustCrypto/hashes | PKCE S256 challenge |
| base64 | local | https://github.com/marshallpierce/rust-base64 | PKCE base64url verifier and challenge |
| insta | local | https://github.com/mitsuhiko/insta | Committed JSON Schema snapshot |
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
| rsa | local | https://github.com/RustCrypto/RSA | DKIM rsa-sha256 verification; S/MIME signatures and key transport |
| rand | local | https://github.com/rust-random/rand | DKIM test keys, OpenPGP key generation, S/MIME content keys, `mailune-server` tokens |
| imap-codec | local | https://github.com/duesee/imap-codec | IMAP LIST, commands, and responses |
| icalendar | local | https://github.com/hoodie/icalendar | iCalendar REQUEST and REPLY |
| pgp | local | https://github.com/rpgp/rpgp | OpenPGP sign, encrypt, decrypt, and verify |
| libfuzzer-sys | local | https://github.com/rust-fuzz/libfuzzer | libFuzzer harness for the fuzz targets |
| uniffi | local | https://github.com/mozilla/uniffi-rs | `mailune-ffi`: Swift, Kotlin and C# bindings from proc-macros |
| libc | local | https://github.com/rust-lang/libc | `mailune-capi`: `malloc`ed answers GLib can `g_free` |
| cbindgen | local (dev) | https://github.com/mozilla/cbindgen | `mailune-capi`: header drift test |
| cms | local | https://github.com/RustCrypto/formats/tree/master/cms | `mailune-smime`: CMS SignedData and EnvelopedData |
| x509-cert | local | https://github.com/RustCrypto/formats/tree/master/x509-cert | `mailune-smime`: signer and recipient certificates |
| aes | local | https://github.com/RustCrypto/block-ciphers | `mailune-smime`: AES content cipher |
| cbc | local | https://github.com/RustCrypto/block-modes | `mailune-smime`: CBC mode for S/MIME content |
| axum | local | https://github.com/tokio-rs/axum | `mailune-server`: HTTP and the WebSocket upgrade |
| tokio | local | https://github.com/tokio-rs/tokio | `mailune-server`: async runtime and TCP listener |
| tokio-tungstenite | local (dev) | https://github.com/snapview/tokio-tungstenite | `mailune-server` tests: real WebSocket client; the line axum already uses |
| futures-util | local (dev) | https://github.com/rust-lang/futures-rs | `mailune-server` tests: send and receive on the client socket |
| proptest | local (dev) | https://github.com/proptest-rs/proptest | `mailune-core` tests: random op sequences against a model mailbox |

Gradle (`desktop/android`):

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| org.jetbrains.kotlin.jvm | local | https://github.com/JetBrains/kotlin | Kotlin JVM plugin for the `core` module |
| kotlin-test | local | https://github.com/JetBrains/kotlin | JUnit 5 tests for the `core` module |
