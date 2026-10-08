//! Graph move, flag, `sendMail`, and `$batch` from a scripted transport.
//!
//! `sendMail` answers 202 with an empty body, so the subject is taken from
//! the request. No socket is opened.

use serde_json::Value;

use mailune_store::{FlagRow, MailboxRow, MembershipRow, Store, SyncStateRow};

use crate::{Error, apply_envelope, envelopes};

/// A message moved to another folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    /// Message id.
    pub message_id: String,
    /// `parentFolderId` after the move.
    pub folder_id: String,
}

/// `isRead`, flag, or categories applied to one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagPatch {
    /// Message id.
    pub message_id: String,
    /// `isRead`. Omitted in the body is unread.
    pub seen: bool,
    /// `flag.flagStatus` of `flagged`.
    pub flagged: bool,
    /// Categories, one per line.
    pub keywords: String,
}

/// A `sendMail` the transport accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentMail {
    /// Subject from the request.
    pub subject: String,
    /// `toRecipients` addresses.
    pub to: Vec<String>,
    /// `saveToSentItems`. Defaults to true, matching Graph.
    pub saved: bool,
}

/// One part of a `$batch` response that was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchPart {
    /// Part id from the batch envelope.
    pub id: String,
    /// HTTP status.
    pub status: u16,
    /// Message id when the part body had one.
    pub message_id: Option<String>,
}

/// Moves a message by `parentFolderId`.
///
/// # Errors
///
/// [`Error::Body`] when the id or folder is missing. [`Error::Store`] when
/// the membership cannot be written.
pub fn apply_move(store: &mut Store, account_id: &str, response: &str) -> Result<Move, Error> {
    let body: Value = parse(response)?;
    let message_id = id_of(&body)?;
    let folder_id = body
        .get("parentFolderId")
        .and_then(Value::as_str)
        .ok_or(Error::Body)?
        .to_string();
    write_move(store, account_id, &message_id, &folder_id)?;
    Ok(Move {
        message_id,
        folder_id,
    })
}

/// Applies `isRead`, `flag`, and `categories`.
///
/// # Errors
///
/// [`Error::Body`] when the message id is missing. [`Error::Store`] when the
/// flag row cannot be written.
pub fn apply_flag(store: &mut Store, response: &str) -> Result<FlagPatch, Error> {
    let body: Value = parse(response)?;
    let patch = flag_patch(&body)?;
    write_flags(store, &patch)?;
    Ok(patch)
}

/// Records `sendMail`. An empty 202 still returns the request subject.
///
/// A response that is a message object is stored in the sent folder.
///
/// # Errors
///
/// [`Error::Body`] when the request has no message object. [`Error::Store`]
/// when the sent folder cannot be written.
pub fn apply_send(
    store: &mut Store,
    account_id: &str,
    request: &str,
    response: &str,
) -> Result<SentMail, Error> {
    let request: Value = parse(request)?;
    let message = request.get("message").ok_or(Error::Body)?;
    let subject = message
        .get("subject")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let to = recipients(message);
    let saved = request
        .get("saveToSentItems")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let response = response.trim();
    if !response.is_empty() && response != "{}" {
        for envelope in envelopes(response)? {
            apply_envelope(store, account_id, Some("sent"), &envelope)?;
        }
    }
    if saved {
        store.upsert_mailbox(&MailboxRow {
            id: "sent".to_string(),
            account_id: account_id.to_string(),
            name: "Sent".to_string(),
            role: Some("sent".to_string()),
        })?;
        store.upsert_sync_state(&SyncStateRow {
            account_id: account_id.to_string(),
            mailbox_id: "sent".to_string(),
            token: "sendMail".to_string(),
        })?;
    }
    Ok(SentMail { subject, to, saved })
}

/// Applies each 2xx part. A 4xx part fails the batch before any write.
///
/// # Errors
///
/// [`Error::Body`] when a part failed or has no status. [`Error::Store`] when
/// a part cannot be written.
pub fn apply_batch(
    store: &mut Store,
    account_id: &str,
    response: &str,
) -> Result<Vec<BatchPart>, Error> {
    let doc: Value = parse(response)?;
    let parts = doc
        .get("responses")
        .and_then(Value::as_array)
        .ok_or(Error::Body)?;
    let mut parsed = Vec::new();
    for part in parts {
        let id = part
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let status = part
            .get("status")
            .and_then(Value::as_u64)
            .ok_or(Error::Body)?;
        let status = u16::try_from(status).map_err(|_| Error::Body)?;
        if status >= 400 {
            return Err(Error::Body);
        }
        parsed.push((id, status, part.get("body").cloned().unwrap_or(Value::Null)));
    }
    let mut out = Vec::new();
    for (id, status, body) in parsed {
        let message_id = apply_part(store, account_id, &body)?;
        out.push(BatchPart {
            id,
            status,
            message_id,
        });
    }
    Ok(out)
}

fn apply_part(store: &mut Store, account_id: &str, body: &Value) -> Result<Option<String>, Error> {
    if !body.is_object() {
        return Ok(None);
    }
    let Some(message_id) = body.get("id").and_then(Value::as_str) else {
        return Ok(None);
    };
    let message_id = message_id.to_string();
    if let Some(folder_id) = body.get("parentFolderId").and_then(Value::as_str) {
        write_move(store, account_id, &message_id, folder_id)?;
    }
    if body.get("isRead").is_some()
        || body.get("categories").is_some()
        || body.get("flag").is_some()
    {
        write_flags(store, &flag_patch(body)?)?;
    }
    Ok(Some(message_id))
}

fn write_move(
    store: &mut Store,
    account_id: &str,
    message_id: &str,
    folder_id: &str,
) -> Result<(), Error> {
    store.upsert_mailbox(&MailboxRow {
        id: folder_id.to_string(),
        account_id: account_id.to_string(),
        name: folder_id.to_string(),
        role: (folder_id.eq_ignore_ascii_case("inbox")).then(|| "inbox".to_string()),
    })?;
    store.upsert_membership(&MembershipRow {
        message_id: message_id.to_string(),
        mailbox_id: folder_id.to_string(),
    })?;
    Ok(())
}

fn write_flags(store: &mut Store, patch: &FlagPatch) -> Result<(), Error> {
    store.upsert_flags(&FlagRow {
        message_id: patch.message_id.clone(),
        seen: patch.seen,
        flagged: patch.flagged,
        draft: false,
        answered: false,
        deleted: false,
        keywords: patch.keywords.clone(),
    })?;
    Ok(())
}

fn flag_patch(body: &Value) -> Result<FlagPatch, Error> {
    let keywords = body
        .get("categories")
        .and_then(Value::as_array)
        .map(|categories| {
            categories
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let flagged = body
        .get("flag")
        .and_then(|flag| flag.get("flagStatus"))
        .and_then(Value::as_str)
        .is_some_and(|status| status.eq_ignore_ascii_case("flagged"));
    Ok(FlagPatch {
        message_id: id_of(body)?,
        seen: body.get("isRead").and_then(Value::as_bool).unwrap_or(false),
        flagged,
        keywords,
    })
}

fn recipients(message: &Value) -> Vec<String> {
    message
        .get("toRecipients")
        .and_then(Value::as_array)
        .map(|people| {
            people
                .iter()
                .filter_map(|person| {
                    person
                        .get("emailAddress")
                        .and_then(|address| address.get("address"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn id_of(body: &Value) -> Result<String, Error> {
    body.get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(Error::Body)
}

fn parse(json: &str) -> Result<Value, Error> {
    serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{apply_batch, apply_flag, apply_move, apply_send};
    use mailune_store::{AccountRow, MessageRow, Store, ThreadRow};

    fn open() -> (std::path::PathBuf, Store) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("mailune-graph-set-{nanos}-{}", std::process::id()));
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
                id: "AAQk".into(),
                account_id: "acc-1".into(),
                subject: "Hello".into(),
            })
            .unwrap();
        store
            .upsert_message(&MessageRow {
                id: "AAMk".into(),
                account_id: "acc-1".into(),
                thread_id: "AAQk".into(),
                subject: "Hello".into(),
                from_email: "ada@example.com".into(),
                to_emails: "me@example.com".into(),
                stamp: "2026-03-01T12:00:00Z".into(),
                received_at: 1_772_366_400_000,
            })
            .unwrap();
        (dir, store)
    }

    #[test]
    fn move_flag_send_and_batch_use_the_script() {
        let (dir, mut store) = open();
        let moved = apply_move(
            &mut store,
            "acc-1",
            r#"{"id":"AAMk","parentFolderId":"archive"}"#,
        )
        .unwrap();
        assert_eq!(moved.message_id, "AAMk");
        assert_eq!(moved.folder_id, "archive");

        let flags = apply_flag(
            &mut store,
            r#"{"id":"AAMk","isRead":true,"categories":["work"],"flag":{"flagStatus":"flagged"}}"#,
        )
        .unwrap();
        assert!(flags.seen);
        assert!(flags.flagged);
        assert_eq!(flags.keywords, "work");

        let sent = apply_send(
            &mut store,
            "acc-1",
            r#"{"message":{"subject":"Hello","toRecipients":[{"emailAddress":{"address":"ada@example.com"}}]},"saveToSentItems":true}"#,
            "{}",
        )
        .unwrap();
        assert_eq!(sent.subject, "Hello");
        assert_eq!(sent.to, ["ada@example.com"]);
        assert!(sent.saved);

        let parts = apply_batch(
            &mut store,
            "acc-1",
            r#"{"responses":[{"id":"1","status":200,"body":{"id":"AAMk","isRead":true,"parentFolderId":"archive"}},{"id":"2","status":202,"body":{}}]}"#,
        )
        .unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].message_id.as_deref(), Some("AAMk"));
        assert_eq!(parts[0].status, 200);
        assert_eq!(parts[1].status, 202);
        assert!(parts[1].message_id.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
