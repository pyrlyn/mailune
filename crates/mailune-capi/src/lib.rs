//! `mailune-ffi` behind a C ABI, for the shell no UniFFI generator reaches:
//! the GTK4 app in Vala, through `vapi/mailune.vapi`.
//!
//! The shape is the Swift binding's, carried in C terms: one opaque
//! `MailuneCore` handle and one function per call. Records cross as JSON, the
//! contract's own JSON, rather than C structs, so no struct layout has to stay
//! in step on two sides. Every call answers with one string, an envelope that
//! is either `{"ok": <value>}` or `{"error": {"type": …, "message": …}}`;
//! `schema/payloads.schema.json` describes every value. The string is
//! `malloc`ed: free it with `mailune_string_free`, or with `g_free`, which is
//! `free`.
//!
//! # Safety
//!
//! Every exported function that takes a pointer is `unsafe`, because it
//! trusts the caller with:
//!
//! - **Strings**: each `const char *` is a NUL-terminated string valid for
//!   the call. Bytes that are not UTF-8 are an error, not undefined behaviour.
//! - **Handles**: a `MailuneCore *` came from `mailune_core_new`, is not yet
//!   freed, and is not freed while a call uses it. Any number of threads may
//!   use one handle at once.
//!
//! A panic inside the core never crosses the boundary: it becomes a `core`
//! error. All the unsafe code lives in [`abi`]; the rest of the crate keeps
//! `unsafe_code` denied.

#![deny(clippy::undocumented_unsafe_blocks)]

pub mod abi;

use std::panic::{AssertUnwindSafe, catch_unwind};

use mailune_ffi::MailuneError;
use serde::Serialize;

/// What every call answers: its value, or why there is none.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Envelope<T> {
    Ok(T),
    Error(ErrorBody),
}

/// An error, with the sentence a person can be shown next to its kind.
#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
struct ErrorBody {
    #[serde(flatten)]
    error: MailuneError,
    message: String,
}

/// Runs one call and renders its answer as the JSON envelope.
///
/// The panic guard is here, not per function, so no function can forget it:
/// unwinding out of an `extern "C"` function aborts the host app.
fn respond<T: Serialize>(call: impl FnOnce() -> Result<T, MailuneError>) -> String {
    let result = catch_unwind(AssertUnwindSafe(call)).unwrap_or_else(|_| {
        Err(MailuneError::Core {
            message: "mailune-capi: the call panicked".into(),
        })
    });
    let envelope = match result {
        Ok(value) => Envelope::Ok(value),
        Err(error) => Envelope::Error(ErrorBody {
            message: error.to_string(),
            error,
        }),
    };
    // Every payload is plain data with string keys, which serde_json always
    // renders; the fallback only keeps a broken invariant from being silent.
    serde_json::to_string(&envelope).unwrap_or_else(|_| {
        r#"{"error":{"type":"core","message":"mailune-capi: the answer could not be encoded"}}"#
            .to_string()
    })
}

/// An argument the caller got wrong, as the error the envelope carries.
fn invalid(what: &str) -> MailuneError {
    MailuneError::Core {
        message: format!("mailune-capi: {what}"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::{MailuneError, respond};

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Compares `rendered` with the committed `relative`, or rewrites it
    /// under `MAILUNE_BLESS`.
    fn check_drift(relative: &str, rendered: &str) {
        let path = crate_dir().join(relative);
        if std::env::var_os("MAILUNE_BLESS").is_some() {
            std::fs::write(&path, rendered).unwrap();
            return;
        }
        // A Windows checkout may have turned LF into CRLF; the text is the same.
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            committed == rendered.replace("\r\n", "\n"),
            "{relative} is stale: run `MAILUNE_BLESS=1 cargo nextest run -p mailune-capi`"
        );
    }

    #[test]
    fn a_value_is_wrapped_in_ok() {
        let rendered = respond(|| Ok(vec!["t1"]));
        assert_eq!(
            serde_json::from_str::<Value>(&rendered).unwrap(),
            json!({"ok": ["t1"]})
        );
    }

    #[test]
    fn an_error_carries_its_type_its_fields_and_a_sentence() {
        let rendered = respond::<()>(|| {
            Err(MailuneError::MissingToken {
                account: "ada".into(),
            })
        });
        assert_eq!(
            serde_json::from_str::<Value>(&rendered).unwrap(),
            json!({"error": {"type": "missing_token", "account": "ada", "message": "no token is stored for ada"}})
        );
    }

    #[test]
    fn a_panic_becomes_a_core_error() {
        let rendered = respond::<()>(|| panic!("boom"));
        let value: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["error"]["type"], "core");
    }

    #[test]
    fn the_committed_header_is_what_cbindgen_generates() {
        let config = cbindgen::Config::from_file(crate_dir().join("cbindgen.toml")).unwrap();
        let mut rendered = Vec::new();
        cbindgen::Builder::new()
            .with_config(config)
            .with_src(crate_dir().join("src/abi.rs"))
            .generate()
            .unwrap()
            .write(&mut rendered);
        check_drift("include/mailune.h", &String::from_utf8(rendered).unwrap());
    }

    #[test]
    fn the_committed_payload_schema_is_what_the_records_generate() {
        check_drift("schema/payloads.schema.json", &payload_schema());
    }

    /// Every value an envelope can carry, under `$defs`, with the envelope
    /// itself as the root.
    fn payload_schema() -> String {
        let mut generator = schemars::generate::SchemaSettings::draft2020_12().into_generator();
        generator.subschema_for::<super::ErrorBody>();
        generator.subschema_for::<mailune_ffi::Event>();
        generator.subschema_for::<mailune_ffi::ViewState>();
        let defs = generator.take_definitions(true);
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$comment": "Generated from the Rust records by `MAILUNE_BLESS=1 cargo nextest run -p mailune-capi`. Do not edit.",
            "title": "mailune-capi payloads",
            "description": "The envelope every mailune-capi call returns. `ok` holds the value named in that function's comment in include/mailune.h; `mailune_core_fold` takes a JSON array of `Event`.",
            "oneOf": [
                {"type": "object", "required": ["ok"], "properties": {"ok": true}, "additionalProperties": false},
                {"type": "object", "required": ["error"], "properties": {"error": {"$ref": "#/$defs/ErrorBody"}}, "additionalProperties": false},
            ],
            "$defs": defs,
        });
        serde_json::to_string_pretty(&schema).unwrap() + "\n"
    }
}
