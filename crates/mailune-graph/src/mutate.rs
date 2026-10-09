//! Mutations: read, flag and category changes through PATCH, moves,
//! `sendMail` with caller-built MIME, and many PATCHes in one `$batch`.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mailune_protocol::{Http, HttpRequest, MailboxId, MessageId, Method};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::client::{ROOT, escape, parse};
use crate::{Error, GraphClient};

/// Graph accepts at most 20 requests per `$batch`.
const BATCH_LIMIT: usize = 20;

/// Changes to one message. `None` leaves a property alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessagePatch {
    /// Read state.
    pub is_read: Option<bool>,
    /// Follow-up flag on (`flagged`) or off (`notFlagged`).
    pub flagged: Option<bool>,
    /// The full category list; Graph replaces it, it does not merge.
    pub categories: Option<Vec<String>>,
}

/// How one request inside a `$batch` ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchOutcome {
    /// The message the request was for.
    pub id: MessageId,
    /// Its HTTP status.
    pub status: u16,
}

#[derive(Deserialize)]
struct Moved {
    id: MessageId,
}

#[derive(Deserialize)]
struct BatchResponse {
    #[serde(default)]
    responses: Vec<BatchItem>,
}

#[derive(Deserialize)]
struct BatchItem {
    id: String,
    status: u16,
}

impl MessagePatch {
    fn body(&self) -> Value {
        let mut body = Map::new();
        if let Some(read) = self.is_read {
            body.insert("isRead".into(), Value::Bool(read));
        }
        if let Some(flagged) = self.flagged {
            let status = if flagged { "flagged" } else { "notFlagged" };
            body.insert("flag".into(), json!({ "flagStatus": status }));
        }
        if let Some(categories) = &self.categories {
            body.insert("categories".into(), json!(categories));
        }
        Value::Object(body)
    }
}

impl<H: Http> GraphClient<'_, H> {
    /// Applies `patch` to one message.
    ///
    /// # Errors
    ///
    /// Transport or status errors.
    pub async fn update_message(&self, id: &MessageId, patch: &MessagePatch) -> Result<(), Error> {
        let url = format!("{ROOT}/me/messages/{}", escape(id.as_str()));
        self.send(json_request(Method::Patch, url, &patch.body())?)
            .await?;
        Ok(())
    }

    /// Moves one message to `destination` and returns its new id: Graph
    /// gives a moved message a new id.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn move_message(
        &self,
        id: &MessageId,
        destination: &MailboxId,
    ) -> Result<MessageId, Error> {
        let url = format!("{ROOT}/me/messages/{}/move", escape(id.as_str()));
        let body = json!({ "destinationId": destination });
        let moved: Moved = parse(&self.send(json_request(Method::Post, url, &body)?).await?)?;
        Ok(moved.id)
    }

    /// Sends `mime`, a complete RFC 5322 message, and saves it to Sent
    /// Items. Call this only after the person confirmed the send in the app.
    ///
    /// # Errors
    ///
    /// Transport or status errors.
    pub async fn send_mime(&self, mime: &[u8]) -> Result<(), Error> {
        // Graph's MIME form of sendMail takes the message base64-encoded as
        // a text/plain body.
        let mut request = HttpRequest::new(Method::Post, format!("{ROOT}/me/sendMail"))
            .header("Content-Type", "text/plain");
        request.body = STANDARD.encode(mime).into_bytes();
        self.send(request).await?;
        Ok(())
    }

    /// Applies many patches through `$batch`, 20 per request. One message
    /// failing does not stop the others; each gets its own status.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors for a batch as a whole.
    pub async fn update_messages(
        &self,
        patches: &[(MessageId, MessagePatch)],
    ) -> Result<Vec<BatchOutcome>, Error> {
        let mut out = Vec::with_capacity(patches.len());
        for chunk in patches.chunks(BATCH_LIMIT) {
            let requests: Vec<Value> = chunk
                .iter()
                .enumerate()
                .map(|(index, (id, patch))| {
                    json!({
                        "id": index.to_string(),
                        "method": "PATCH",
                        "url": format!("/me/messages/{}", escape(id.as_str())),
                        "headers": { "Content-Type": "application/json" },
                        "body": patch.body(),
                    })
                })
                .collect();
            let url = format!("{ROOT}/$batch");
            let body = json!({ "requests": requests });
            let reply: BatchResponse =
                parse(&self.send(json_request(Method::Post, url, &body)?).await?)?;
            // Responses may come back in any order; ids are our indexes.
            let mut statuses = vec![None; chunk.len()];
            for item in reply.responses {
                if let Some(slot) = item
                    .id
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| statuses.get_mut(index))
                {
                    *slot = Some(item.status);
                }
            }
            for ((id, _), status) in chunk.iter().zip(statuses) {
                let status =
                    status.ok_or_else(|| Error::Format("$batch left a request out".into()))?;
                out.push(BatchOutcome {
                    id: id.clone(),
                    status,
                });
            }
        }
        Ok(out)
    }
}

fn json_request(method: Method, url: String, body: &Value) -> Result<HttpRequest, Error> {
    let bytes = serde_json::to_vec(body).map_err(|error| Error::Format(error.to_string()))?;
    Ok(HttpRequest::new(method, url).json(bytes))
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use mailune_protocol::{HttpResponse, MailboxId, MessageId, Method, Secret};
    use mailune_testkit::{ScriptedHttp, poll_now};
    use serde_json::Value;

    use super::{BatchOutcome, MessagePatch};
    use crate::{Error, GraphClient};

    fn body(http: &ScriptedHttp, index: usize) -> Value {
        serde_json::from_slice(&http.requests()[index].body).unwrap()
    }

    #[test]
    fn a_patch_and_a_move_carry_only_what_changed() {
        let http = ScriptedHttp::new()
            .json(r#"{"id":"m1","isRead":true}"#)
            .json(r#"{"id":"m1-moved","parentFolderId":"AAMkArchive"}"#)
            .reply(HttpResponse::new(
                404,
                r#"{"error":{"code":"ErrorItemNotFound"}}"#,
            ));
        let client = GraphClient::new(&http, Secret::new("t"));
        let id = MessageId::new("m1=");
        let patch = MessagePatch {
            is_read: Some(true),
            flagged: Some(false),
            categories: None,
        };
        poll_now(client.update_message(&id, &patch))
            .unwrap()
            .unwrap();
        let first = &http.requests()[0];
        assert_eq!(first.method, Method::Patch);
        assert_eq!(
            first.url,
            "https://graph.microsoft.com/v1.0/me/messages/m1%3D"
        );
        let sent = body(&http, 0);
        assert_eq!(sent["isRead"], true);
        assert_eq!(sent["flag"]["flagStatus"], "notFlagged");
        assert!(sent.get("categories").is_none());

        let archive = MailboxId::new("AAMkArchive");
        let moved = poll_now(client.move_message(&id, &archive))
            .unwrap()
            .unwrap();
        assert_eq!(moved, MessageId::new("m1-moved"));
        assert!(http.requests()[1].url.ends_with("/me/messages/m1%3D/move"));
        assert_eq!(body(&http, 1)["destinationId"], "AAMkArchive");
        assert!(matches!(
            poll_now(client.move_message(&id, &archive)).unwrap(),
            Err(Error::Status { status: 404 })
        ));
    }

    #[test]
    fn send_mail_posts_base64_mime() {
        let mime: &[u8] =
            b"From: me@contoso.com\r\nTo: ada@example.com\r\nSubject: Dock?\r\n\r\nHi\r\n";
        let http = ScriptedHttp::new().reply(HttpResponse::new(202, ""));
        let client = GraphClient::new(&http, Secret::new("t"));
        poll_now(client.send_mime(mime)).unwrap().unwrap();
        let request = &http.requests()[0];
        assert_eq!(request.url, "https://graph.microsoft.com/v1.0/me/sendMail");
        assert_eq!(request.header_value("content-type"), Some("text/plain"));
        assert_eq!(STANDARD.decode(&request.body).unwrap(), mime);
    }

    #[test]
    fn a_batch_reports_each_status_in_request_order() {
        let http = ScriptedHttp::new().json(
            r#"{"responses":[
                {"id":"1","status":404,"body":{"error":{"code":"ErrorItemNotFound"}}},
                {"id":"0","status":200,"body":{"id":"a"}}]}"#,
        );
        let client = GraphClient::new(&http, Secret::new("t"));
        let tag = MessagePatch {
            categories: Some(vec!["Work".into()]),
            ..MessagePatch::default()
        };
        let outcomes = poll_now(client.update_messages(&[
            (MessageId::new("a"), tag.clone()),
            (MessageId::new("b"), tag),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(
            outcomes,
            [
                BatchOutcome {
                    id: MessageId::new("a"),
                    status: 200
                },
                BatchOutcome {
                    id: MessageId::new("b"),
                    status: 404
                },
            ]
        );
        assert!(http.requests()[0].url.ends_with("/v1.0/$batch"));
        let sent = body(&http, 0);
        assert_eq!(sent["requests"][1]["url"], "/me/messages/b");
        assert_eq!(sent["requests"][1]["method"], "PATCH");
        assert_eq!(sent["requests"][0]["body"]["categories"][0], "Work");
    }
}
