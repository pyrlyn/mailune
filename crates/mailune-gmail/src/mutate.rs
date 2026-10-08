//! Mutations: label changes through `messages.batchModify`, sending through
//! `messages.send`, and saving through `drafts.create`. The caller builds
//! the RFC 5322 bytes; this module only carries them.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE;
use mailune_protocol::{Http, HttpRequest, MailboxId, MessageId, Method, ThreadId};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::client::{API, USER, parse};
use crate::{Error, GmailClient};

/// `batchModify` takes at most 1000 ids per call.
const MODIFY_LIMIT: usize = 1000;

/// A message Gmail accepted for delivery.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sent {
    /// The sent message's id.
    pub id: MessageId,
    /// The thread Gmail filed it in.
    pub thread_id: ThreadId,
}

/// A saved draft.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Draft {
    /// Draft id, used to update or send the draft later.
    pub id: String,
    /// The message the draft holds.
    pub message: Sent,
}

impl<H: Http> GmailClient<'_, H> {
    /// Adds and removes labels on many messages. Marking read is removing
    /// `UNREAD`; a move is adding the target label and removing the source.
    ///
    /// # Errors
    ///
    /// Transport or status errors.
    pub async fn modify_labels(
        &self,
        ids: &[MessageId],
        add: &[MailboxId],
        remove: &[MailboxId],
    ) -> Result<(), Error> {
        for chunk in ids.chunks(MODIFY_LIMIT) {
            let body = json!({
                "ids": chunk,
                "addLabelIds": add,
                "removeLabelIds": remove,
            });
            // Success is `204 No Content`; there is nothing to parse.
            self.post(&format!("{USER}/messages/batchModify"), &body)
                .await?;
        }
        Ok(())
    }

    /// Sends `raw`, a complete RFC 5322 message, optionally into `thread`.
    /// Call this only after the person confirmed the send in the app.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn send_raw(&self, raw: &[u8], thread: Option<&ThreadId>) -> Result<Sent, Error> {
        let body = self
            .post(&format!("{USER}/messages/send"), &message(raw, thread))
            .await?;
        parse(&body)
    }

    /// Saves `raw` as a new draft, optionally in `thread`.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn create_draft(
        &self,
        raw: &[u8],
        thread: Option<&ThreadId>,
    ) -> Result<Draft, Error> {
        let body = self
            .post(
                &format!("{USER}/drafts"),
                &json!({ "message": message(raw, thread) }),
            )
            .await?;
        parse(&body)
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Vec<u8>, Error> {
        let bytes = serde_json::to_vec(body).map_err(|error| Error::Format(error.to_string()))?;
        let request = HttpRequest::new(Method::Post, format!("{API}{path}")).json(bytes);
        Ok(self.send(request).await?.body)
    }
}

fn message(raw: &[u8], thread: Option<&ThreadId>) -> Value {
    let mut message = json!({ "raw": URL_SAFE.encode(raw) });
    if let (Some(thread), Some(object)) = (thread, message.as_object_mut()) {
        object.insert("threadId".into(), json!(thread));
    }
    message
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE;
    use mailune_protocol::{HttpResponse, MailboxId, MessageId, Secret, ThreadId};
    use mailune_testkit::{ScriptedHttp, poll_now};
    use serde_json::Value;

    use crate::{Error, GmailClient};

    const RAW: &[u8] =
        b"From: me@gmail.com\r\nTo: ada@example.com\r\nSubject: Dock?\r\n\r\nSee you >> there\r\n";

    fn body(http: &ScriptedHttp, index: usize) -> Value {
        serde_json::from_slice(&http.requests()[index].body).unwrap()
    }

    #[test]
    fn labels_change_in_one_batch_modify() {
        let http = ScriptedHttp::new()
            .reply(HttpResponse::new(204, ""))
            .reply(HttpResponse::new(400, r#"{"error":{"code":400}}"#));
        let client = GmailClient::new(&http, Secret::new("t"));
        let ids = [MessageId::new("m1"), MessageId::new("m2")];
        poll_now(client.modify_labels(
            &ids,
            &[MailboxId::new("Label_1")],
            &[MailboxId::new("INBOX")],
        ))
        .unwrap()
        .unwrap();
        let request = &http.requests()[0];
        assert_eq!(
            request.url,
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/batchModify"
        );
        let sent = body(&http, 0);
        assert_eq!(sent["ids"], serde_json::json!(["m1", "m2"]));
        assert_eq!(sent["addLabelIds"][0], "Label_1");
        assert_eq!(sent["removeLabelIds"][0], "INBOX");
        assert!(matches!(
            poll_now(client.modify_labels(&ids, &[], &[MailboxId::new("UNREAD")])).unwrap(),
            Err(Error::Status { status: 400 })
        ));
    }

    #[test]
    fn a_send_carries_base64url_bytes_into_the_thread() {
        let http = ScriptedHttp::new().json(r#"{"id":"m9","threadId":"t1","labelIds":["SENT"]}"#);
        let client = GmailClient::new(&http, Secret::new("t"));
        let sent = poll_now(client.send_raw(RAW, Some(&ThreadId::new("t1"))))
            .unwrap()
            .unwrap();
        assert_eq!(sent.id, MessageId::new("m9"));
        assert_eq!(sent.thread_id, ThreadId::new("t1"));
        let request = body(&http, 0);
        let raw = request["raw"].as_str().unwrap();
        assert!(!raw.contains('+') && !raw.contains('/'));
        assert_eq!(URL_SAFE.decode(raw).unwrap(), RAW);
        assert_eq!(request["threadId"], "t1");
        assert!(http.requests()[0].url.ends_with("/users/me/messages/send"));
    }

    #[test]
    fn a_draft_is_saved_without_a_thread() {
        let http = ScriptedHttp::new()
            .json(r#"{"id":"r-55","message":{"id":"m10","threadId":"t9","labelIds":["DRAFT"]}}"#);
        let client = GmailClient::new(&http, Secret::new("t"));
        let draft = poll_now(client.create_draft(RAW, None)).unwrap().unwrap();
        assert_eq!(draft.id, "r-55");
        assert_eq!(draft.message.id, MessageId::new("m10"));
        let request = body(&http, 0);
        assert!(request["message"].get("threadId").is_none());
        assert_eq!(
            URL_SAFE
                .decode(request["message"]["raw"].as_str().unwrap())
                .unwrap(),
            RAW
        );
        assert_eq!(
            http.requests()[0].header_value("content-type"),
            Some("application/json")
        );
    }
}
