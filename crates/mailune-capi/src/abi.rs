//! The C ABI itself: the exported functions and the handle, which is what
//! `include/mailune.h` is generated from.
//!
//! The only module that may hold unsafe code, and it holds as little as the
//! boundary needs: reading the caller's strings and handing out `malloc`ed
//! answers. The handle crosses as `Box` and `Option<&T>`, which the compiler
//! already knows are nullable pointers, so taking and freeing it is safe
//! Rust. The caller's side of the contract is the crate's `# Safety`.
//!
//! Each function's comment names what its envelope's `ok` holds, a type from
//! `schema/payloads.schema.json`. Each body is one forwarded call; the syn
//! test in `crates/mailune-cli/tests/conventions.rs` holds that.

// `#[unsafe(no_mangle)]` and the unsafe blocks below; nowhere else in the crate.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_char};
use std::ptr;
use std::sync::Arc;

use mailune_ffi::{Event, MailuneError, ViewState};

use crate::{invalid, respond};

/// The view models one shell window renders. Safe to use from several
/// threads at once.
pub struct MailuneCore(Arc<mailune_ffi::MailuneCore>);

fn this(core: Option<&MailuneCore>) -> Result<&mailune_ffi::MailuneCore, MailuneError> {
    core.map(|core| &*core.0)
        .ok_or_else(|| invalid("core is NULL"))
}

/// A JSON string argument, decoded.
///
/// # Safety
///
/// `p` is NULL or a NUL-terminated string valid for the call.
unsafe fn json_arg<T: serde::de::DeserializeOwned>(
    p: *const c_char,
    what: &str,
) -> Result<T, MailuneError> {
    if p.is_null() {
        return Err(invalid(&format!("{what} is NULL")));
    }
    // SAFETY: not NULL, and this function's contract makes it a NUL-terminated
    // string that outlives the borrow.
    let text = unsafe { CStr::from_ptr(p) }
        .to_str()
        .map_err(|_| invalid(&format!("{what} is not UTF-8")))?;
    serde_json::from_str(text).map_err(|err| invalid(&format!("{what}: {err}")))
}

/// `fold`, with its argument read from C.
///
/// # Safety
///
/// As [`json_arg`] for `events`.
unsafe fn fold(
    core: Option<&MailuneCore>,
    events: *const c_char,
) -> Result<ViewState, MailuneError> {
    // SAFETY: this function's contract is `json_arg`'s.
    let events: Vec<Event> = unsafe { json_arg(events, "events") }?;
    Ok(this(core)?.fold(events))
}

/// `json` copied into `malloc`ed memory, so either `mailune_string_free` or
/// GLib's `g_free` can free it. NULL only when memory ran out.
fn answer(json: String) -> *mut c_char {
    let bytes = json.as_bytes();
    // SAFETY: `malloc` has no preconditions; its NULL is checked below.
    let out = unsafe { libc::malloc(bytes.len() + 1) }.cast::<u8>();
    if out.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: `out` holds `len + 1` writable bytes and is fresh memory, so it
    // cannot overlap `bytes`.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        out.add(bytes.len()).write(0);
    }
    out.cast()
}

/// The version of the core this library was built from. `ok`: a string.
#[unsafe(no_mangle)]
pub extern "C" fn mailune_version() -> *mut c_char {
    answer(respond(|| Ok(mailune_ffi::core_version())))
}

/// Free an answer from any function in this library. NULL is ignored.
///
/// # Safety
///
/// `answer` is NULL or a string this library returned, not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mailune_string_free(answer: *mut c_char) {
    // SAFETY: by this function's contract the string came from `answer`, so
    // from `malloc`, or is NULL, which `free` ignores.
    unsafe { libc::free(answer.cast()) }
}

/// A new core: empty list, no open thread, empty draft, English on the free
/// plan. Free it with `mailune_core_free`.
#[unsafe(no_mangle)]
pub extern "C" fn mailune_core_new() -> Box<MailuneCore> {
    Box::new(MailuneCore(mailune_ffi::MailuneCore::new()))
}

/// Free a core from `mailune_core_new`. NULL is ignored.
///
/// # Safety
///
/// `core` came from `mailune_core_new`, is not freed yet, and no call is
/// using it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mailune_core_free(core: Option<Box<MailuneCore>>) {
    drop(core)
}

/// Apply `events`, a JSON array of `Event`, in order. `ok`: the `ViewState`
/// to render.
///
/// # Safety
///
/// `core` came from `mailune_core_new` and is not freed yet; `events` is a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mailune_core_fold(
    core: Option<&MailuneCore>,
    events: *const c_char,
) -> *mut c_char {
    // SAFETY: this function's contract is `fold`'s.
    answer(respond(|| unsafe { fold(core, events) }))
}

/// The state to render now. `ok`: a `ViewState`.
///
/// # Safety
///
/// `core` came from `mailune_core_new` and is not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mailune_core_state(core: Option<&MailuneCore>) -> *mut c_char {
    answer(respond(|| Ok(this(core)?.state())))
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString, c_char};

    use serde_json::{Value, json};

    use super::{
        mailune_core_fold, mailune_core_free, mailune_core_new, mailune_core_state,
        mailune_string_free, mailune_version,
    };

    /// Reads and frees an answer, as a C caller would.
    fn take(answer: *mut c_char) -> Value {
        assert!(!answer.is_null());
        // SAFETY: a non-NULL answer is a NUL-terminated string from `answer`.
        let text = unsafe { CStr::from_ptr(answer) }
            .to_str()
            .unwrap()
            .to_owned();
        // SAFETY: the string came from this library and is freed once.
        unsafe { mailune_string_free(answer) };
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn the_version_is_an_ok_string() {
        assert_eq!(
            take(mailune_version()),
            json!({"ok": env!("CARGO_PKG_VERSION")})
        );
    }

    #[test]
    fn contract_json_folds_into_the_state_to_render() {
        let core = mailune_core_new();
        let events = CString::new(
            json!([
                {"snapshot": {"threads": [{
                    "id": "t1", "account": "local",
                    "from": {"name": null, "email": "ada@example.com"},
                    "subject": "Hello", "snippet": "plain", "stamp": "t0",
                    "message_count": 1, "unread": true, "flagged": false,
                    "important": false, "pinned": false, "snoozed": false,
                    "draft": false, "has_attachment": false,
                    "category": "primary", "mailbox": "inbox", "labels": []
                }]}},
                {"notice": {"message": "open t1"}}
            ])
            .to_string(),
        )
        .unwrap();
        // SAFETY: `core` is live and `events` is a C string for the call.
        let folded = take(unsafe { mailune_core_fold(Some(&core), events.as_ptr()) });
        assert_eq!(folded["ok"]["open"]["subject"], "Hello");
        // SAFETY: `core` is live.
        let state = take(unsafe { mailune_core_state(Some(&core)) });
        assert_eq!(state["ok"]["list"][0]["id"], "t1");
        // SAFETY: `core` came from `mailune_core_new` and nothing uses it now.
        unsafe { mailune_core_free(Some(core)) };
    }

    #[test]
    fn bad_arguments_are_errors_not_crashes() {
        let core = mailune_core_new();
        let broken = CString::new("not json").unwrap();
        // SAFETY: `core` is live and `broken` is a C string for the call.
        let answer = take(unsafe { mailune_core_fold(Some(&core), broken.as_ptr()) });
        assert_eq!(answer["error"]["type"], "core");
        // SAFETY: NULL pointers are the documented error cases.
        let answer = take(unsafe { mailune_core_fold(Some(&core), std::ptr::null()) });
        assert_eq!(answer["error"]["message"], "mailune-capi: events is NULL");
        // SAFETY: a NULL core is the documented error case.
        let answer = take(unsafe { mailune_core_state(None) });
        assert_eq!(answer["error"]["message"], "mailune-capi: core is NULL");
        // SAFETY: NULL is ignored, and `core` is freed once.
        unsafe {
            mailune_core_free(None);
            mailune_core_free(Some(core));
            mailune_string_free(std::ptr::null_mut());
        }
    }
}
