//! S14: the S8 in-SQLite cosine scan against a usearch HNSW index, at the mailbox size in
//! `docs/architecture.md`.
//!
//! Both answer the same top-10 query over the same vectors. The store keeps the S8 scan unless
//! it misses its latency budget; there is no search budget until T6, so the 50 ms the
//! architecture gives a thread-list page at 100k messages stands in for it.
//!
//! Run with `mise exec -- cargo bench -p mailune-store --bench knn`. Seeding the store takes a
//! while; it happens once per bench, and only the search is timed.
//!
//! Helpers sit outside `#[test]`, so clippy holds them to production rules; a failed seed
//! should stop the bench loudly.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use divan::Bencher;
use mailune_protocol::{
    AccountId, Address, Envelope, Flags, MailboxId, MailboxRole, MessageId, ThreadId,
    TransportSecurity,
};
use mailune_store::{Account, ChunkRef, Mailbox, Store, StoredMessage};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn main() {
    divan::main();
}

/// Messages in the target mailbox (`docs/architecture.md`, performance budgets).
const MESSAGES: usize = 100_000;
/// Width of a small local embedding model (MiniLM, bge-small); the scan cost grows with it.
const DIMS: usize = 384;
/// Results a retrieval step asks for.
const TOP_K: usize = 10;
const MODEL: &str = "bench-384";
/// Fixed so every run times the same vectors.
const SEED: u64 = 0x5_14;

fn account() -> AccountId {
    AccountId::new("bench")
}

fn vectors(count: usize) -> Vec<Vec<f32>> {
    let mut rng = StdRng::seed_from_u64(SEED);
    (0..count)
        .map(|_| (0..DIMS).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

fn message(index: usize) -> StoredMessage {
    StoredMessage {
        account: account(),
        envelope: Envelope {
            id: MessageId::new(format!("m{index}")),
            thread: ThreadId::new(format!("t{index}")),
            from: Address {
                name: None,
                email: "ada@example.com".into(),
            },
            to: Vec::new(),
            cc: Vec::new(),
            subject: format!("subject {index}"),
            stamp: index.to_string(),
            snippet: String::new(),
            flags: Flags {
                seen: false,
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
        mailboxes: vec![MailboxId::new("inbox")],
    }
}

/// A store keyed like production where SQLCipher is linked (Apple targets).
fn open(dir: &std::path::Path) -> Store {
    let path = dir.join("mail.db");
    if cfg!(target_vendor = "apple") {
        Store::open(&path, Some(&[7_u8; mailune_store::KEY_LEN])).unwrap()
    } else {
        Store::open(&path, None).unwrap()
    }
}

fn seeded(vectors: &[Vec<f32>]) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
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
            id: MailboxId::new("inbox"),
            name: "Inbox".into(),
            role: Some(MailboxRole::Inbox),
            parent: None,
        })
        .unwrap();
    for (index, vector) in vectors.iter().enumerate() {
        let message = message(index);
        store.upsert_message(&message).unwrap();
        let chunk = ChunkRef {
            message: message.envelope.id.clone(),
            chunk: 0,
        };
        store
            .put_embedding(&account(), &chunk, MODEL, vector)
            .unwrap();
    }
    (dir, store)
}

fn index(vectors: &[Vec<f32>]) -> usearch::Index {
    let options = usearch::IndexOptions {
        dimensions: DIMS,
        metric: usearch::MetricKind::Cos,
        quantization: usearch::ScalarKind::F32,
        ..usearch::IndexOptions::default()
    };
    let index = usearch::Index::new(&options).unwrap();
    index.reserve(vectors.len()).unwrap();
    for (key, vector) in vectors.iter().enumerate() {
        index.add(u64::try_from(key).unwrap(), vector).unwrap();
    }
    index
}

#[divan::bench(sample_count = 10)]
fn s8_sqlite_scan(bencher: Bencher) {
    let vectors = vectors(MESSAGES);
    let (_dir, mut store) = seeded(&vectors);
    let query = &vectors[0];
    bencher
        .bench_local(|| divan::black_box(store.nearest(&account(), MODEL, query, TOP_K).unwrap()));
}

/// The same exact cosine top-k over vectors already in memory: the floor of any exact scan, so
/// the gap to `s8_sqlite_scan` is what loading and decoding the rows costs.
#[divan::bench(sample_count = 20)]
fn exact_scan_in_memory(bencher: Bencher) {
    let vectors = vectors(MESSAGES);
    let query = &vectors[0];
    let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
    let query_norm = norm(query);
    bencher.bench_local(|| {
        let mut scored: Vec<(usize, f32)> = vectors
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let dot: f32 = v.iter().zip(query).map(|(a, b)| a * b).sum();
                (i, dot / (norm(v) * query_norm))
            })
            .collect();
        scored.select_nth_unstable_by(TOP_K, |a, b| b.1.total_cmp(&a.1));
        scored.truncate(TOP_K);
        divan::black_box(scored)
    });
}

#[divan::bench(sample_count = 100)]
fn usearch_hnsw(bencher: Bencher) {
    let vectors = vectors(MESSAGES);
    let index = index(&vectors);
    let query = &vectors[0];
    bencher.bench_local(|| divan::black_box(index.search(query, TOP_K).unwrap()));
}
