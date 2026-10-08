//! JMAP `Email/set` and `EmailSubmission/set` from a scripted exchange.
//!
//! The response is already in hand. Submission records `\Answered` and the
//! created id. No SMTP socket is opened.

use serde_json::Value;

use mailune_store::{FlagRow, Store};

use crate::Error;

/// A flag patch the server accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagChange {
    /// Message the patch applied to.
    pub message_id: String,
    /// `\Seen`.
    pub seen: bool,
    /// `\Flagged`.
    pub flagged: bool,
    /// `\Draft`.
    pub draft: bool,
    /// `\Answered`.
    pub answered: bool,
    /// `\Deleted`.
    pub deleted: bool,
    /// `newState` from the response, when the server sent one.
    pub state: String,
}

/// An `EmailSubmission` the server created. The message was not handed to SMTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    /// Server id from `created`.
    pub id: String,
    /// `emailId` from the request.
    pub email_id: String,
    /// `newState` from the response, when the server sent one.
    pub state: String,
}

/// Applies `keywords/` patches for ids the response lists under `updated`.
///
/// # Errors
///
/// [`Error::Body`] when the exchange is not `Email/set` or the server rejected
/// an update. [`Error::Store`] when the flag row cannot be written.
pub fn apply_flags(
    store: &mut Store,
    request: &str,
    response: &str,
) -> Result<Vec<FlagChange>, Error> {
    let request = method_body(request, "methodCalls", "Email/set")?;
    let response = method_body(response, "methodResponses", "Email/set")?;
    if rejected(&response, "notUpdated") {
        return Err(Error::Body);
    }
    let update = response_map(&request, "update")?;
    let updated = response_map(&response, "updated")?;
    let state = state_of(&response);
    let mut out = Vec::new();
    for message_id in updated.keys() {
        let patch = update.get(message_id).ok_or(Error::Body)?;
        let bits = bits_from_patch(patch)?;
        store.upsert_flags(&FlagRow {
            message_id: message_id.clone(),
            seen: bits.seen,
            flagged: bits.flagged,
            draft: bits.draft,
            answered: bits.answered,
            deleted: bits.deleted,
            keywords: bits.keywords.join("\n"),
        })?;
        out.push(FlagChange {
            message_id: message_id.clone(),
            seen: bits.seen,
            flagged: bits.flagged,
            draft: bits.draft,
            answered: bits.answered,
            deleted: bits.deleted,
            state: state.clone(),
        });
    }
    Ok(out)
}

/// Records a created `EmailSubmission` and marks that message `\Answered`.
///
/// # Errors
///
/// [`Error::Body`] when the exchange is not `EmailSubmission/set` or `created`
/// is missing. [`Error::Store`] when the flag row cannot be written.
pub fn apply_submission(
    store: &mut Store,
    request: &str,
    response: &str,
) -> Result<Vec<Submission>, Error> {
    let request = method_body(request, "methodCalls", "EmailSubmission/set")?;
    let response = method_body(response, "methodResponses", "EmailSubmission/set")?;
    if rejected(&response, "notCreated") {
        return Err(Error::Body);
    }
    let create = response_map(&request, "create")?;
    let created = response_map(&response, "created")?;
    let state = state_of(&response);
    let mut out = Vec::new();
    for (client_id, made) in created {
        let id = made.get("id").and_then(Value::as_str).ok_or(Error::Body)?;
        let email_id = create
            .get(client_id)
            .and_then(|row| row.get("emailId"))
            .and_then(Value::as_str)
            .ok_or(Error::Body)?;
        store.upsert_flags(&FlagRow {
            message_id: email_id.to_string(),
            seen: false,
            flagged: false,
            draft: false,
            answered: true,
            deleted: false,
            keywords: String::new(),
        })?;
        out.push(Submission {
            id: id.to_string(),
            email_id: email_id.to_string(),
            state: state.clone(),
        });
    }
    Ok(out)
}

struct Bits {
    seen: bool,
    flagged: bool,
    draft: bool,
    answered: bool,
    deleted: bool,
    keywords: Vec<String>,
}

fn bits_from_patch(patch: &Value) -> Result<Bits, Error> {
    let obj = patch.as_object().ok_or(Error::Body)?;
    let mut bits = Bits {
        seen: false,
        flagged: false,
        draft: false,
        answered: false,
        deleted: false,
        keywords: Vec::new(),
    };
    if let Some(map) = obj.get("keywords").and_then(Value::as_object) {
        for (name, value) in map {
            bits.set(name, on(value)?);
        }
    }
    for (key, value) in obj {
        if let Some(name) = key.strip_prefix("keywords/") {
            bits.set(name, on(value)?);
        }
    }
    Ok(bits)
}

impl Bits {
    fn set(&mut self, name: &str, on: bool) {
        match name {
            "$seen" => self.seen = on,
            "$flagged" => self.flagged = on,
            "$draft" => self.draft = on,
            "$answered" => self.answered = on,
            "$deleted" => self.deleted = on,
            other if on => self.keywords.push(other.to_string()),
            _ => {}
        }
    }
}

fn on(value: &Value) -> Result<bool, Error> {
    match value {
        Value::Bool(on) => Ok(*on),
        Value::Null => Ok(false),
        _ => Err(Error::Body),
    }
}

pub(crate) fn method_body(json: &str, key: &str, name: &str) -> Result<Value, Error> {
    let doc: Value = serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))?;
    let calls = doc.get(key).and_then(Value::as_array).ok_or(Error::Body)?;
    for call in calls {
        let call = call.as_array().ok_or(Error::Body)?;
        let call_name = call.first().and_then(Value::as_str).ok_or(Error::Body)?;
        if call_name == name {
            return call.get(1).cloned().ok_or(Error::Body);
        }
    }
    Err(Error::Body)
}

fn response_map<'a>(
    body: &'a Value,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, Error> {
    body.get(key).and_then(Value::as_object).ok_or(Error::Body)
}

fn rejected(body: &Value, key: &str) -> bool {
    body.get(key)
        .and_then(Value::as_object)
        .is_some_and(|map| !map.is_empty())
}

fn state_of(body: &Value) -> String {
    body.get("newState")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{apply_flags, apply_submission};
    use mailune_store::{AccountRow, MessageRow, Store, ThreadRow};

    fn open() -> (std::path::PathBuf, Store) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("mailune-jmap-set-{nanos}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        store
            .upsert_account(&AccountRow {
                id: "acc-1".into(),
                email: "me@example.com".into(),
            })
            .unwrap();
        store
            .upsert_thread(&ThreadRow {
                id: "t1".into(),
                account_id: "acc-1".into(),
                subject: "Hello".into(),
            })
            .unwrap();
        store
            .upsert_message(&MessageRow {
                id: "m1".into(),
                account_id: "acc-1".into(),
                thread_id: "t1".into(),
                subject: "Hello".into(),
                from_email: "me@example.com".into(),
                to_emails: "ada@example.com".into(),
                stamp: "2026-03-01T12:00:00Z".into(),
                received_at: 1_772_366_400_000,
            })
            .unwrap();
        (dir, store)
    }

    #[test]
    fn a_flag_patch_and_a_submission_are_applied() {
        let (dir, mut store) = open();
        let flags = apply_flags(
            &mut store,
            r#"{"methodCalls":[["Email/set",{"accountId":"acc-1","update":{"m1":{"keywords/$seen":true,"keywords/$flagged":true}}},"0"]]}"#,
            r#"{"methodResponses":[["Email/set",{"accountId":"acc-1","newState":"s2","updated":{"m1":null}},"0"]]}"#,
        )
        .unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].message_id, "m1");
        assert!(flags[0].seen);
        assert!(flags[0].flagged);
        assert!(!flags[0].answered);
        assert_eq!(flags[0].state, "s2");

        let sent = apply_submission(
            &mut store,
            r#"{"methodCalls":[["EmailSubmission/set",{"accountId":"acc-1","create":{"k1":{"emailId":"m1"}}},"0"]]}"#,
            r#"{"methodResponses":[["EmailSubmission/set",{"accountId":"acc-1","newState":"s3","created":{"k1":{"id":"sub1"}}},"0"]]}"#,
        )
        .unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].id, "sub1");
        assert_eq!(sent[0].email_id, "m1");
        assert_eq!(sent[0].state, "s3");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_rejected_update_is_not_applied() {
        let (dir, mut store) = open();
        let err = apply_flags(
            &mut store,
            r#"{"methodCalls":[["Email/set",{"update":{"m1":{"keywords/$seen":true}}},"0"]]}"#,
            r#"{"methodResponses":[["Email/set",{"notUpdated":{"m1":{"type":"invalidPatch"}}},"0"]]}"#,
        )
        .unwrap_err();
        assert!(matches!(err, crate::Error::Body));
        let _ = std::fs::remove_dir_all(dir);
    }
}
