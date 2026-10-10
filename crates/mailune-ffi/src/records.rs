//! Plain records the foreign side sees, one per contract or view-model type.
//!
//! The contract types stay free of UniFFI derives, so a binding never decides
//! how they change. The price is a conversion per record, both ways where the
//! foreign side sends the value back. Ids cross as strings: providers spell
//! them differently and the contract does not interpret them either.
//!
//! The serde derives give the JSON that `mailune-capi` carries over its C
//! ABI. It is the contract's JSON too (snake_case, externally tagged), so a
//! fake core that replays contract scenarios speaks it unchanged.

use mailune_app::{Msg, Pending, Ui, Views};
use mailune_protocol as proto;

/// Someone a message is from, to or copied to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Address {
    /// Display name when the header had one.
    pub name: Option<String>,
    /// Addr-spec (`ana@acme.io`).
    pub email: String,
}

/// Triage tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Primary.
    Primary,
    /// Social.
    Social,
    /// Promotions.
    Promotions,
    /// Updates.
    Updates,
}

/// One row in the thread list.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThreadRow {
    /// Conversation id.
    pub id: String,
    /// Owning account id.
    pub account: String,
    /// Sender shown on the row.
    pub from: Address,
    /// Subject.
    pub subject: String,
    /// One-line preview.
    pub snippet: String,
    /// Display stamp.
    pub stamp: String,
    /// How many messages the conversation holds.
    pub message_count: u32,
    /// The row is unread.
    pub unread: bool,
    /// Starred.
    pub flagged: bool,
    /// Marked important.
    pub important: bool,
    /// Pinned to the top of the list.
    pub pinned: bool,
    /// Hidden until a snooze wakes it.
    pub snoozed: bool,
    /// The conversation is a draft.
    pub draft: bool,
    /// At least one message has an attachment.
    pub has_attachment: bool,
    /// Triage tab.
    pub category: Category,
    /// Mailbox id the row was listed from.
    pub mailbox: String,
    /// Label texts.
    pub labels: Vec<String>,
}

/// A fact the UI renders.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Event {
    /// A sentence the UI shows as a notice.
    Notice {
        /// Text the UI can show. Not a message body.
        message: String,
    },
    /// The current page of the thread list.
    Snapshot {
        /// Rows in list order.
        threads: Vec<ThreadRow>,
    },
}

/// The composer draft.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ComposerDraft {
    /// Recipient address, as the notice wrote it.
    pub to: String,
    /// Subject line.
    pub subject: String,
    /// Plain body.
    pub body: String,
}

/// Settings the UI is showing.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Settings {
    /// Language code; English is `en`.
    pub language: String,
    /// Plan key (`free`, `plus`, `team`).
    pub plan: String,
}

/// Everything a shell renders, read in one call.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ViewState {
    /// Thread list rows in list order.
    pub list: Vec<ThreadRow>,
    /// The open conversation, while the list still holds it.
    pub open: Option<ThreadRow>,
    /// Composer draft.
    pub composer: ComposerDraft,
    /// Settings.
    pub settings: Settings,
}

/// Something the user did, or something the core said: the input of the
/// shared reducer (`mailune_app::Msg`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum UiMsg {
    /// Events from the core.
    Core {
        /// Events in order.
        events: Vec<Event>,
    },
    /// The user opened a row.
    Select {
        /// Conversation id.
        thread: String,
    },
    /// The user opened an empty composer.
    ComposeNew,
    /// The user edited the composer.
    EditDraft {
        /// The composer as it is now.
        draft: ComposerDraft,
    },
    /// The user closed the composer; the draft is saved, not sent.
    CloseComposer,
    /// The user pressed send. Nothing leaves until `Confirm`.
    Send,
    /// The user asked to delete conversations. Nothing leaves until `Confirm`.
    Delete {
        /// Conversation ids.
        threads: Vec<String>,
    },
    /// The user archived conversations.
    Archive {
        /// Conversation ids.
        threads: Vec<String>,
    },
    /// The user confirmed the pending action.
    Confirm,
    /// The user dismissed the pending action.
    Cancel,
    /// The user ran a search.
    Search {
        /// The query string.
        query: String,
    },
}

/// The confirmation a shell must show.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PendingAction {
    /// Send the composer draft.
    Send,
    /// Move these conversations to Trash.
    Delete {
        /// Conversation ids.
        threads: Vec<String>,
    },
}

/// Everything the shared reducer says a shell renders.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UiState {
    /// The view models.
    pub view: ViewState,
    /// The composer is on screen.
    pub composing: bool,
    /// The confirmation to show, if any.
    pub pending: Option<PendingAction>,
    /// The last search query.
    pub query: String,
}

/// Which secret the host keychain is asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SecretKind {
    /// OAuth or other bearer token.
    Token,
    /// IMAP or SMTP password.
    Password,
    /// SQLCipher key for the local database.
    DbKey,
}

impl From<proto::Address> for Address {
    fn from(value: proto::Address) -> Self {
        Self {
            name: value.name,
            email: value.email,
        }
    }
}

impl From<Address> for proto::Address {
    fn from(value: Address) -> Self {
        Self {
            name: value.name,
            email: value.email,
        }
    }
}

impl From<proto::Category> for Category {
    fn from(value: proto::Category) -> Self {
        match value {
            proto::Category::Primary => Self::Primary,
            proto::Category::Social => Self::Social,
            proto::Category::Promotions => Self::Promotions,
            proto::Category::Updates => Self::Updates,
        }
    }
}

impl From<Category> for proto::Category {
    fn from(value: Category) -> Self {
        match value {
            Category::Primary => Self::Primary,
            Category::Social => Self::Social,
            Category::Promotions => Self::Promotions,
            Category::Updates => Self::Updates,
        }
    }
}

impl From<proto::ThreadRow> for ThreadRow {
    fn from(row: proto::ThreadRow) -> Self {
        Self {
            id: row.id.as_str().to_string(),
            account: row.account.as_str().to_string(),
            from: row.from.into(),
            subject: row.subject,
            snippet: row.snippet,
            stamp: row.stamp,
            message_count: row.message_count,
            unread: row.unread,
            flagged: row.flagged,
            important: row.important,
            pinned: row.pinned,
            snoozed: row.snoozed,
            draft: row.draft,
            has_attachment: row.has_attachment,
            category: row.category.into(),
            mailbox: row.mailbox.as_str().to_string(),
            labels: row.labels,
        }
    }
}

impl From<ThreadRow> for proto::ThreadRow {
    fn from(row: ThreadRow) -> Self {
        Self {
            id: proto::ThreadId::new(row.id),
            account: proto::AccountId::new(row.account),
            from: row.from.into(),
            subject: row.subject,
            snippet: row.snippet,
            stamp: row.stamp,
            message_count: row.message_count,
            unread: row.unread,
            flagged: row.flagged,
            important: row.important,
            pinned: row.pinned,
            snoozed: row.snoozed,
            draft: row.draft,
            has_attachment: row.has_attachment,
            category: row.category.into(),
            mailbox: proto::MailboxId::new(row.mailbox),
            labels: row.labels,
        }
    }
}

impl From<proto::Event> for Event {
    fn from(event: proto::Event) -> Self {
        match event {
            proto::Event::Notice { message } => Self::Notice { message },
            proto::Event::Snapshot { threads } => Self::Snapshot {
                threads: threads.into_iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<Event> for proto::Event {
    fn from(event: Event) -> Self {
        match event {
            Event::Notice { message } => Self::Notice { message },
            Event::Snapshot { threads } => Self::Snapshot {
                threads: threads.into_iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<&Views> for ViewState {
    fn from(views: &Views) -> Self {
        Self {
            list: views.list.rows.iter().cloned().map(Into::into).collect(),
            open: views.open.row.clone().map(Into::into),
            composer: ComposerDraft {
                to: views.composer.to.clone(),
                subject: views.composer.subject.clone(),
                body: views.composer.body.clone(),
            },
            settings: Settings {
                language: views.settings.language.clone(),
                plan: views.settings.plan.clone(),
            },
        }
    }
}

fn thread_ids(threads: Vec<String>) -> Vec<proto::ThreadId> {
    threads.into_iter().map(proto::ThreadId::new).collect()
}

impl From<UiMsg> for Msg {
    fn from(msg: UiMsg) -> Self {
        match msg {
            UiMsg::Core { events } => Self::Core(events.into_iter().map(Into::into).collect()),
            UiMsg::Select { thread } => Self::Select(proto::ThreadId::new(thread)),
            UiMsg::ComposeNew => Self::ComposeNew,
            UiMsg::EditDraft { draft } => Self::EditDraft(mailune_app::ComposerDraft {
                to: draft.to,
                subject: draft.subject,
                body: draft.body,
            }),
            UiMsg::CloseComposer => Self::CloseComposer,
            UiMsg::Send => Self::Send,
            UiMsg::Delete { threads } => Self::Delete(thread_ids(threads)),
            UiMsg::Archive { threads } => Self::Archive(thread_ids(threads)),
            UiMsg::Confirm => Self::Confirm,
            UiMsg::Cancel => Self::Cancel,
            UiMsg::Search { query } => Self::Search(query),
        }
    }
}

impl From<&Pending> for PendingAction {
    fn from(pending: &Pending) -> Self {
        match pending {
            Pending::Send => Self::Send,
            Pending::Delete(threads) => Self::Delete {
                threads: threads.iter().map(|id| id.as_str().to_string()).collect(),
            },
        }
    }
}

impl From<&Ui> for UiState {
    fn from(ui: &Ui) -> Self {
        Self {
            view: ViewState::from(&ui.views),
            composing: ui.composing,
            pending: ui.pending.as_ref().map(Into::into),
            query: ui.query.clone(),
        }
    }
}

impl From<proto::SecretKind> for SecretKind {
    fn from(kind: proto::SecretKind) -> Self {
        match kind {
            proto::SecretKind::Token => Self::Token,
            proto::SecretKind::Password => Self::Password,
            proto::SecretKind::DbKey => Self::DbKey,
        }
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol as proto;

    use super::{Address, Category, Event, ThreadRow};

    fn row(id: &str) -> proto::ThreadRow {
        proto::ThreadRow {
            id: proto::ThreadId::new(id),
            account: proto::AccountId::new("local"),
            from: proto::Address {
                name: Some("Ada".into()),
                email: "ada@example.com".into(),
            },
            subject: "Hello".into(),
            snippet: "plain".into(),
            stamp: "t0".into(),
            message_count: 2,
            unread: true,
            flagged: true,
            important: false,
            pinned: false,
            snoozed: false,
            draft: false,
            has_attachment: true,
            category: proto::Category::Social,
            mailbox: proto::MailboxId::new("inbox"),
            labels: vec!["work".into()],
        }
    }

    #[test]
    fn a_thread_row_round_trips_through_the_record() {
        let record = ThreadRow::from(row("t1"));
        assert_eq!(record.id, "t1");
        assert_eq!(record.category, Category::Social);
        assert_eq!(proto::ThreadRow::from(record), row("t1"));
    }

    /// The same bytes `desktop/android` expects in `AddressTest`, so the
    /// hand-written Kotlin converter cannot drift from what uniffi writes.
    #[test]
    fn the_address_layout_is_the_one_the_kotlin_test_reads() {
        let mut buf = Vec::new();
        <Address as uniffi::Lower<crate::UniFfiTag>>::write(
            Address {
                name: Some("É".into()),
                email: "a@b".into(),
            },
            &mut buf,
        );
        assert_eq!(
            buf,
            [1, 0, 0, 0, 2, 0xC3, 0x89, 0, 0, 0, 3, b'a', b'@', b'b']
        );
    }

    #[test]
    fn the_record_json_is_the_contract_json() {
        let event = proto::Event::Snapshot {
            threads: vec![row("t1")],
        };
        assert_eq!(
            serde_json::to_value(Event::from(event.clone())).unwrap(),
            serde_json::to_value(event).unwrap()
        );
    }

    #[test]
    fn events_round_trip_through_the_record() {
        let events = [
            proto::Event::Notice {
                message: "open t1".into(),
            },
            proto::Event::Snapshot {
                threads: vec![row("t1"), row("t2")],
            },
        ];
        for event in events {
            assert_eq!(proto::Event::from(Event::from(event.clone())), event);
        }
    }
}
