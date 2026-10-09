//! JMAP client over an injected transport: the session, the read calls, and
//! one sync step from a saved `Email` state.

use std::collections::BTreeMap;

use mailune_protocol::{Http, HttpRequest, MailboxId, MessageId, Method, Secret};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mail::{EMAIL_PROPERTIES, GetResult, WireEmail, WireMailbox, WireThread};
use crate::wire::{USING, decode, encode, json_error, reference, take};
use crate::{Changes, Error, JmapEmail, JmapMailbox, JmapThread, QueryResult};

/// The parts of the JMAP session resource the client uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Where method calls are posted.
    pub api_url: String,
    /// The primary mail account.
    pub account_id: String,
    /// Session state, to notice a changed session.
    pub state: String,
}

/// What one sync step found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBatch {
    /// Every mailbox, as `Mailbox/get` returned it.
    pub mailboxes: Vec<JmapMailbox>,
    /// Emails created or updated since the saved state.
    pub emails: Vec<JmapEmail>,
    /// Emails removed since the saved state.
    pub destroyed: Vec<MessageId>,
    /// `Email` state to save for the next step.
    pub state: String,
    /// The server has more changes; run another step.
    pub more: bool,
}

/// A JMAP client. It holds the bearer token and the loaded session.
pub struct JmapClient<'h, H: Http> {
    http: &'h H,
    token: Secret,
    session: Option<Session>,
}

impl<H: Http> std::fmt::Debug for JmapClient<'_, H> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JmapClient")
            .field("session", &self.session)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireSession {
    api_url: String,
    state: String,
    #[serde(default)]
    primary_accounts: BTreeMap<String, String>,
}

impl<'h, H: Http> JmapClient<'h, H> {
    /// A client that will authenticate with `token` as a bearer token.
    pub fn new(http: &'h H, token: Secret) -> Self {
        Self {
            http,
            token,
            session: None,
        }
    }

    /// Loads the session resource from `url` (usually `/.well-known/jmap`).
    ///
    /// # Errors
    ///
    /// [`Error::NoMailAccount`] when the session has no primary mail account.
    pub async fn load_session(&mut self, url: &str) -> Result<Session, Error> {
        let body = self.send(HttpRequest::new(Method::Get, url)).await?;
        let wire: WireSession = serde_json::from_slice(&body).map_err(json_error)?;
        let account_id = wire
            .primary_accounts
            .get("urn:ietf:params:jmap:mail")
            .cloned()
            .ok_or(Error::NoMailAccount)?;
        let session = Session {
            api_url: wire.api_url,
            account_id,
            state: wire.state,
        };
        self.session = Some(session.clone());
        Ok(session)
    }

    /// Every mailbox and the `Mailbox` state.
    ///
    /// # Errors
    ///
    /// Transport, status, JSON, or method errors.
    pub async fn mailboxes(&self) -> Result<(Vec<JmapMailbox>, String), Error> {
        let account = self.account()?;
        let responses = self
            .call(&[("Mailbox/get", json!({ "accountId": account, "ids": null }))])
            .await?;
        let got: GetResult<WireMailbox> = take(&responses, "Mailbox/get")?;
        Ok((got.list.into_iter().map(Into::into).collect(), got.state))
    }

    /// Loads the session from `url` and lists every mailbox id: the first
    /// thing a client asks of a new account.
    ///
    /// # Errors
    ///
    /// The errors of [`Self::load_session`] and [`Self::mailboxes`].
    pub async fn mailbox_ids(&mut self, url: &str) -> Result<Vec<MailboxId>, Error> {
        self.load_session(url).await?;
        let (mailboxes, _state) = self.mailboxes().await?;
        Ok(mailboxes.into_iter().map(|mailbox| mailbox.id).collect())
    }

    /// Emails by id and the `Email` state.
    ///
    /// # Errors
    ///
    /// Transport, status, JSON, or method errors.
    pub async fn emails(&self, ids: &[MessageId]) -> Result<(Vec<JmapEmail>, String), Error> {
        let account = self.account()?;
        let responses = self
            .call(&[(
                "Email/get",
                json!({ "accountId": account, "ids": ids, "properties": EMAIL_PROPERTIES }),
            )])
            .await?;
        let got: GetResult<WireEmail> = take(&responses, "Email/get")?;
        Ok((got.list.into_iter().map(Into::into).collect(), got.state))
    }

    /// Threads by id.
    ///
    /// # Errors
    ///
    /// Transport, status, JSON, or method errors.
    pub async fn threads(&self, ids: &[String]) -> Result<Vec<JmapThread>, Error> {
        let account = self.account()?;
        let responses = self
            .call(&[("Thread/get", json!({ "accountId": account, "ids": ids }))])
            .await?;
        let got: GetResult<WireThread> = take(&responses, "Thread/get")?;
        Ok(got.list.into_iter().map(Into::into).collect())
    }

    /// Ids of emails in `mailbox`, newest first.
    ///
    /// # Errors
    ///
    /// Transport, status, JSON, or method errors.
    pub async fn query(&self, mailbox: &MailboxId, limit: u32) -> Result<QueryResult, Error> {
        let account = self.account()?;
        let responses = self
            .call(&[("Email/query", query_args(account, Some(mailbox), limit))])
            .await?;
        take(&responses, "Email/query")
    }

    /// Changes to emails since `state`.
    ///
    /// # Errors
    ///
    /// [`Error::Method`] with `cannotCalculateChanges` when the state is too old.
    pub async fn changes(&self, state: &str) -> Result<Changes, Error> {
        let account = self.account()?;
        let responses = self
            .call(&[(
                "Email/changes",
                json!({ "accountId": account, "sinceState": state }),
            )])
            .await?;
        take(&responses, "Email/changes")
    }

    /// One sync step in a single request. With no saved state it lists the
    /// newest `limit` emails; with one it fetches what changed. A state the
    /// server can no longer diff from falls back to the full listing.
    ///
    /// # Errors
    ///
    /// Transport, status, JSON, or method errors.
    pub async fn sync(&self, since: Option<&str>, limit: u32) -> Result<SyncBatch, Error> {
        if let Some(state) = since {
            match self.sync_changes(state).await {
                Err(Error::Method { kind }) if kind == "cannotCalculateChanges" => {}
                other => return other,
            }
        }
        let account = self.account()?;
        let responses = self
            .call(&[
                ("Mailbox/get", json!({ "accountId": account, "ids": null })),
                ("Email/query", query_args(account, None, limit)),
                (
                    "Email/get",
                    json!({
                        "accountId": account,
                        "#ids": reference(1, "Email/query", "/ids"),
                        "properties": EMAIL_PROPERTIES,
                    }),
                ),
            ])
            .await?;
        let boxes: GetResult<WireMailbox> = take(&responses, "Mailbox/get")?;
        let emails: GetResult<WireEmail> = take(&responses, "Email/get")?;
        Ok(SyncBatch {
            mailboxes: boxes.list.into_iter().map(Into::into).collect(),
            emails: emails.list.into_iter().map(Into::into).collect(),
            destroyed: Vec::new(),
            state: emails.state,
            more: false,
        })
    }

    async fn sync_changes(&self, state: &str) -> Result<SyncBatch, Error> {
        let account = self.account()?;
        let fetch = |path| {
            json!({
                "accountId": account,
                "#ids": reference(1, "Email/changes", path),
                "properties": EMAIL_PROPERTIES,
            })
        };
        let responses = self
            .call(&[
                ("Mailbox/get", json!({ "accountId": account, "ids": null })),
                (
                    "Email/changes",
                    json!({ "accountId": account, "sinceState": state }),
                ),
                ("Email/get", fetch("/created")),
                ("Email/get", fetch("/updated")),
            ])
            .await?;
        let boxes: GetResult<WireMailbox> = take(&responses, "Mailbox/get")?;
        let changes: Changes = take(&responses, "Email/changes")?;
        let mut emails = Vec::new();
        for (name, args) in &responses {
            if name == "Email/get" {
                let got: GetResult<WireEmail> = GetResult::deserialize(args).map_err(json_error)?;
                emails.extend(got.list.into_iter().map(JmapEmail::from));
            }
        }
        Ok(SyncBatch {
            mailboxes: boxes.list.into_iter().map(Into::into).collect(),
            emails,
            destroyed: changes.destroyed,
            state: changes.new_state,
            more: changes.has_more_changes,
        })
    }

    /// Posts method calls and returns each response's arguments in order.
    pub(crate) async fn call(
        &self,
        calls: &[(&str, Value)],
    ) -> Result<Vec<(String, Value)>, Error> {
        self.call_using(&USING, calls).await
    }

    /// Like [`Self::call`] with extra capabilities (submission, Sieve, …).
    pub(crate) async fn call_using(
        &self,
        using: &[&str],
        calls: &[(&str, Value)],
    ) -> Result<Vec<(String, Value)>, Error> {
        let session = self.session.as_ref().ok_or(Error::NoSession)?;
        let request =
            HttpRequest::new(Method::Post, session.api_url.clone()).json(encode(using, calls)?);
        let body = self.send(request).await?;
        decode(&body)
    }

    pub(crate) fn account(&self) -> Result<&str, Error> {
        self.session
            .as_ref()
            .map(|session| session.account_id.as_str())
            .ok_or(Error::NoSession)
    }

    async fn send(&self, request: HttpRequest) -> Result<Vec<u8>, Error> {
        // The token is UTF-8 by contract; a non-UTF-8 byte is replaced, which
        // the server then rejects, rather than being logged anywhere.
        let bearer = format!("Bearer {}", String::from_utf8_lossy(self.token.as_bytes()));
        let request = request
            .header("Authorization", bearer)
            .header("Accept", "application/json");
        let response = self.http.send(request).await?;
        if !(200..300).contains(&response.status) {
            return Err(Error::Status {
                status: response.status,
            });
        }
        Ok(response.body)
    }
}

fn query_args(account: &str, mailbox: Option<&MailboxId>, limit: u32) -> Value {
    let filter = mailbox.map(|id| json!({ "inMailbox": id }));
    json!({
        "accountId": account,
        "filter": filter,
        "sort": [{ "property": "receivedAt", "isAscending": false }],
        "limit": limit,
    })
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{AccountId, HttpResponse, MailboxId, MessageId, Secret, ThreadId};
    use mailune_store::{Account, Counts, Mailbox, Store, StoredMessage};
    use mailune_testkit::{ScriptedHttp, poll_now};

    use super::{JmapClient, SyncBatch};
    use crate::Error;

    const SESSION: &str = include_str!("../fixtures/session.json");
    const INITIAL: &str = include_str!("../fixtures/sync-initial.json");
    const CHANGES: &str = include_str!("../fixtures/sync-changes.json");
    const THREADS: &str = include_str!("../fixtures/thread-query.json");

    fn apply(store: &mut Store, account: &AccountId, batch: &SyncBatch) {
        for mailbox in &batch.mailboxes {
            store
                .upsert_mailbox(&Mailbox {
                    account: account.clone(),
                    id: mailbox.id.clone(),
                    name: mailbox.name.clone(),
                    role: mailbox.role,
                    parent: mailbox.parent.clone(),
                })
                .unwrap();
        }
        for email in &batch.emails {
            store
                .upsert_message(&StoredMessage {
                    account: account.clone(),
                    envelope: email.envelope.clone(),
                    received_at: email.received_at,
                    mailboxes: email.mailboxes.clone(),
                })
                .unwrap();
        }
        store
            .set_sync_state(account, "jmap:Email", &batch.state)
            .unwrap();
    }

    #[test]
    fn mailbox_ids_loads_the_session_then_lists_every_mailbox() {
        let url = "https://jmap.example.com/.well-known/jmap";
        let http = ScriptedHttp::new().json(SESSION).json(INITIAL);
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session(url)).unwrap().unwrap();
        let (listed, _) = poll_now(client.mailboxes()).unwrap().unwrap();
        let listed: Vec<MailboxId> = listed.into_iter().map(|mailbox| mailbox.id).collect();
        assert!(!listed.is_empty());

        let http = ScriptedHttp::new().json(SESSION).json(INITIAL);
        let mut client = JmapClient::new(&http, Secret::new("t"));
        assert_eq!(poll_now(client.mailbox_ids(url)).unwrap().unwrap(), listed);
        assert_eq!(client.account().unwrap(), "acc-1");

        let http = ScriptedHttp::new().json(r#"{"apiUrl":"x","state":"s","primaryAccounts":{}}"#);
        let mut client = JmapClient::new(&http, Secret::new("t"));
        assert!(matches!(
            poll_now(client.mailbox_ids(url)).unwrap(),
            Err(Error::NoMailAccount)
        ));
    }

    #[test]
    fn a_scripted_sync_lands_in_the_store_and_resumes_from_its_state() {
        let http = ScriptedHttp::new()
            .json(SESSION)
            .json(INITIAL)
            .json(CHANGES);
        let mut client = JmapClient::new(&http, Secret::new("tok-123"));
        let session = poll_now(client.load_session("https://jmap.example.com/.well-known/jmap"))
            .unwrap()
            .unwrap();
        assert_eq!(session.account_id, "acc-1");

        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        let account = AccountId::new("acc-1");
        store
            .upsert_account(&Account {
                id: account.clone(),
                email: "me@example.com".into(),
                display_name: None,
                provider: "jmap".into(),
            })
            .unwrap();

        let saved = store.sync_state(&account, "jmap:Email").unwrap();
        let first = poll_now(client.sync(saved.as_deref(), 50))
            .unwrap()
            .unwrap();
        apply(&mut store, &account, &first);
        let inbox = MailboxId::new("mb-inbox");
        assert_eq!(
            store.mailbox_counts(&account, &inbox).unwrap(),
            Counts {
                total: 2,
                unread: 1
            }
        );
        let thread = store
            .thread_messages(&account, &ThreadId::new("t1"))
            .unwrap();
        let subjects: Vec<&str> = thread.iter().map(|m| m.subject.as_str()).collect();
        assert_eq!(subjects, ["Dock", "Re: Dock"]);

        let saved = store.sync_state(&account, "jmap:Email").unwrap();
        assert_eq!(saved.as_deref(), Some("e-1"));
        let second = poll_now(client.sync(saved.as_deref(), 50))
            .unwrap()
            .unwrap();
        apply(&mut store, &account, &second);
        assert_eq!(second.state, "e-2");
        assert_eq!(
            store.mailbox_counts(&account, &inbox).unwrap(),
            Counts {
                total: 2,
                unread: 1
            }
        );
        let page = store.thread_page(&account, &inbox, None, 10).unwrap();
        let ids: Vec<&str> = page.threads.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["t2", "t1"]);

        let requests = http.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[1].url, "https://jmap.example.com/api/");
        assert_eq!(
            requests[2].header_value("authorization"),
            Some("Bearer tok-123")
        );
        let body: serde_json::Value = serde_json::from_slice(&requests[2].body).unwrap();
        assert_eq!(body["methodCalls"][1][1]["sinceState"], "e-1");
        assert!(!format!("{client:?}").contains("tok-123"));
    }

    #[test]
    fn an_expired_state_falls_back_to_a_full_listing() {
        let expired = r#"{"methodResponses":[["error",{"type":"cannotCalculateChanges"},"c1"]],"sessionState":"s"}"#;
        let http = ScriptedHttp::new()
            .json(SESSION)
            .json(expired)
            .json(INITIAL);
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let batch = poll_now(client.sync(Some("ancient"), 50)).unwrap().unwrap();
        assert_eq!(batch.state, "e-1");
        assert_eq!(batch.emails.len(), 2);
    }

    #[test]
    fn thread_get_and_errors_parse() {
        let http = ScriptedHttp::new()
            .json(SESSION)
            .json(THREADS)
            .reply(HttpResponse::new(401, "nope"));
        let mut client = JmapClient::new(&http, Secret::new("t"));
        assert!(matches!(
            poll_now(client.mailboxes()).unwrap(),
            Err(Error::NoSession)
        ));
        poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let threads = poll_now(client.threads(&["t1".into()])).unwrap().unwrap();
        assert_eq!(
            threads[0].emails,
            [MessageId::new("m1"), MessageId::new("m2")]
        );
        assert!(matches!(
            poll_now(client.changes("e-1")).unwrap(),
            Err(Error::Status { status: 401 })
        ));
    }

    #[test]
    fn query_parses_ids_and_state() {
        let body = r#"{"methodResponses":[["Email/query",{"accountId":"acc-1","queryState":"q9","ids":["a","b"],"position":0,"canCalculateChanges":false}, "c0"]]}"#;
        let http = ScriptedHttp::new().json(SESSION).json(body);
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let result = poll_now(client.query(&MailboxId::new("mb-inbox"), 10))
            .unwrap()
            .unwrap();
        assert_eq!(result.ids, [MessageId::new("a"), MessageId::new("b")]);
        assert_eq!(result.query_state, "q9");
        let sent: serde_json::Value = serde_json::from_slice(&http.requests()[1].body).unwrap();
        assert_eq!(sent["methodCalls"][0][1]["filter"]["inMailbox"], "mb-inbox");
    }
}
