//! EWS client over an injected transport: one `SyncFolderItems` round,
//! resumed from a saved sync state.

use ews::soap::{Envelope, Header};
use ews::sync_folder_items::{Change, SyncFolderItems, SyncFolderItemsResponse};
use ews::{
    BaseFolderId, BaseShape, FlagStatus, ItemShape, Message, OperationResponse, RealItem,
    ResponseClass, server_version::ExchangeServerVersion,
};
use mailune_core::format_rfc3339_utc;
use mailune_protocol::{
    Address, Envelope as MailEnvelope, Flags, Http, HttpRequest, MailboxId, MessageId, Method,
    Secret, ThreadId, TransportSecurity,
};

use crate::Error;

/// Exchange Online hosts. EWS there is being switched off; Graph is the way in.
const EXCHANGE_ONLINE: [&str; 4] = [
    "outlook.office365.com",
    "outlook.office.com",
    "outlook.office365.us",
    "partner.outlook.cn",
];

/// A message as `SyncFolderItems` returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EwsMessage {
    /// Headers and flags. Categories become keywords.
    pub envelope: MailEnvelope,
    /// `DateTimeReceived` in seconds since the epoch.
    pub received_at: i64,
    /// The folder the message is in.
    pub mailboxes: Vec<MailboxId>,
}

/// What one sync round found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPage {
    /// Messages created or updated.
    pub messages: Vec<EwsMessage>,
    /// Messages deleted or moved out of the folder.
    pub removed: Vec<MessageId>,
    /// Read-state changes that came without the rest of the message.
    pub read_changes: Vec<(MessageId, bool)>,
    /// Sync state to save and pass to the next round.
    pub state: String,
    /// `false` while the server has more changes; run another round.
    pub done: bool,
}

/// An EWS client for one on-premises endpoint (`https://host/EWS/Exchange.asmx`).
pub struct EwsClient<'h, H: Http> {
    http: &'h H,
    endpoint: String,
    token: Secret,
}

impl<H: Http> std::fmt::Debug for EwsClient<'_, H> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EwsClient")
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}

impl<'h, H: Http> EwsClient<'h, H> {
    /// A client for `endpoint` that authenticates with `token` as a bearer
    /// token (Exchange Server hybrid modern authentication).
    ///
    /// # Errors
    ///
    /// [`Error::Endpoint`] when `endpoint` is not https, carries user
    /// info, or is an Exchange Online host.
    pub fn new(http: &'h H, endpoint: &str, token: Secret) -> Result<Self, Error> {
        let rest = endpoint
            .strip_prefix("https://")
            .ok_or(Error::Endpoint("not an https URL"))?;
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if authority.contains('@') {
            return Err(Error::Endpoint("user info in the URL"));
        }
        let host = authority
            .split(':')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if host.is_empty() {
            return Err(Error::Endpoint("no host"));
        }
        if EXCHANGE_ONLINE.contains(&host.trim_end_matches('.')) {
            return Err(Error::Endpoint("Exchange Online goes through Graph"));
        }
        Ok(Self {
            http,
            endpoint: endpoint.to_string(),
            token,
        })
    }

    /// One `SyncFolderItems` round over `folder`, a distinguished name such
    /// as `inbox`, from `state` (none for a first sync), returning at most
    /// `max` changes.
    ///
    /// # Errors
    ///
    /// [`Error::Response`] when the server rejects the round (for example a
    /// stale state); transport, status, or SOAP errors.
    pub async fn sync_folder(
        &self,
        folder: &str,
        state: Option<&str>,
        max: u16,
    ) -> Result<SyncPage, Error> {
        let operation = SyncFolderItems {
            item_shape: ItemShape {
                base_shape: BaseShape::AllProperties,
                ..ItemShape::default()
            },
            sync_folder_id: BaseFolderId::new_distinguished(folder),
            sync_state: state.map(str::to_string),
            ignore: None,
            max_changes_returned: max,
            sync_scope: None,
        };
        let response: SyncFolderItemsResponse = self.call(operation).await?;
        let message = match response.into_response_messages().into_iter().next() {
            Some(ResponseClass::Success(message) | ResponseClass::Warning(message)) => message,
            Some(ResponseClass::Error(error)) => {
                return Err(Error::Response {
                    code: format!("{:?}", error.response_code),
                });
            }
            None => return Err(Error::Soap("no response message".into())),
        };
        let mut page = SyncPage {
            messages: Vec::new(),
            removed: Vec::new(),
            read_changes: Vec::new(),
            state: message.sync_state,
            done: message.includes_last_item_in_range,
        };
        for change in message.changes.inner {
            match change {
                Change::Create { item } | Change::Update { item } => {
                    if let Some(mapped) = map(item) {
                        page.messages.push(mapped);
                    }
                }
                Change::Delete { item_id } => page.removed.push(MessageId::new(item_id.id)),
                Change::ReadFlagChange { item_id, is_read } => {
                    page.read_changes
                        .push((MessageId::new(item_id.id), is_read));
                }
            }
        }
        Ok(page)
    }

    async fn call<B>(&self, body: B) -> Result<B::Response, Error>
    where
        B: ews::Operation,
    {
        let envelope = Envelope {
            headers: vec![Header::RequestServerVersion {
                version: ExchangeServerVersion::Exchange2013_SP1,
            }],
            body,
        };
        let document = envelope
            .as_xml_document()
            .map_err(|error| Error::Soap(error.to_string()))?;
        let bearer = format!("Bearer {}", String::from_utf8_lossy(self.token.as_bytes()));
        let mut request = HttpRequest::new(Method::Post, self.endpoint.as_str())
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("Authorization", bearer);
        request.body = document;
        let response = self.http.send(request).await?;
        // EWS reports SOAP faults with a 500 and a fault document.
        if !(200..300).contains(&response.status) && response.status != 500 {
            return Err(Error::Status {
                status: response.status,
            });
        }
        // `ews` 0.1.2 panics on a non-fault document without a SOAP header,
        // so such a document is refused before it reaches the parser.
        if !mentions(&response.body, b"Header") && !mentions(&response.body, b"Fault") {
            return Err(Error::Soap("response has no SOAP header".into()));
        }
        match Envelope::<B::Response>::from_xml_document(&response.body) {
            Ok(envelope) if response.status != 500 => Ok(envelope.body),
            Ok(_) => Err(Error::Status { status: 500 }),
            Err(ews::Error::RequestFault(fault)) => Err(Error::Response {
                code: fault
                    .detail
                    .and_then(|detail| detail.response_code)
                    .map_or_else(|| "SOAP fault".to_string(), |code| format!("{code:?}")),
            }),
            Err(error) => Err(Error::Soap(error.to_string())),
        }
    }
}

fn mentions(document: &[u8], name: &[u8]) -> bool {
    document.windows(name.len()).any(|window| window == name)
}

fn map(item: RealItem) -> Option<EwsMessage> {
    let message = match item {
        RealItem::Message(message) | RealItem::Item(message) => message,
        // Meeting items and other kinds are not mail for the list views.
        _ => return None,
    };
    let id = message.item_id.as_ref()?.id.clone();
    Some(EwsMessage {
        received_at: message
            .date_time_received
            .as_ref()
            .map_or(0, |time| time.0.unix_timestamp()),
        mailboxes: message
            .parent_folder_id
            .as_ref()
            .map(|folder| MailboxId::new(&folder.id))
            .into_iter()
            .collect(),
        envelope: envelope(id, message),
    })
}

fn envelope(id: String, message: Message) -> MailEnvelope {
    let address = |recipient: ews::Recipient| Address {
        name: recipient.mailbox.name.filter(|name| !name.is_empty()),
        email: recipient.mailbox.email_address.unwrap_or_default(),
    };
    let list = |recipients: Option<ews::ArrayOfRecipients>| {
        recipients
            .map(|array| array.0.into_iter().map(address).collect())
            .unwrap_or_default()
    };
    MailEnvelope {
        thread: ThreadId::new(
            message
                .conversation_id
                .map_or_else(|| id.clone(), |conversation| conversation.id),
        ),
        id: MessageId::new(id),
        from: message.from.map_or(
            Address {
                name: None,
                email: String::new(),
            },
            address,
        ),
        to: list(message.to_recipients),
        cc: list(message.cc_recipients),
        subject: message.subject.unwrap_or_default(),
        stamp: message
            .date_time_sent
            .map(|time| format_rfc3339_utc(time.0.unix_timestamp()))
            .unwrap_or_default(),
        snippet: message.preview.unwrap_or_default(),
        flags: Flags {
            seen: message.is_read.unwrap_or(false),
            flagged: message
                .flag
                .and_then(|flag| flag.flag_status)
                .is_some_and(|status| matches!(status, FlagStatus::Flagged)),
            draft: message.is_draft.unwrap_or(false),
            answered: false,
            deleted: false,
            keywords: message
                .categories
                .unwrap_or_default()
                .into_iter()
                .map(|category| category.string)
                .collect(),
        },
        // `HasAttachments` is a yes/no; the count needs a GetItem.
        attachment_count: u32::from(message.has_attachments.unwrap_or(false)),
        transport: TransportSecurity::Tls,
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{AccountId, HttpResponse, MailboxId, MessageId, Secret};
    use mailune_store::{Account, Counts, Mailbox, Store, StoredMessage};
    use mailune_testkit::{ScriptedHttp, poll_now};

    use super::EwsClient;
    use crate::Error;

    const FIRST: &str = include_str!("../fixtures/sync-1.xml");
    const SECOND: &str = include_str!("../fixtures/sync-2.xml");
    const STALE: &str = include_str!("../fixtures/sync-stale.xml");
    const ENDPOINT: &str = "https://mail.example.org/EWS/Exchange.asmx";

    #[test]
    fn a_scripted_sync_lands_in_the_store_and_resumes_from_its_state() {
        let http = ScriptedHttp::new().json(FIRST).json(SECOND);
        let client = EwsClient::new(&http, ENDPOINT, Secret::new("tok-ews")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
        let account = AccountId::new("me@example.org");
        store
            .upsert_account(&Account {
                id: account.clone(),
                email: "me@example.org".into(),
                display_name: None,
                provider: "ews".into(),
            })
            .unwrap();
        let inbox = MailboxId::new("AAMkInbox");
        store
            .upsert_mailbox(&Mailbox {
                account: account.clone(),
                id: inbox.clone(),
                name: "Inbox".into(),
                role: None,
                parent: None,
            })
            .unwrap();

        let mut pages = Vec::new();
        for _ in 0..2 {
            let saved = store.sync_state(&account, "ews:inbox").unwrap();
            let page = poll_now(client.sync_folder("inbox", saved.as_deref(), 100))
                .unwrap()
                .unwrap();
            for message in &page.messages {
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
                .set_sync_state(&account, "ews:inbox", &page.state)
                .unwrap();
            pages.push(page);
        }
        let (first, second) = (&pages[0], &pages[1]);
        assert_eq!(first.messages.len(), 2);
        assert!(!first.done && second.done);
        let ada = &first.messages[0].envelope;
        assert_eq!(ada.from.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(ada.thread.as_str(), "conv-1");
        assert!(ada.flags.flagged && !ada.flags.seen);
        assert_eq!(ada.flags.keywords, ["Work"]);
        assert_eq!(ada.stamp, "2023-11-14T22:13:18Z");
        assert_eq!(first.messages[0].received_at, 1_700_000_000);
        assert_eq!(second.removed, [MessageId::new("m2")]);
        assert_eq!(second.read_changes, [(MessageId::new("m1"), true)]);
        assert_eq!(
            store.mailbox_counts(&account, &inbox).unwrap(),
            Counts {
                total: 3,
                unread: 2
            }
        );

        let requests = http.requests();
        assert_eq!(requests[0].url, ENDPOINT);
        assert_eq!(
            requests[0].header_value("authorization"),
            Some("Bearer tok-ews")
        );
        let first_body = String::from_utf8(requests[0].body.clone()).unwrap();
        assert!(first_body.contains("SyncFolderItems"));
        assert!(first_body.contains(r#"DistinguishedFolderId Id="inbox""#));
        assert!(!first_body.contains("SyncState"));
        let second_body = String::from_utf8(requests[1].body.clone()).unwrap();
        assert!(second_body.contains("state-1</SyncState>"));
        assert!(!format!("{client:?}").contains("tok-ews"));
    }

    #[test]
    fn a_stale_state_and_bad_documents_are_typed_errors() {
        let http = ScriptedHttp::new()
            .json(STALE)
            .json("<Envelope><Body/></Envelope>")
            .reply(HttpResponse::new(401, ""));
        let client = EwsClient::new(&http, ENDPOINT, Secret::new("t")).unwrap();
        assert!(matches!(
            poll_now(client.sync_folder("inbox", Some("old"), 10)).unwrap(),
            Err(Error::Response { code }) if code == "ErrorInvalidSyncStateData"
        ));
        assert!(matches!(
            poll_now(client.sync_folder("inbox", None, 10)).unwrap(),
            Err(Error::Soap(_))
        ));
        assert!(matches!(
            poll_now(client.sync_folder("inbox", None, 10)).unwrap(),
            Err(Error::Status { status: 401 })
        ));
    }

    #[test]
    fn exchange_online_and_unsafe_endpoints_are_refused() {
        let http = ScriptedHttp::new();
        for bad in [
            "https://outlook.office365.com/EWS/Exchange.asmx",
            "https://OUTLOOK.OFFICE.COM./EWS/Exchange.asmx",
            "http://mail.example.org/EWS/Exchange.asmx",
            "https://user@mail.example.org/EWS/Exchange.asmx",
            "https:///EWS",
        ] {
            assert!(
                matches!(
                    EwsClient::new(&http, bad, Secret::new("t")),
                    Err(Error::Endpoint(_))
                ),
                "{bad}"
            );
        }
        assert!(
            EwsClient::new(
                &http,
                "https://mail.example.org:8443/EWS/Exchange.asmx",
                Secret::new("t")
            )
            .is_ok()
        );
    }
}
