//! Gmail client over an injected transport: labels, thread listing,
//! `history.list` diffs, batch fetches, and one sync step from a saved
//! historyId.

use mailune_protocol::{Http, HttpRequest, MessageId, Method, Secret, ThreadId};
use serde::de::DeserializeOwned;

use crate::batch::{self, segment};
use crate::model::{HistoryPage, LabelList, ThreadList, WireMessage, WireThread};
use crate::{Error, GmailLabel, GmailMessage, HistoryDiff};

const API: &str = "https://gmail.googleapis.com";
const USER: &str = "/gmail/v1/users/me";
const BATCH: &str = "https://gmail.googleapis.com/batch/gmail/v1";
/// Google advises at most 50 calls per batch to avoid rate limiting.
const BATCH_SIZE: usize = 50;
/// `metadata` format with only the headers the envelope needs.
const METADATA: &str = "format=metadata&metadataHeaders=From&metadataHeaders=To&metadataHeaders=Cc&metadataHeaders=Subject&metadataHeaders=Date";

/// What one sync step found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBatch {
    /// Every label.
    pub labels: Vec<GmailLabel>,
    /// Messages added or changed since the saved historyId.
    pub messages: Vec<GmailMessage>,
    /// Messages deleted since the saved historyId.
    pub deleted: Vec<MessageId>,
    /// historyId to save for the next step.
    pub history_id: String,
}

/// A Gmail client. It holds the OAuth access token.
pub struct GmailClient<'h, H: Http> {
    http: &'h H,
    token: Secret,
}

impl<H: Http> std::fmt::Debug for GmailClient<'_, H> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GmailClient")
            .finish_non_exhaustive()
    }
}

impl<'h, H: Http> GmailClient<'h, H> {
    /// A client that will authenticate with `token` as a bearer token.
    pub fn new(http: &'h H, token: Secret) -> Self {
        Self { http, token }
    }

    /// Every label.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn labels(&self) -> Result<Vec<GmailLabel>, Error> {
        let list: LabelList = self.get(&format!("{USER}/labels")).await?;
        Ok(list.labels.into_iter().map(Into::into).collect())
    }

    /// Ids of the newest `limit` threads.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn thread_ids(&self, limit: u32) -> Result<Vec<ThreadId>, Error> {
        let list: ThreadList = self
            .get(&format!("{USER}/threads?maxResults={limit}"))
            .await?;
        Ok(list
            .threads
            .into_iter()
            .map(|thread| ThreadId::new(thread.id))
            .collect())
    }

    /// Everything `history.list` reports since `start`, across pages.
    /// `None` when Gmail no longer keeps history that old (404), which
    /// means a full sync.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] when `start` is not a number; transport, status,
    /// or format errors.
    pub async fn history(&self, start: &str) -> Result<Option<HistoryDiff>, Error> {
        if start.is_empty() || !start.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Error::Format("historyId is not a number".into()));
        }
        let mut diff = HistoryDiff::default();
        let mut token: Option<String> = None;
        loop {
            let mut path = format!("{USER}/history?startHistoryId={start}");
            if let Some(token) = &token {
                path.push_str("&pageToken=");
                path.push_str(&query_escape(token));
            }
            let page: HistoryPage = match self.get(&path).await {
                Err(Error::Status { status: 404 }) => return Ok(None),
                other => other?,
            };
            let next = page.next_page_token.clone();
            page.fold_into(&mut diff);
            match next {
                Some(next) if token.as_ref() != Some(&next) => token = Some(next),
                // A repeated token would loop forever.
                _ => return Ok(Some(diff)),
            }
        }
    }

    /// Messages by id, fetched in batches. A message deleted since it was
    /// listed (404 inside the batch) is left out.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn messages(&self, ids: &[MessageId]) -> Result<Vec<GmailMessage>, Error> {
        let mut paths = Vec::with_capacity(ids.len());
        for id in ids {
            paths.push(format!(
                "{USER}/messages/{}?{METADATA}",
                segment(id.as_str())?
            ));
        }
        let wire: Vec<WireMessage> = self.batch(&paths).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    /// Every message of the given threads, fetched in batches.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn threads(&self, ids: &[ThreadId]) -> Result<Vec<GmailMessage>, Error> {
        let mut paths = Vec::with_capacity(ids.len());
        for id in ids {
            paths.push(format!(
                "{USER}/threads/{}?{METADATA}",
                segment(id.as_str())?
            ));
        }
        let wire: Vec<WireThread> = self.batch(&paths).await?;
        Ok(wire
            .into_iter()
            .flat_map(|thread| thread.messages)
            .map(Into::into)
            .collect())
    }

    /// One sync step. With a saved historyId it fetches what changed; with
    /// none, or one Gmail has expired, it lists the newest `limit` threads.
    ///
    /// # Errors
    ///
    /// Transport, status, or format errors.
    pub async fn sync(&self, since: Option<&str>, limit: u32) -> Result<SyncBatch, Error> {
        let labels = self.labels().await?;
        if let Some(start) = since
            && let Some(diff) = self.history(start).await?
        {
            let messages = self.messages(&diff.changed).await?;
            return Ok(SyncBatch {
                labels,
                messages,
                deleted: diff.deleted,
                history_id: diff.history_id,
            });
        }
        let ids = self.thread_ids(limit).await?;
        let messages = self.threads(&ids).await?;
        // The newest message's historyId is where later changes start.
        let newest = messages.iter().map(|m| m.history_id).max().unwrap_or(0);
        Ok(SyncBatch {
            labels,
            messages,
            deleted: Vec::new(),
            history_id: newest.to_string(),
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let request = HttpRequest::new(Method::Get, format!("{API}{path}"));
        let response = self.send(request).await?;
        parse(&response.body)
    }

    async fn batch<T: DeserializeOwned>(&self, paths: &[String]) -> Result<Vec<T>, Error> {
        let mut out = Vec::with_capacity(paths.len());
        for chunk in paths.chunks(BATCH_SIZE) {
            let mut request = HttpRequest::new(Method::Post, BATCH).header(
                "Content-Type",
                format!("multipart/mixed; boundary={}", batch::BOUNDARY),
            );
            request.body = batch::encode(chunk);
            let response = self.send(request).await?;
            let content_type = response.header_value("content-type").unwrap_or_default();
            for part in batch::decode(content_type, &response.body)? {
                match part.status {
                    200..=299 => out.push(parse(&part.body)?),
                    404 => {}
                    status => return Err(Error::Status { status }),
                }
            }
        }
        Ok(out)
    }

    async fn send(&self, request: HttpRequest) -> Result<mailune_protocol::HttpResponse, Error> {
        // The token is UTF-8 by contract; a non-UTF-8 byte is replaced, which
        // the server then rejects, rather than being logged anywhere.
        let bearer = format!("Bearer {}", String::from_utf8_lossy(self.token.as_bytes()));
        let response = self
            .http
            .send(request.header("Authorization", bearer))
            .await?;
        if !(200..300).contains(&response.status) {
            return Err(Error::Status {
                status: response.status,
            });
        }
        Ok(response)
    }
}

fn parse<T: DeserializeOwned>(body: &[u8]) -> Result<T, Error> {
    serde_json::from_slice(body).map_err(|error| Error::Format(error.to_string()))
}

/// Percent-encodes everything outside RFC 3986's unreserved set.
fn query_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{
        AccountId, HttpResponse, MailboxId, MailboxRole, MessageId, Secret, ThreadId,
    };
    use mailune_store::{Account, Counts, Mailbox, Store, StoredMessage};
    use mailune_testkit::{ScriptedHttp, poll_now};

    use super::{GmailClient, SyncBatch, query_escape};
    use crate::Error;

    const LABELS: &str = include_str!("../fixtures/labels.json");
    const THREADS: &str = include_str!("../fixtures/threads-list.json");
    const BATCH_THREADS: &str = include_str!("../fixtures/batch-threads.txt");
    const HISTORY_1: &str = include_str!("../fixtures/history-1.json");
    const HISTORY_2: &str = include_str!("../fixtures/history-2.json");
    const BATCH_MESSAGES: &str = include_str!("../fixtures/batch-messages.txt");

    fn multipart(body: &str) -> HttpResponse {
        HttpResponse::new(200, body.replace('\n', "\r\n"))
            .header("Content-Type", "multipart/mixed; boundary=batch_abc")
    }

    fn apply(store: &mut Store, account: &AccountId, batch: &SyncBatch) {
        for label in &batch.labels {
            store
                .upsert_mailbox(&Mailbox {
                    account: account.clone(),
                    id: label.id.clone(),
                    name: label.name.clone(),
                    role: label.role,
                    parent: None,
                })
                .unwrap();
        }
        for message in &batch.messages {
            store
                .upsert_message(&StoredMessage {
                    account: account.clone(),
                    envelope: message.envelope.clone(),
                    received_at: message.received_at,
                    mailboxes: message.mailboxes.clone(),
                })
                .unwrap();
        }
        store
            .set_sync_state(account, "gmail:history", &batch.history_id)
            .unwrap();
    }

    #[test]
    fn a_scripted_sync_lands_in_the_store_and_resumes_from_its_history_id() {
        let http = ScriptedHttp::new()
            .json(LABELS)
            .json(THREADS)
            .reply(multipart(BATCH_THREADS))
            .json(LABELS)
            .json(HISTORY_1)
            .json(HISTORY_2)
            .reply(multipart(BATCH_MESSAGES));
        let client = GmailClient::new(&http, Secret::new("ya29.tok"));
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        let account = AccountId::new("me@gmail.com");
        store
            .upsert_account(&Account {
                id: account.clone(),
                email: "me@gmail.com".into(),
                display_name: None,
                provider: "gmail".into(),
            })
            .unwrap();

        let saved = store.sync_state(&account, "gmail:history").unwrap();
        let first = poll_now(client.sync(saved.as_deref(), 20))
            .unwrap()
            .unwrap();
        apply(&mut store, &account, &first);
        assert_eq!(first.history_id, "1203");
        let inbox = MailboxId::new("INBOX");
        assert!(
            first
                .labels
                .iter()
                .any(|label| label.id == inbox && label.role == Some(MailboxRole::Inbox))
        );
        assert_eq!(
            store.mailbox_counts(&account, &inbox).unwrap(),
            Counts {
                total: 3,
                unread: 2
            }
        );
        let thread = store
            .thread_messages(&account, &ThreadId::new("t1"))
            .unwrap();
        let subjects: Vec<&str> = thread.iter().map(|m| m.subject.as_str()).collect();
        assert_eq!(subjects, ["Dock", "Re: Dock"]);
        assert_eq!(thread[0].from.name.as_deref(), Some("Lovelace, Ada"));

        let saved = store.sync_state(&account, "gmail:history").unwrap();
        let second = poll_now(client.sync(saved.as_deref(), 20))
            .unwrap()
            .unwrap();
        apply(&mut store, &account, &second);
        assert_eq!(second.history_id, "1300");
        assert_eq!(second.deleted, [MessageId::new("m2")]);
        let changed: Vec<&str> = second
            .messages
            .iter()
            .map(|m| m.envelope.id.as_str())
            .collect();
        assert_eq!(changed, ["m3", "m4"]);
        assert_eq!(
            store.mailbox_counts(&account, &inbox).unwrap(),
            Counts {
                total: 4,
                unread: 2
            }
        );

        let requests = http.requests();
        assert_eq!(requests.len(), 7);
        assert_eq!(
            requests[1].url,
            "https://gmail.googleapis.com/gmail/v1/users/me/threads?maxResults=20"
        );
        assert_eq!(
            requests[4].url,
            "https://gmail.googleapis.com/gmail/v1/users/me/history?startHistoryId=1203"
        );
        assert!(requests[5].url.ends_with("&pageToken=next%2Fpage"));
        let batch = String::from_utf8(requests[6].body.clone()).unwrap();
        assert!(batch.contains("GET /gmail/v1/users/me/messages/m3?format=metadata"));
        assert!(batch.contains("GET /gmail/v1/users/me/messages/m4?format=metadata"));
        assert_eq!(
            requests[6].header_value("content-type"),
            Some("multipart/mixed; boundary=mailune_batch")
        );
        assert_eq!(
            requests[6].header_value("authorization"),
            Some("Bearer ya29.tok")
        );
        assert!(!format!("{client:?}").contains("ya29"));
    }

    #[test]
    fn an_expired_history_id_falls_back_to_a_full_listing() {
        let http = ScriptedHttp::new()
            .json(LABELS)
            .reply(HttpResponse::new(404, "{}"))
            .json(THREADS)
            .reply(multipart(BATCH_THREADS));
        let client = GmailClient::new(&http, Secret::new("t"));
        let batch = poll_now(client.sync(Some("5"), 20)).unwrap().unwrap();
        assert_eq!(batch.messages.len(), 3);
        assert_eq!(batch.history_id, "1203");
        assert_eq!(http.remaining(), 0);
    }

    #[test]
    fn a_failed_part_or_a_bad_id_is_an_error() {
        let failed = "--batch_abc\nContent-Type: application/http\n\nHTTP/1.1 500 Internal Server Error\n\n{}\n--batch_abc--\n";
        let http = ScriptedHttp::new()
            .reply(multipart(failed))
            .reply(HttpResponse::new(401, "{}"));
        let client = GmailClient::new(&http, Secret::new("t"));
        assert!(matches!(
            poll_now(client.messages(&[MessageId::new("m1")])).unwrap(),
            Err(Error::Status { status: 500 })
        ));
        assert!(matches!(
            poll_now(client.labels()).unwrap(),
            Err(Error::Status { status: 401 })
        ));
        assert!(matches!(
            poll_now(client.messages(&[MessageId::new("../labels")])).unwrap(),
            Err(Error::Format(_))
        ));
        assert!(matches!(
            poll_now(client.history("1&x=y")).unwrap(),
            Err(Error::Format(_))
        ));
        assert_eq!(http.remaining(), 0);
    }

    #[test]
    fn query_values_are_escaped() {
        assert_eq!(query_escape("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(query_escape("a/b&c=d"), "a%2Fb%26c%3Dd");
    }
}
