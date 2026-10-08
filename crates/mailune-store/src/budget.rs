//! Ceilings for a cold open, a list page, and a search.
//!
//! nextest checks a small mailbox against [`Budget::loose`]. The 100k sample
//! stays in the divan bench and is not run here.

use std::time::Duration;

/// How long each step may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Opening an existing database.
    pub cold_open: Duration,
    /// One `page_messages` call.
    pub list_page: Duration,
    /// One `search_text` call.
    pub search: Duration,
}

impl Budget {
    /// Loose ceilings for a debug SQLCipher build of a small mailbox.
    pub fn loose() -> Self {
        Self {
            cold_open: Duration::from_secs(5),
            list_page: Duration::from_secs(2),
            search: Duration::from_secs(2),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::Budget;
    use crate::{AccountRow, MessageRow, Store, ThreadRow, scratch_dir};

    #[test]
    fn a_small_mailbox_stays_under_the_loose_budget() {
        let budget = Budget::loose();
        let dir = scratch_dir();
        let path = dir.join("mail.db");
        {
            let mut store = Store::open(&path, b"key").unwrap();
            store
                .upsert_account(&AccountRow {
                    id: "a".into(),
                    email: "a@example.com".into(),
                })
                .unwrap();
            store
                .upsert_thread(&ThreadRow {
                    id: "t".into(),
                    account_id: "a".into(),
                    subject: "Hello".into(),
                })
                .unwrap();
            for i in 0..8_i64 {
                let id = format!("m{i}");
                store
                    .upsert_message(&MessageRow {
                        id: id.clone(),
                        account_id: "a".into(),
                        thread_id: "t".into(),
                        subject: "Hello".into(),
                        from_email: "ada@example.com".into(),
                        to_emails: "me@example.com".into(),
                        stamp: "2026-03-01T12:00:00Z".into(),
                        received_at: i,
                    })
                    .unwrap();
                store
                    .index_message(&id, "Hello", "ada@example.com", "dock")
                    .unwrap();
            }
        }
        let started = Instant::now();
        let mut store = Store::open(&path, b"key").unwrap();
        let opened = started.elapsed();
        let started = Instant::now();
        let page = store.page_messages(None, 50).unwrap();
        let listed = started.elapsed();
        let started = Instant::now();
        let hits = store.search_text("dock").unwrap();
        let searched = started.elapsed();
        assert_eq!(page.messages.len(), 8);
        assert_eq!(hits.len(), 8);
        assert!(opened <= budget.cold_open);
        assert!(listed <= budget.list_page);
        assert!(searched <= budget.search);
        let _ = std::fs::remove_dir_all(dir);
    }
}
