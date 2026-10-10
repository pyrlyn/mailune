//! T6 budgets at the mailbox size in `docs/architecture.md`: 100k messages, one 384-dimension
//! embedding each, SQLCipher-keyed on Apple targets. S12 adds 100k inserts and a text search,
//! which have no budget yet.
//!
//! The budgets live in `docs/architecture.md`; `tests/budgets.rs` checks a small mailbox against
//! loose ceilings in every `nextest` run. Run this with
//! `mise exec -- cargo bench -p mailune-store --bench budgets`. The first run seeds the mailbox
//! into Cargo's target temp directory, which takes minutes; later runs reuse it.
//!
//! Helpers sit outside `#[test]`, so clippy holds them to production rules; a failed seed
//! should stop the bench loudly.

#![allow(clippy::unwrap_used)]

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    divan::main();
}

#[cfg(not(target_arch = "wasm32"))]
#[path = "../tests/support/mod.rs"]
mod support;

// The store is empty on wasm32, but a `harness = false` bench still needs a `main` there.
#[cfg(not(target_arch = "wasm32"))]
mod budgets {
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use divan::Bencher;

    use super::support::{
        MODEL, PAGE, TOP_K, account, inbox, insert_messages, open, query, seed, seed_account,
    };

    /// Messages in the target mailbox.
    const MESSAGES: usize = 100_000;
    /// Bump when the fixture changes, so a stale file is not reused.
    const FIXTURE: &str = "mailune-budgets-v1-100k.db";

    /// The seeded file, built once and kept between runs.
    fn mailbox() -> &'static Path {
        static PATH: OnceLock<PathBuf> = OnceLock::new();
        PATH.get_or_init(|| {
            let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
            let path = dir.join(FIXTURE);
            if !path.exists() {
                // Seeded under another name and renamed, so an interrupted seed is never reused.
                let partial = dir.join(format!("{FIXTURE}.partial"));
                let _ = std::fs::remove_file(&partial);
                let mut store = open(&partial);
                seed(&mut store, MESSAGES);
                drop(store);
                std::fs::rename(&partial, &path).unwrap();
            }
            // A fixture seeded before a schema change migrates here, not inside a timed open.
            drop(open(&path));
            path
        })
    }

    /// S12: a first sync of the whole mailbox, one upsert per message, into a fresh file.
    #[divan::bench(sample_count = 3, sample_size = 1)]
    fn insert_100k(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let dir = tempfile::tempdir().unwrap();
                let mut store = open(&dir.path().join("mail.db"));
                seed_account(&mut store);
                (dir, store)
            })
            .bench_local_values(|(dir, mut store)| {
                insert_messages(&mut store, MESSAGES);
                (dir, store)
            });
    }

    /// S12: a full-text search for a word every message holds, so BM25 ranks all of them.
    #[divan::bench(sample_count = 50)]
    fn text_search(bencher: Bencher) {
        let mut store = open(mailbox());
        bencher
            .bench_local(|| divan::black_box(store.search_text(&account(), "ada", PAGE).unwrap()));
    }

    /// Opening the file and showing the first list page: what launch waits on.
    #[divan::bench(sample_count = 20)]
    fn cold_open_to_first_page(bencher: Bencher) {
        let path = mailbox();
        bencher.bench(|| {
            let mut store = open(path);
            divan::black_box(store.thread_page(&account(), &inbox(), None, PAGE).unwrap())
        });
    }

    #[divan::bench(sample_count = 50)]
    fn list_page(bencher: Bencher) {
        let mut store = open(mailbox());
        bencher.bench_local(|| {
            divan::black_box(store.thread_page(&account(), &inbox(), None, PAGE).unwrap())
        });
    }

    /// A search once the vectors are in memory: every query after the first.
    #[divan::bench(sample_count = 50)]
    fn search(bencher: Bencher) {
        let mut store = open(mailbox());
        let query = query();
        store.nearest(&account(), MODEL, &query, TOP_K).unwrap();
        bencher.bench_local(|| {
            divan::black_box(store.nearest(&account(), MODEL, &query, TOP_K).unwrap())
        });
    }

    /// The first search after open, which loads and decrypts every vector into the cache.
    #[divan::bench(sample_count = 5)]
    fn first_search_after_open(bencher: Bencher) {
        let path = mailbox();
        let query = query();
        bencher
            .with_inputs(|| open(path))
            .bench_local_values(|mut store| {
                divan::black_box(store.nearest(&account(), MODEL, &query, TOP_K).unwrap())
            });
    }
}
