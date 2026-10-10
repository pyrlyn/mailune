//! Mailbox fixture shared by the T6 budget test (`tests/budgets.rs`) and bench
//! (`benches/budgets.rs`): one account, one inbox, three messages per thread, one embedding per
//! message. The insert bench and test write the same messages without embeddings.
//!
//! Helpers sit outside `#[test]`, so clippy holds them to production rules; a failed seed
//! should stop the run loudly.

#![allow(clippy::unwrap_used, dead_code)]

use std::path::Path;

use mailune_protocol::{
    AccountId, Address, Envelope, Flags, MailboxId, MailboxRole, MessageId, ThreadId,
    TransportSecurity,
};
use mailune_store::{Account, ChunkRef, KEY_LEN, Mailbox, Store, StoredMessage};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Width of a small local embedding model (MiniLM, bge-small).
pub const DIMS: usize = 384;
/// Threads in one list page.
pub const PAGE: usize = 50;
/// Results one retrieval step asks for.
pub const TOP_K: usize = 10;
pub const MODEL: &str = "budget-384";
const MESSAGES_PER_THREAD: usize = 3;
/// Fixed so every run measures the same mailbox.
const SEED: u64 = 0x76;

pub fn account() -> AccountId {
    AccountId::new("budget")
}

pub fn inbox() -> MailboxId {
    MailboxId::new("inbox")
}

/// A store keyed like production where SQLCipher is linked (Apple targets).
pub fn open(path: &Path) -> Store {
    if cfg!(target_vendor = "apple") {
        Store::open(path, Some(&[7_u8; KEY_LEN])).unwrap()
    } else {
        Store::open(path, None).unwrap()
    }
}

/// A vector unlike the stored ones, so a search scores every row.
pub fn query() -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(!SEED);
    vector(&mut rng)
}

/// Seeds `messages` messages, each with one embedding.
pub fn seed(store: &mut Store, messages: usize) {
    seed_account(store);
    let mut rng = StdRng::seed_from_u64(SEED);
    for index in 0..messages {
        let message = message(index);
        store.upsert_message(&message).unwrap();
        let chunk = ChunkRef {
            message: message.envelope.id.clone(),
            chunk: 0,
        };
        store
            .put_embedding(&account(), &chunk, MODEL, &vector(&mut rng))
            .unwrap();
    }
}

/// Inserts `messages` messages without embeddings, one upsert each, as a first sync writes them.
pub fn insert_messages(store: &mut Store, messages: usize) {
    for index in 0..messages {
        store.upsert_message(&message(index)).unwrap();
    }
}

/// The account and inbox every message belongs to.
pub fn seed_account(store: &mut Store) {
    store
        .upsert_account(&Account {
            id: account(),
            email: "me@example.com".into(),
            display_name: None,
            provider: "jmap".into(),
        })
        .unwrap();
    store
        .upsert_mailbox(&Mailbox {
            account: account(),
            id: inbox(),
            name: "Inbox".into(),
            role: Some(MailboxRole::Inbox),
            parent: None,
        })
        .unwrap();
}

fn vector(rng: &mut StdRng) -> Vec<f32> {
    (0..DIMS).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

fn message(index: usize) -> StoredMessage {
    let thread = index / MESSAGES_PER_THREAD;
    StoredMessage {
        account: account(),
        envelope: Envelope {
            id: MessageId::new(format!("m{index}")),
            thread: ThreadId::new(format!("t{thread}")),
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
            stamp: index.to_string(),
            snippet: "snippet".into(),
            flags: Flags {
                seen: index.is_multiple_of(2),
                flagged: false,
                draft: false,
                answered: false,
                deleted: false,
                keywords: Vec::new(),
            },
            attachment_count: 0,
            transport: TransportSecurity::Tls,
        },
        received_at: i64::try_from(index).unwrap(),
        mailboxes: vec![inbox()],
    }
}
