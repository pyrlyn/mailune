//! Gmail `batchModify`, `messages.send`, and `drafts.create` from a script.
//!
//! The transport already returned these bodies. Nothing here opens a socket.

use serde::Deserialize;
use serde_json::Value;

use mailune_store::{FlagRow, MailboxRow, MembershipRow, Store};

use crate::{Error, apply_fetched, mailbox_label, role_of, system_label};

/// Labels a `batchModify` applied to one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelChange {
    /// Message id from the request.
    pub message_id: String,
    /// `UNREAD` was removed and not added back.
    pub seen: bool,
    /// `STARRED` was added.
    pub flagged: bool,
    /// `DRAFT` was added.
    pub draft: bool,
    /// User labels that were added, one per line.
    pub keywords: String,
}

/// A message `messages.send` returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentMessage {
    /// Sent message id.
    pub id: String,
    /// Thread id.
    pub thread_id: String,
}

/// A draft the scripted transport stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// Draft resource id. Same as the message id when the body is the message.
    pub draft_id: String,
    /// Message id inside the draft.
    pub message_id: String,
}

/// Applies `addLabelIds` / `removeLabelIds` after the transport accepts them.
///
/// # Errors
///
/// [`Error::Body`] when the response carries an `error` or `ids` is missing.
/// [`Error::Store`] when a flag row cannot be written.
pub fn apply_batch_modify(
    store: &mut Store,
    account_id: &str,
    request: &str,
    response: &str,
) -> Result<Vec<LabelChange>, Error> {
    ensure_ok(response)?;
    let modify: Modify =
        serde_json::from_str(request).map_err(|err| Error::Json(err.to_string()))?;
    let add = modify.add.unwrap_or_default();
    let remove = modify.remove.unwrap_or_default();
    let seen =
        remove.iter().any(|label| label == "UNREAD") && !add.iter().any(|label| label == "UNREAD");
    let flagged = add.iter().any(|label| label == "STARRED");
    let draft = add.iter().any(|label| label == "DRAFT");
    let keywords = add
        .iter()
        .filter(|label| !system_label(label))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = Vec::new();
    for message_id in &modify.ids {
        store.upsert_flags(&FlagRow {
            message_id: message_id.clone(),
            seen,
            flagged,
            draft,
            answered: false,
            deleted: false,
            keywords: keywords.clone(),
        })?;
        for label in add.iter().filter(|label| mailbox_label(label)) {
            store.upsert_mailbox(&MailboxRow {
                id: label.clone(),
                account_id: account_id.to_string(),
                name: label.clone(),
                role: role_of(label),
            })?;
            store.upsert_membership(&MembershipRow {
                message_id: message_id.clone(),
                mailbox_id: label.clone(),
            })?;
        }
        out.push(LabelChange {
            message_id: message_id.clone(),
            seen,
            flagged,
            draft,
            keywords: keywords.clone(),
        });
    }
    Ok(out)
}

/// Stores the message `messages.send` returned.
///
/// # Errors
///
/// [`Error::Body`] when the body has no message id. [`Error::Store`] when the
/// row cannot be written.
pub fn apply_send(
    store: &mut Store,
    account_id: &str,
    response: &str,
) -> Result<SentMessage, Error> {
    let id = apply_fetched(store, account_id, response)?
        .into_iter()
        .next()
        .ok_or(Error::Body)?;
    Ok(SentMessage {
        id,
        thread_id: thread_id(response)?,
    })
}

/// Stores the message inside a `drafts.create` or `drafts.update` response.
///
/// # Errors
///
/// [`Error::Body`] when the body has no message. [`Error::Store`] when the
/// row cannot be written.
pub fn apply_draft(store: &mut Store, account_id: &str, response: &str) -> Result<Draft, Error> {
    let value: Value =
        serde_json::from_str(response).map_err(|err| Error::Json(err.to_string()))?;
    let draft_id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let message = match value.get("message") {
        Some(message) => {
            serde_json::to_string(message).map_err(|err| Error::Json(err.to_string()))?
        }
        None => response.to_string(),
    };
    let message_id = apply_fetched(store, account_id, &message)?
        .into_iter()
        .next()
        .ok_or(Error::Body)?;
    let draft_id = if draft_id.is_empty() {
        message_id.clone()
    } else {
        draft_id
    };
    Ok(Draft {
        draft_id,
        message_id,
    })
}

fn ensure_ok(response: &str) -> Result<(), Error> {
    let value: Value =
        serde_json::from_str(response).map_err(|err| Error::Json(err.to_string()))?;
    if value.get("error").is_some() {
        return Err(Error::Body);
    }
    Ok(())
}

fn thread_id(response: &str) -> Result<String, Error> {
    let value: Value =
        serde_json::from_str(response).map_err(|err| Error::Json(err.to_string()))?;
    let value = value.get("message").unwrap_or(&value);
    value
        .get("threadId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(Error::Body)
}

#[derive(Deserialize)]
struct Modify {
    ids: Vec<String>,
    #[serde(rename = "addLabelIds")]
    add: Option<Vec<String>>,
    #[serde(rename = "removeLabelIds")]
    remove: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::{apply_batch_modify, apply_draft, apply_send};
    use mailune_store::{AccountRow, MessageRow, Store, ThreadRow};

    fn open() -> (std::path::PathBuf, Store) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("mailune-gmail-set-{nanos}-{}", std::process::id()));
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
                received_at: 1,
            })
            .unwrap();
        (dir, store)
    }

    #[test]
    fn batch_modify_send_and_draft_use_the_script() {
        let (dir, mut store) = open();
        let changed = apply_batch_modify(
            &mut store,
            "acc-1",
            r#"{"ids":["m1"],"addLabelIds":["STARRED","Label_1"],"removeLabelIds":["UNREAD"]}"#,
            "{}",
        )
        .unwrap();
        assert_eq!(changed.len(), 1);
        assert!(changed[0].seen);
        assert!(changed[0].flagged);
        assert!(!changed[0].draft);
        assert_eq!(changed[0].keywords, "Label_1");

        let sent = apply_send(
            &mut store,
            "acc-1",
            r#"{"id":"m1","threadId":"t1","labelIds":["SENT"],"snippet":"See you at the dock","internalDate":"1772366400000","payload":{"headers":[{"name":"From","value":"me@example.com"},{"name":"To","value":"ada@example.com"},{"name":"Subject","value":"Hello"}]}}"#,
        )
        .unwrap();
        assert_eq!(sent.id, "m1");
        assert_eq!(sent.thread_id, "t1");
        let rows = store.messages_in_thread("t1").unwrap();
        assert!(
            rows.iter()
                .any(|row| row.id == "m1" && row.subject == "Hello")
        );

        let draft = apply_draft(
            &mut store,
            "acc-1",
            r#"{"id":"d1","message":{"id":"m2","threadId":"t1","labelIds":["DRAFT"],"snippet":"Later","internalDate":"1772366400000","payload":{"headers":[{"name":"From","value":"me@example.com"},{"name":"To","value":"ada@example.com"},{"name":"Subject","value":"Draft"}]}}}"#,
        )
        .unwrap();
        assert_eq!(draft.draft_id, "d1");
        assert_eq!(draft.message_id, "m2");
        let rows = store.messages_in_thread("t1").unwrap();
        assert!(
            rows.iter()
                .any(|row| row.id == "m2" && row.subject == "Draft")
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_batch_modify_error_is_not_applied() {
        let (dir, mut store) = open();
        let err = apply_batch_modify(
            &mut store,
            "acc-1",
            r#"{"ids":["m1"],"addLabelIds":["STARRED"]}"#,
            r#"{"error":{"code":400}}"#,
        )
        .unwrap_err();
        assert!(matches!(err, crate::Error::Body));
        let _ = std::fs::remove_dir_all(dir);
    }
}
