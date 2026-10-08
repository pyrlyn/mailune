//! View models for the thread list, the open thread, the composer, and settings.
//!
//! A snapshot replaces the list and refreshes the open row when that id is
//! still in the list. A notice whose first line is `open`, `draft`, or
//! `settings` updates the other models. Any other notice is not a view, so
//! a banner does not wipe a draft.

use mailune_protocol::{Event, ThreadId, ThreadRow};

/// The thread list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThreadList {
    /// Rows in list order.
    pub rows: Vec<ThreadRow>,
}

/// The conversation the UI has open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenThread {
    /// The row, when the list still contains the selected id.
    pub row: Option<ThreadRow>,
}

/// The composer. Filled from a `draft` notice.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComposerDraft {
    /// Recipient address, as the notice wrote it.
    pub to: String,
    /// Subject line.
    pub subject: String,
    /// Plain body.
    pub body: String,
}

/// Settings the UI is showing. Language starts as English.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    /// Language code. Empty is not used; English is `en`.
    pub language: String,
    /// Plan key (`free`, `plus`, `team`).
    pub plan: String,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            language: "en".into(),
            plan: "free".into(),
        }
    }
}

/// The four models a surface renders. [`Views::fold`] is the only writer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Views {
    /// Thread list.
    pub list: ThreadList,
    /// Open conversation.
    pub open: OpenThread,
    /// Composer draft.
    pub composer: ComposerDraft,
    /// Settings.
    pub settings: SettingsSnapshot,
    selected: Option<ThreadId>,
}

impl Views {
    /// Empty list, no open thread, empty draft, English on the free plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies `events` in order.
    pub fn fold(&mut self, events: &[Event]) {
        for event in events {
            match event {
                Event::Snapshot { threads } => {
                    self.list.rows.clone_from(threads);
                    self.open.row = self
                        .selected
                        .as_ref()
                        .and_then(|id| threads.iter().find(|row| &row.id == id).cloned());
                }
                Event::Notice { message } => self.notice(message),
            }
        }
    }

    fn notice(&mut self, message: &str) {
        let mut lines = message.lines();
        let Some(head) = lines.next() else {
            return;
        };
        if let Some(id) = head.strip_prefix("open ")
            && !id.is_empty()
        {
            self.selected = Some(ThreadId::new(id));
            self.open.row = self
                .list
                .rows
                .iter()
                .find(|row| row.id.as_str() == id)
                .cloned();
            return;
        }
        let rest: Vec<&str> = lines.collect();
        match head {
            "draft" => {
                self.composer = ComposerDraft {
                    to: field(&rest, "to"),
                    subject: field(&rest, "subject"),
                    body: field(&rest, "body"),
                };
            }
            "settings" => {
                self.settings = SettingsSnapshot {
                    language: field(&rest, "language"),
                    plan: field(&rest, "plan"),
                };
            }
            _ => {}
        }
    }
}

fn field(lines: &[&str], name: &str) -> String {
    let prefix = format!("{name}: ");
    lines
        .iter()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{AccountId, Address, Category, Event, MailboxId, ThreadId, ThreadRow};

    use super::Views;

    fn row(id: &str, unread: bool) -> ThreadRow {
        ThreadRow {
            id: ThreadId::new(id),
            account: AccountId::new("local"),
            from: Address {
                name: None,
                email: "ada@example.com".into(),
            },
            subject: "Hello".into(),
            snippet: "plain".into(),
            stamp: "t0".into(),
            message_count: 1,
            unread,
            flagged: false,
            important: false,
            pinned: false,
            snoozed: false,
            draft: false,
            has_attachment: false,
            category: Category::Primary,
            mailbox: MailboxId::new("inbox"),
            labels: Vec::new(),
        }
    }

    #[test]
    fn folding_events_updates_list_thread_draft_and_settings() {
        let mut views = Views::new();
        views.fold(&[
            Event::Snapshot {
                threads: vec![row("t1", true), row("t2", false)],
            },
            Event::Notice {
                message: "open t1".into(),
            },
            Event::Notice {
                message: "draft\nto: ada@example.com\nsubject: Hi\nbody: Hello there".into(),
            },
            Event::Notice {
                message: "settings\nlanguage: fr\nplan: plus".into(),
            },
            Event::Snapshot {
                threads: vec![row("t1", false), row("t2", false)],
            },
        ]);
        assert_eq!(views.list.rows.len(), 2);
        assert!(!views.list.rows[0].unread);
        let open = views.open.row.as_ref().unwrap();
        assert_eq!(open.id, ThreadId::new("t1"));
        assert!(!open.unread);
        assert_eq!(views.composer.to, "ada@example.com");
        assert_eq!(views.composer.subject, "Hi");
        assert_eq!(views.composer.body, "Hello there");
        assert_eq!(views.settings.language, "fr");
        assert_eq!(views.settings.plan, "plus");
    }

    #[test]
    fn a_banner_does_not_clear_the_draft() {
        let mut views = Views::new();
        views.fold(&[
            Event::Notice {
                message: "draft\nto: ada@example.com\nsubject: Hi\nbody: Hello".into(),
            },
            Event::Notice {
                message: "signed out".into(),
            },
        ]);
        assert_eq!(views.composer.subject, "Hi");
        assert_eq!(views.settings.language, "en");
    }
}
