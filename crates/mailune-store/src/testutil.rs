//! Test fixtures shared by the store's unit tests: one account, two
//! mailboxes, and a message builder.
//!
//! Clippy treats helpers outside `#[test]` as production code.

#![allow(clippy::unwrap_used)]

use mailune_protocol::{
    AccountId, Address, Envelope, Flags, MailboxId, MailboxRole, MessageId, ThreadId,
    TransportSecurity,
};

use crate::{Account, Mailbox, Store, StoredMessage};

pub(crate) fn account() -> AccountId {
    AccountId::new("acc")
}

pub(crate) fn inbox() -> MailboxId {
    MailboxId::new("inbox")
}

pub(crate) fn seeded() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("mail.db"), None).unwrap();
    store
        .upsert_account(&Account {
            id: account(),
            email: "me@example.com".into(),
            display_name: Some("Me".into()),
            provider: "jmap".into(),
        })
        .unwrap();
    for (id, role) in [
        ("inbox", MailboxRole::Inbox),
        ("archive", MailboxRole::Archive),
    ] {
        store
            .upsert_mailbox(&Mailbox {
                account: account(),
                id: MailboxId::new(id),
                name: id.into(),
                role: Some(role),
                parent: None,
            })
            .unwrap();
    }
    (dir, store)
}

pub(crate) fn message(id: &str, thread: &str, at: i64, seen: bool) -> StoredMessage {
    StoredMessage {
        account: account(),
        envelope: Envelope {
            id: MessageId::new(id),
            thread: ThreadId::new(thread),
            from: Address {
                name: Some("Ada".into()),
                email: "ada@example.com".into(),
            },
            to: vec![Address {
                name: None,
                email: "me@example.com".into(),
            }],
            cc: Vec::new(),
            subject: format!("subject {thread}"),
            stamp: at.to_string(),
            snippet: "hi".into(),
            flags: Flags {
                seen,
                flagged: false,
                draft: false,
                answered: false,
                deleted: false,
                keywords: vec!["work".into()],
            },
            attachment_count: 0,
            transport: TransportSecurity::Tls,
        },
        received_at: at,
        mailboxes: vec![inbox()],
    }
}
