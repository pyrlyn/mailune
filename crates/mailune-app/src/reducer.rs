//! The shared UI state machine: one pure reducer every shell renders.
//!
//! Crux-style. A shell turns a gesture into a [`Msg`], calls [`Ui::update`],
//! and renders the new [`Ui`]. The reducer never does I/O: what it wants the
//! core to do becomes a typed [`Submission`] in [`Ui::outbox`], which the
//! runtime drains. Core events come back in as [`Msg::Core`] and fold into
//! the B1 [`Views`]. Because every shell runs this same code, a send that
//! needs confirmation needs it in SwiftUI, Compose, WinUI and GTK alike.

use mailune_protocol::{Address, Event, Submission, ThreadId};

use crate::views::{ComposerDraft, Views};

/// Something the user did, or something the core said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// Events from the core, folded into the view models.
    Core(Vec<Event>),
    /// The user opened a row.
    Select(ThreadId),
    /// The user opened an empty composer.
    ComposeNew,
    /// The user edited the composer.
    EditDraft(ComposerDraft),
    /// The user closed the composer; the draft is saved, not sent.
    CloseComposer,
    /// The user pressed send. Nothing leaves until [`Msg::Confirm`].
    Send,
    /// The user asked to delete conversations. Nothing leaves until [`Msg::Confirm`].
    Delete(Vec<ThreadId>),
    /// The user archived conversations. Reversible, so no confirmation.
    Archive(Vec<ThreadId>),
    /// The user confirmed the pending action.
    Confirm,
    /// The user dismissed the pending action.
    Cancel,
    /// The user ran a search.
    Search(String),
}

/// An action waiting for the user's confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// Send the composer draft.
    Send,
    /// Move these conversations to Trash.
    Delete(Vec<ThreadId>),
}

/// Everything a shell renders, and the submissions waiting for the core.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ui {
    /// The list, open thread, composer and settings view models.
    pub views: Views,
    /// The composer is on screen.
    pub composing: bool,
    /// The confirmation the shell must show, if any.
    pub pending: Option<Pending>,
    /// The last search query.
    pub query: String,
    /// Submissions for the core, oldest first. Drained by the runtime.
    pub outbox: Vec<Submission>,
}

impl Ui {
    /// Empty views, no composer, nothing pending.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies `msg`. Pure: state changes and queued submissions only.
    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Core(events) => self.views.fold(&events),
            Msg::Select(thread) => self.outbox.push(Submission::OpenThread { thread }),
            Msg::ComposeNew => {
                self.views.composer = ComposerDraft::default();
                self.composing = true;
            }
            Msg::EditDraft(draft) => self.views.composer = draft,
            Msg::CloseComposer => {
                if self.composing {
                    let draft = &self.views.composer;
                    self.outbox.push(Submission::SaveDraft {
                        to: recipients(&draft.to),
                        subject: draft.subject.clone(),
                        body: draft.body.clone(),
                    });
                }
                self.composing = false;
                self.pending = None;
            }
            // A send with nobody to send to cannot be confirmed into one.
            Msg::Send if self.composing && !recipients(&self.views.composer.to).is_empty() => {
                self.pending = Some(Pending::Send);
            }
            Msg::Send => {}
            Msg::Delete(threads) if !threads.is_empty() => {
                self.pending = Some(Pending::Delete(threads));
            }
            Msg::Delete(_) => {}
            Msg::Archive(threads) => {
                if !threads.is_empty() {
                    self.outbox.push(Submission::Archive { threads });
                }
            }
            Msg::Confirm => self.confirm(),
            Msg::Cancel => self.pending = None,
            Msg::Search(query) => {
                self.query.clone_from(&query);
                self.outbox.push(Submission::Search { query });
            }
        }
    }

    /// Takes the queued submissions, oldest first.
    pub fn drain_outbox(&mut self) -> Vec<Submission> {
        std::mem::take(&mut self.outbox)
    }

    fn confirm(&mut self) {
        match self.pending.take() {
            Some(Pending::Send) => {
                let draft = std::mem::take(&mut self.views.composer);
                self.outbox.push(Submission::Send {
                    to: recipients(&draft.to),
                    subject: draft.subject,
                    body: draft.body,
                });
                self.composing = false;
            }
            Some(Pending::Delete(threads)) => self.outbox.push(Submission::Delete { threads }),
            None => {}
        }
    }
}

/// The composer's comma-separated `to` field as addresses. Display names are
/// left to the MIME layer; empty entries are dropped.
fn recipients(to: &str) -> Vec<Address> {
    to.split(',')
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .map(|email| Address {
            name: None,
            email: email.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{Address, Event, Submission, ThreadId};

    use super::{Msg, Pending, Ui};
    use crate::views::ComposerDraft;

    fn draft(to: &str) -> Msg {
        Msg::EditDraft(ComposerDraft {
            to: to.into(),
            subject: "Hi".into(),
            body: "Hello".into(),
        })
    }

    fn addr(email: &str) -> Address {
        Address {
            name: None,
            email: email.into(),
        }
    }

    #[test]
    fn a_send_leaves_only_after_confirmation() {
        let mut ui = Ui::new();
        ui.update(Msg::ComposeNew);
        ui.update(draft("ada@example.com, , bob@example.com"));
        ui.update(Msg::Send);
        assert_eq!(ui.pending, Some(Pending::Send));
        assert!(ui.outbox.is_empty());
        ui.update(Msg::Confirm);
        assert_eq!(
            ui.drain_outbox(),
            [Submission::Send {
                to: vec![addr("ada@example.com"), addr("bob@example.com")],
                subject: "Hi".into(),
                body: "Hello".into(),
            }]
        );
        assert!(!ui.composing);
        assert_eq!(ui.views.composer, ComposerDraft::default());
        assert!(ui.outbox.is_empty());
    }

    #[test]
    fn a_cancelled_send_sends_nothing_and_keeps_the_draft() {
        let mut ui = Ui::new();
        ui.update(Msg::ComposeNew);
        ui.update(draft("ada@example.com"));
        ui.update(Msg::Send);
        ui.update(Msg::Cancel);
        ui.update(Msg::Confirm);
        assert!(ui.outbox.is_empty());
        assert_eq!(ui.views.composer.subject, "Hi");
    }

    #[test]
    fn a_send_without_recipients_or_composer_asks_nothing() {
        let mut ui = Ui::new();
        ui.update(Msg::Send);
        assert_eq!(ui.pending, None);
        ui.update(Msg::ComposeNew);
        ui.update(draft(" , "));
        ui.update(Msg::Send);
        assert_eq!(ui.pending, None);
    }

    #[test]
    fn delete_waits_for_confirmation_and_archive_does_not() {
        let mut ui = Ui::new();
        let threads = vec![ThreadId::new("t1")];
        ui.update(Msg::Delete(threads.clone()));
        ui.update(Msg::Archive(vec![ThreadId::new("t2")]));
        ui.update(Msg::Delete(Vec::new()));
        assert_eq!(
            ui.outbox,
            [Submission::Archive {
                threads: vec![ThreadId::new("t2")]
            }]
        );
        assert_eq!(ui.pending, Some(Pending::Delete(threads.clone())));
        ui.update(Msg::Confirm);
        assert_eq!(ui.outbox[1], Submission::Delete { threads });
    }

    #[test]
    fn closing_the_composer_saves_the_draft() {
        let mut ui = Ui::new();
        ui.update(Msg::ComposeNew);
        ui.update(draft("ada@example.com"));
        ui.update(Msg::CloseComposer);
        ui.update(Msg::CloseComposer);
        assert_eq!(
            ui.drain_outbox(),
            [Submission::SaveDraft {
                to: vec![addr("ada@example.com")],
                subject: "Hi".into(),
                body: "Hello".into(),
            }]
        );
        assert!(!ui.composing);
    }

    #[test]
    fn select_search_and_core_events_flow_through_one_reducer() {
        let mut ui = Ui::new();
        ui.update(Msg::Select(ThreadId::new("t1")));
        ui.update(Msg::Search("from:ada".into()));
        ui.update(Msg::Core(vec![Event::Notice {
            message: "settings\nlanguage: fr\nplan: plus".into(),
        }]));
        assert_eq!(ui.query, "from:ada");
        assert_eq!(ui.views.settings.language, "fr");
        assert_eq!(
            ui.drain_outbox(),
            [
                Submission::OpenThread {
                    thread: ThreadId::new("t1")
                },
                Submission::Search {
                    query: "from:ada".into()
                },
            ]
        );
    }
}
