//! In-SQLite cosine KNN at the mailbox size in `docs/architecture.md`.
//!
//! The stated budget there is 50 ms for a thread-list page at 100k messages.
//! This bench times [`mailune_store::Store::nearest`] and a usearch cosine
//! index on 100k stored vectors of 32 dimensions, which is that mailbox size.
//! One sample: SQLCipher makes the seed expensive, and the search is what
//! the budget is about. The store implementation stays the in-SQLite scan
//! unless that scan is slower than 50 ms.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use divan::Bencher;
use mailune_store::{Error, Store};

fn main() {
    divan::main();
}

const ROWS: usize = 100_000;
const DIMS: usize = 32;

struct Db {
    _dir: PathBuf,
    store: Store,
}

struct UsearchDb {
    index: usearch::Index,
}

fn vector(index: usize) -> Vec<f32> {
    (0..DIMS)
        .map(|dim| ((index * 17 + dim * 13) % 100) as f32 / 100.0)
        .collect()
}

fn seeded() -> Result<Db, Error> {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mailune-knn-{n}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|_| Error::Open)?;
    let mut store = Store::open(&dir.join("mail.db"), b"bench")?;
    for index in 0..ROWS {
        let id = index.to_string();
        store.upsert_embedding(&id, &vector(index))?;
    }
    Ok(Db { _dir: dir, store })
}

fn usearch_index() -> Result<UsearchDb, ()> {
    let options = usearch::IndexOptions {
        dimensions: DIMS,
        metric: usearch::MetricKind::Cos,
        quantization: usearch::ScalarKind::F32,
        ..usearch::IndexOptions::default()
    };
    let index = usearch::Index::new(&options).map_err(|_| ())?;
    index.reserve(ROWS).map_err(|_| ())?;
    for index_id in 0..ROWS {
        let key = u64::try_from(index_id).map_err(|_| ())?;
        index.add(key, &vector(index_id)).map_err(|_| ())?;
    }
    Ok(UsearchDb { index })
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn sqlite_knn(bencher: Bencher) {
    bencher.with_inputs(seeded).bench_refs(|db| {
        if let Ok(db) = db.as_mut() {
            let _ = divan::black_box(db.store.nearest(&vector(0), 10));
        }
    });
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn usearch_knn(bencher: Bencher) {
    bencher.with_inputs(usearch_index).bench_refs(|db| {
        if let Ok(db) = db.as_ref() {
            let _ = divan::black_box(db.index.search(&vector(0), 10));
        }
    });
}
