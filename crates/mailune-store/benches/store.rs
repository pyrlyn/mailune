//! Insert, list-page, and search throughput.
//!
//! Each sample opens a fresh database. One sample is enough: SQLCipher makes
//! 100k writes expensive, and nextest covers the same calls with a few rows.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use divan::Bencher;
use mailune_store::{AccountRow, Error, MessageRow, Store, ThreadRow};

fn main() {
    divan::main();
}

const ROWS: usize = 100_000;

struct Db {
    _dir: PathBuf,
    store: Store,
}

fn empty() -> Result<Db, Error> {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mailune-bench-{n}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|_| Error::Open)?;
    let store = Store::open(&dir.join("mail.db"), b"bench")?;
    Ok(Db { _dir: dir, store })
}

fn seed(store: &mut Store, rows: usize, index: bool) -> Result<(), Error> {
    store.upsert_account(&AccountRow {
        id: "a".into(),
        email: "a@example.com".into(),
    })?;
    store.upsert_thread(&ThreadRow {
        id: "t".into(),
        account_id: "a".into(),
        subject: "Hello".into(),
    })?;
    for i in 0..rows {
        let id = i.to_string();
        let received_at = i64::try_from(i).unwrap_or(0);
        store.upsert_message(&MessageRow {
            id: id.clone(),
            account_id: "a".into(),
            thread_id: "t".into(),
            subject: "Hello".into(),
            from_email: "ada@example.com".into(),
            to_emails: "me@example.com".into(),
            stamp: "2026-03-01T12:00:00Z".into(),
            received_at,
        })?;
        if index {
            store.index_message(&id, "Hello", "ada@example.com", "dock")?;
        }
    }
    Ok(())
}

fn ready(index: bool) -> Result<Db, Error> {
    let mut db = empty()?;
    seed(&mut db.store, ROWS, index)?;
    Ok(db)
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn inserts(bencher: Bencher) {
    bencher.with_inputs(empty).bench_refs(|db| {
        if let Ok(db) = db.as_mut() {
            let _ = divan::black_box(seed(&mut db.store, ROWS, false));
        }
    });
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn list_page(bencher: Bencher) {
    bencher.with_inputs(|| ready(false)).bench_refs(|db| {
        if let Ok(db) = db.as_mut() {
            let _ = divan::black_box(db.store.page_messages(None, 50));
        }
    });
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn search(bencher: Bencher) {
    bencher.with_inputs(|| ready(true)).bench_refs(|db| {
        if let Ok(db) = db.as_mut() {
            let _ = divan::black_box(db.store.search_text("Hello"));
        }
    });
}
