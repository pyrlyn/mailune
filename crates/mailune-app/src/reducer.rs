//! Pure view-model step.
//!
//! A shell renders [`Screen`] and sends [`Input`]. [`reduce`] returns the next
//! screen and leaves the previous one alone, so every shell can show the same
//! state. Nothing here draws, stores, or opens a socket.

use mailune_protocol::Event;

use crate::views::{Views, is_view_notice};

/// What a shell renders. [`Views`] is the mailbox; `banner` is a notice that
/// is not a view update.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Screen {
    /// Thread list, open thread, composer, and settings.
    pub views: Views,
    /// A notice [`Views::fold`] does not apply. `None` when there is nothing to show.
    pub banner: Option<String>,
}

/// One step into [`reduce`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// A fact from the core.
    Event(Event),
    /// The person closed the banner. The mailbox views stay.
    DismissBanner,
}

/// Next screen for `input`.
///
/// A banner is kept on the screen because [`Views::fold`] ignores it: a
/// conflict sentence must not wipe a draft. Dismissing the banner does not
/// fold an event.
pub fn reduce(screen: &Screen, input: &Input) -> Screen {
    match input {
        Input::DismissBanner => Screen {
            views: screen.views.clone(),
            banner: None,
        },
        Input::Event(event) => apply_event(screen, event),
    }
}

fn apply_event(screen: &Screen, event: &Event) -> Screen {
    let mut views = screen.views.clone();
    views.fold(std::slice::from_ref(event));
    let banner = match event {
        Event::Notice { message } if !is_view_notice(message) && !message.is_empty() => {
            Some(message.clone())
        }
        _ => screen.banner.clone(),
    };
    Screen { views, banner }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{AccountId, Address, Category, Event, MailboxId, ThreadId, ThreadRow};

    use super::{Input, Screen, reduce};

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

    fn step(screen: &Screen, input: &Input) -> Screen {
        let before = screen.clone();
        let next = reduce(screen, input);
        assert_eq!(screen, &before);
        next
    }

    #[test]
    fn the_same_steps_reach_the_same_screen() {
        let inputs = [
            Input::Event(Event::Snapshot {
                threads: vec![row("t1", true), row("t2", false)],
            }),
            Input::Event(Event::Notice {
                message: "open t1".into(),
            }),
            Input::Event(Event::Notice {
                message: "draft\nto: ada@example.com\nsubject: Hi\nbody: Hello there".into(),
            }),
            Input::Event(Event::Notice {
                message: "settings\nlanguage: fr\nplan: plus".into(),
            }),
        ];
        let mut first = Screen::default();
        let mut second = Screen::default();
        for input in &inputs {
            first = step(&first, input);
            second = step(&second, input);
        }
        assert_eq!(first, second);
        assert!(first.banner.is_none());
        assert_eq!(first.views.list.rows.len(), 2);
        let open = first.views.open.row.as_ref().unwrap();
        assert_eq!(open.id, ThreadId::new("t1"));
        assert!(open.unread);
        assert_eq!(first.views.composer.to, "ada@example.com");
        assert_eq!(first.views.composer.subject, "Hi");
        assert_eq!(first.views.composer.body, "Hello there");
        assert_eq!(first.views.settings.language, "fr");
        assert_eq!(first.views.settings.plan, "plus");
    }

    #[test]
    fn a_banner_stays_until_dismissed_and_does_not_clear_the_draft() {
        let mut screen = Screen::default();
        screen = step(
            &screen,
            &Input::Event(Event::Notice {
                message: "draft\nto: ada@example.com\nsubject: Hi\nbody: Hello".into(),
            }),
        );
        screen = step(
            &screen,
            &Input::Event(Event::Notice {
                message: "signed out".into(),
            }),
        );
        assert_eq!(screen.banner.as_deref(), Some("signed out"));
        assert_eq!(screen.views.composer.subject, "Hi");
        assert_eq!(screen.views.settings.language, "en");
        screen = step(
            &screen,
            &Input::Event(Event::Snapshot {
                threads: vec![row("t1", false)],
            }),
        );
        assert_eq!(screen.banner.as_deref(), Some("signed out"));
        assert_eq!(screen.views.list.rows.len(), 1);
        screen = step(&screen, &Input::DismissBanner);
        assert!(screen.banner.is_none());
        assert_eq!(screen.views.composer.subject, "Hi");
        assert_eq!(screen.views.list.rows.len(), 1);
    }
}
