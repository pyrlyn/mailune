//! Priority and needs-reply from flags and headers.
//!
//! A star and an unanswered message addressed to the person raise the score.
//! Bulk and machine senders do not need a reply. No model is called.

use mailune_protocol::{Category, Flags};

use crate::triage::automated;

/// Flags and headers the score can see. The body is not an input.
#[derive(Debug, Clone, Copy)]
pub struct PriorityInput<'a> {
    /// Standard flags and keywords.
    pub flags: &'a Flags,
    /// `From`.
    pub from: &'a str,
    /// `Subject`.
    pub subject: &'a str,
    /// The person's address is on `To`. A `Cc` does not count.
    pub to_me: bool,
    /// Tab already chosen by the triage heuristic.
    pub category: Category,
    /// `Auto-Submitted`, when the message has one.
    pub auto_submitted: Option<&'a str>,
}

/// A score in `0..=100` and whether the person still owes a reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priority {
    /// Higher means look at this message first.
    pub score: u8,
    /// True when a person wrote to the account and nobody has answered.
    pub needs_reply: bool,
}

/// Scores `input`. The weights put a star above unread, and a needed reply
/// above the tab, because those are choices the person or the sender already made.
pub fn assess(input: &PriorityInput<'_>) -> Priority {
    let needs_reply = needs_reply(input);
    let mut total: u16 = 0;
    if input.flags.flagged {
        total += 30;
    }
    if !input.flags.seen {
        total += 20;
    }
    if important(input.flags) {
        total += 15;
    }
    if input.category == Category::Primary {
        total += 15;
    }
    if input.to_me {
        total += 10;
    }
    if needs_reply {
        total += 20;
    }
    if input.subject.contains('?') {
        total += 5;
    }
    let score = u8::try_from(total.min(100)).unwrap_or(100);
    Priority { score, needs_reply }
}

fn needs_reply(input: &PriorityInput<'_>) -> bool {
    if input.flags.answered || input.flags.draft || input.flags.deleted {
        return false;
    }
    if !input.to_me || input.category != Category::Primary {
        return false;
    }
    if automated(input.auto_submitted) || machine_sender(input.from) {
        return false;
    }
    true
}

fn important(flags: &Flags) -> bool {
    flags
        .keywords
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case("important"))
}

fn machine_sender(from: &str) -> bool {
    const NAMES: &[&str] = &[
        "noreply",
        "no-reply",
        "no_reply",
        "notifications",
        "mailer-daemon",
        "donotreply",
    ];
    let local = local_part(from);
    NAMES.iter().any(|name| {
        local == *name
            || local
                .strip_prefix(name)
                .is_some_and(|rest| rest.starts_with(['+', '.', '-']))
    })
}

fn local_part(from: &str) -> String {
    let address = from
        .rsplit(['<', ' '])
        .next()
        .unwrap_or(from)
        .trim()
        .trim_end_matches('>');
    let local = address.split_once('@').map_or(address, |(local, _)| local);
    local.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{Category, Flags};

    use super::{PriorityInput, assess};

    fn flags(answered: bool, flagged: bool) -> Flags {
        Flags {
            seen: false,
            flagged,
            draft: false,
            answered,
            deleted: false,
            keywords: vec!["important".into()],
        }
    }

    fn input<'a>(
        flags: &'a Flags,
        from: &'a str,
        category: Category,
        to_me: bool,
    ) -> PriorityInput<'a> {
        PriorityInput {
            flags,
            from,
            subject: "Can you look?",
            to_me,
            category,
            auto_submitted: None,
        }
    }

    #[test]
    fn a_starred_unanswered_message_needs_a_reply() {
        let flags = flags(false, true);
        let priority = assess(&input(&flags, "Ada <ada@acme.io>", Category::Primary, true));
        assert!(priority.needs_reply);
        assert_eq!(priority.score, 100);
    }

    #[test]
    fn answered_bulk_and_machine_mail_do_not_need_a_reply() {
        let answered = flags(true, false);
        let priority = assess(&input(
            &answered,
            "Ada <ada@acme.io>",
            Category::Primary,
            true,
        ));
        assert!(!priority.needs_reply);
        assert!(priority.score < 100);

        let open = flags(false, false);
        let promo = assess(&input(
            &open,
            "Shop <deals@shop.example>",
            Category::Promotions,
            true,
        ));
        assert!(!promo.needs_reply);

        let robot = assess(&input(
            &open,
            "Robot <noreply@shop.example>",
            Category::Primary,
            true,
        ));
        assert!(!robot.needs_reply);

        let mut away = input(&open, "Ada <ada@acme.io>", Category::Primary, true);
        away.auto_submitted = Some("auto-replied");
        assert!(!assess(&away).needs_reply);

        let mut draft = flags(false, false);
        draft.draft = true;
        assert!(!assess(&input(&draft, "Ada <ada@acme.io>", Category::Primary, true)).needs_reply);
    }
}
