//! Graph client over an injected transport: the folder tree and one
//! folder's message delta, resumed from a saved delta link.

use mailune_protocol::{Http, HttpRequest, HttpResponse, MailboxId, MessageId, Method, Secret};
use serde::de::DeserializeOwned;

use crate::model::{Page, SELECT, WireFolder, WireMessage};
use crate::{Error, GraphFolder, GraphMessage};

pub(crate) const ROOT: &str = "https://graph.microsoft.com/v1.0";
/// Every link the client follows must start here.
const HOST: &str = "https://graph.microsoft.com/";
/// A server that keeps sending links would otherwise never let go.
const MAX_PAGES: usize = 1000;
/// Bounds the folder walk the same way.
const MAX_FOLDERS: usize = 10_000;

/// What one delta round over a folder found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderDelta {
    /// Messages added or changed.
    pub messages: Vec<GraphMessage>,
    /// Messages that left the folder (deleted or moved out).
    pub removed: Vec<MessageId>,
    /// Link to save and pass to the next round.
    pub delta_link: String,
}

/// A Microsoft Graph client. It holds the OAuth access token.
pub struct GraphClient<'h, H: Http> {
    http: &'h H,
    token: Secret,
}

impl<H: Http> std::fmt::Debug for GraphClient<'_, H> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GraphClient")
            .finish_non_exhaustive()
    }
}

impl<'h, H: Http> GraphClient<'h, H> {
    /// A client that will authenticate with `token` as a bearer token.
    pub fn new(http: &'h H, token: Secret) -> Self {
        Self { http, token }
    }

    /// Every mail folder, parents before their children.
    ///
    /// # Errors
    ///
    /// Transport, status, format, or foreign-link errors.
    pub async fn folders(&self) -> Result<Vec<GraphFolder>, Error> {
        let mut out = Vec::new();
        let mut queue = vec![format!("{ROOT}/me/mailFolders?$top=100")];
        let mut next = 0;
        while let Some(url) = queue.get(next).cloned() {
            next += 1;
            for wire in self.all::<WireFolder>(url).await? {
                let children = wire.child_folder_count;
                let folder = GraphFolder::from(wire);
                if children > 0 {
                    queue.push(format!(
                        "{ROOT}/me/mailFolders/{}/childFolders?$top=100",
                        escape(folder.id.as_str())
                    ));
                }
                out.push(folder);
                if out.len() > MAX_FOLDERS {
                    return Err(Error::Format("too many folders".into()));
                }
            }
        }
        Ok(out)
    }

    /// One delta round over `folder`. With a saved link only the changes
    /// since then come back; with none, or one Graph has expired (410),
    /// the folder's messages are listed from the start.
    ///
    /// # Errors
    ///
    /// [`Error::ForeignLink`] when the saved or a returned link is not on
    /// the Graph host; transport, status, or format errors.
    pub async fn delta(
        &self,
        folder: &MailboxId,
        saved: Option<&str>,
    ) -> Result<FolderDelta, Error> {
        let start = format!(
            "{ROOT}/me/mailFolders/{}/messages/delta?$select={SELECT}",
            escape(folder.as_str())
        );
        if let Some(link) = saved {
            match self.delta_from(checked(link)?.to_string()).await {
                Err(Error::Status { status: 410 }) => {}
                other => return other,
            }
        }
        self.delta_from(start).await
    }

    async fn delta_from(&self, mut url: String) -> Result<FolderDelta, Error> {
        let mut messages = Vec::new();
        let mut removed = Vec::new();
        for _ in 0..MAX_PAGES {
            // Small pages keep each response bounded; Graph may go lower.
            let request = HttpRequest::new(Method::Get, url.as_str())
                .header("Prefer", "odata.maxpagesize=50");
            let page: Page<WireMessage> = parse(&self.send(request).await?)?;
            for wire in page.value {
                if wire.removed.is_some() {
                    removed.push(MessageId::new(wire.id));
                } else {
                    messages.push(GraphMessage::from(wire));
                }
            }
            match (page.next_link, page.delta_link) {
                (Some(next), _) => url = checked(&next)?.to_string(),
                (None, Some(delta)) => {
                    return Ok(FolderDelta {
                        messages,
                        removed,
                        delta_link: checked(&delta)?.to_string(),
                    });
                }
                (None, None) => {
                    return Err(Error::Format("delta page has no next or delta link".into()));
                }
            }
        }
        Err(Error::Format("delta did not finish".into()))
    }

    /// Every item of a collection, following `@odata.nextLink`.
    async fn all<T: DeserializeOwned>(&self, mut url: String) -> Result<Vec<T>, Error> {
        let mut out = Vec::new();
        for _ in 0..MAX_PAGES {
            let page: Page<T> = parse(
                &self
                    .send(HttpRequest::new(Method::Get, url.as_str()))
                    .await?,
            )?;
            out.extend(page.value);
            match page.next_link {
                Some(next) => url = checked(&next)?.to_string(),
                None => return Ok(out),
            }
        }
        Err(Error::Format("collection did not finish".into()))
    }

    pub(crate) async fn send(&self, request: HttpRequest) -> Result<HttpResponse, Error> {
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

pub(crate) fn parse<T: DeserializeOwned>(response: &HttpResponse) -> Result<T, Error> {
    serde_json::from_slice(&response.body).map_err(|error| Error::Format(error.to_string()))
}

/// A server-supplied link, accepted only on the Graph host so the bearer
/// token is never sent anywhere else.
fn checked(link: &str) -> Result<&str, Error> {
    if link.starts_with(HOST) {
        Ok(link)
    } else {
        Err(Error::ForeignLink)
    }
}

/// Percent-encodes a path segment: everything outside RFC 3986's
/// unreserved set. Graph ids carry `=`, `+` and `/`.
pub(crate) fn escape(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
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
    use mailune_protocol::{AccountId, HttpResponse, MailboxId, MailboxRole, MessageId, Secret};
    use mailune_store::{Account, Counts, Mailbox, Store, StoredMessage};
    use mailune_testkit::{ScriptedHttp, poll_now};

    use super::{GraphClient, escape};
    use crate::Error;

    const FOLDERS_1: &str = include_str!("../fixtures/folders-1.json");
    const FOLDERS_2: &str = include_str!("../fixtures/folders-2.json");
    const CHILDREN: &str = include_str!("../fixtures/child-folders.json");
    const DELTA_1: &str = include_str!("../fixtures/delta-1.json");
    const DELTA_2: &str = include_str!("../fixtures/delta-2.json");
    const DELTA_3: &str = include_str!("../fixtures/delta-3.json");

    const INBOX: &str = "AAMkInbox=";

    fn store() -> (tempfile::TempDir, Store, AccountId) {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        let account = AccountId::new("me@contoso.com");
        store
            .upsert_account(&Account {
                id: account.clone(),
                email: "me@contoso.com".into(),
                display_name: None,
                provider: "graph".into(),
            })
            .unwrap();
        (dir, store, account)
    }

    #[test]
    fn folders_and_a_delta_land_in_the_store_and_resume_from_the_link() {
        let http = ScriptedHttp::new()
            .json(FOLDERS_1)
            .json(FOLDERS_2)
            .json(CHILDREN)
            .json(DELTA_1)
            .json(DELTA_2)
            .json(DELTA_3);
        let client = GraphClient::new(&http, Secret::new("eyJ.tok"));
        let (_dir, mut store, account) = store();

        let folders = poll_now(client.folders()).unwrap().unwrap();
        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["Inbox", "Archive", "Sent Items", "Projects"]);
        assert_eq!(folders[0].role, Some(MailboxRole::Inbox));
        assert_eq!(folders[3].parent, Some(MailboxId::new("AAMkArchive")));
        for folder in &folders {
            store
                .upsert_mailbox(&Mailbox {
                    account: account.clone(),
                    id: folder.id.clone(),
                    name: folder.name.clone(),
                    role: folder.role,
                    parent: folder.parent.clone(),
                })
                .unwrap();
        }

        let inbox = MailboxId::new(INBOX);
        let key = format!("graph:delta:{INBOX}");
        for round in 0..2 {
            let saved = store.sync_state(&account, &key).unwrap();
            assert_eq!(saved.is_some(), round == 1);
            let delta = poll_now(client.delta(&inbox, saved.as_deref()))
                .unwrap()
                .unwrap();
            for message in &delta.messages {
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
                .set_sync_state(&account, &key, &delta.delta_link)
                .unwrap();
            if round == 0 {
                assert_eq!(delta.messages.len(), 3);
                assert_eq!(
                    store.mailbox_counts(&account, &inbox).unwrap(),
                    Counts {
                        total: 3,
                        unread: 2
                    }
                );
            } else {
                assert_eq!(delta.removed, [MessageId::new("m3")]);
                assert!(delta.delta_link.ends_with("$deltatoken=t2"));
            }
        }
        let thread = store
            .thread_messages(&account, &mailune_protocol::ThreadId::new("conv-1"))
            .unwrap();
        assert!(thread.iter().all(|m| m.flags.seen));
        assert_eq!(thread[1].flags.keywords, ["Work"]);

        let requests = http.requests();
        assert_eq!(
            requests[2].url,
            "https://graph.microsoft.com/v1.0/me/mailFolders/AAMkArchive/childFolders?$top=100"
        );
        assert!(
            requests[3]
                .url
                .starts_with("https://graph.microsoft.com/v1.0/me/mailFolders/AAMkInbox%3D/messages/delta?$select=id,conversationId,")
        );
        assert_eq!(
            requests[3].header_value("prefer"),
            Some("odata.maxpagesize=50")
        );
        assert!(requests[5].url.ends_with("$deltatoken=t1"));
        assert_eq!(
            requests[5].header_value("authorization"),
            Some("Bearer eyJ.tok")
        );
        assert!(!format!("{client:?}").contains("eyJ"));
    }

    #[test]
    fn an_expired_link_restarts_and_a_foreign_link_is_refused() {
        let http = ScriptedHttp::new()
            .reply(HttpResponse::new(
                410,
                r#"{"error":{"code":"syncStateNotFound"}}"#,
            ))
            .json(DELTA_2)
            .json(r#"{"value":[],"@odata.nextLink":"https://evil.example/steal"}"#);
        let client = GraphClient::new(&http, Secret::new("t"));
        let inbox = MailboxId::new(INBOX);
        let saved =
            "https://graph.microsoft.com/v1.0/me/mailFolders/x/messages/delta?$deltatoken=old";
        let delta = poll_now(client.delta(&inbox, Some(saved)))
            .unwrap()
            .unwrap();
        assert_eq!(delta.messages.len(), 1);
        assert!(http.requests()[1].url.contains("delta?$select="));
        assert!(matches!(
            poll_now(client.delta(&inbox, None)).unwrap(),
            Err(Error::ForeignLink)
        ));
        assert!(matches!(
            poll_now(client.delta(&inbox, Some("https://graph.microsoft.com.evil.example/x")))
                .unwrap(),
            Err(Error::ForeignLink)
        ));
        assert_eq!(http.remaining(), 0);
    }

    #[test]
    fn segments_are_escaped() {
        assert_eq!(escape("AAMk/a+b="), "AAMk%2Fa%2Bb%3D");
    }
}
