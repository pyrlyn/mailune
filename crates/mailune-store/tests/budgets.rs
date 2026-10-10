//! T6: a small mailbox stays far under the performance budgets in `docs/architecture.md` on
//! every `nextest` run, debug build included. The ceilings are ten times the 100k budgets, so
//! only a gross regression (a scan per row, a missing index, a cache that never fills) trips
//! them; the 100k numbers come from `benches/budgets.rs`.

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used)]

mod support;

use std::time::{Duration, Instant};

use support::{
    MODEL, PAGE, TOP_K, account, inbox, insert_messages, open, query, seed, seed_account,
};

const MESSAGES: usize = 300;
/// Budgets 300 ms, 50 ms and 100 ms, times ten.
const COLD_OPEN_CEILING: Duration = Duration::from_millis(3_000);
const LIST_PAGE_CEILING: Duration = Duration::from_millis(500);
const SEARCH_CEILING: Duration = Duration::from_millis(1_000);

fn timed<T>(run: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let value = run();
    (value, start.elapsed())
}

#[test]
fn a_small_mailbox_opens_lists_and_searches_under_loose_ceilings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mail.db");
    let mut store = open(&path);
    seed(&mut store, MESSAGES);
    drop(store);

    let ((mut store, first), open_time) = timed(|| {
        let mut store = open(&path);
        let page = store.thread_page(&account(), &inbox(), None, PAGE).unwrap();
        (store, page)
    });
    assert_eq!(first.threads.len(), PAGE);
    assert!(
        open_time < COLD_OPEN_CEILING,
        "cold open took {open_time:?}"
    );

    let (_, page_time) = timed(|| store.thread_page(&account(), &inbox(), None, PAGE).unwrap());
    assert!(
        page_time < LIST_PAGE_CEILING,
        "list page took {page_time:?}"
    );

    let query = query();
    store.nearest(&account(), MODEL, &query, TOP_K).unwrap();
    let (hits, search_time) = timed(|| store.nearest(&account(), MODEL, &query, TOP_K).unwrap());
    assert_eq!(hits.len(), TOP_K);
    assert!(search_time < SEARCH_CEILING, "search took {search_time:?}");
}

/// S12: the insert and text search paths the 100k bench measures, at a size `nextest` runs in
/// well under a second. Neither has a budget yet, so this checks the result, not the time.
#[test]
fn a_small_mailbox_inserts_and_is_found_by_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("mail.db"));
    seed_account(&mut store);
    insert_messages(&mut store, MESSAGES);

    let counts = store.mailbox_counts(&account(), &inbox()).unwrap();
    assert_eq!(counts.total, u64::try_from(MESSAGES).unwrap());
    assert_eq!(
        store.search_text(&account(), "ada", PAGE).unwrap().len(),
        PAGE
    );
    let hits = store.search_text(&account(), "subject 42", PAGE).unwrap();
    let mut ids: Vec<&str> = hits.iter().map(|id| id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, ["m126", "m127", "m128"]);
}
