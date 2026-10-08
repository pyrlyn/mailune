//! Deterministic mailbox for evaluations.
//!
//! The same seed always builds the same threads. Ids include the seed so two
//! seeds cannot collide. Nothing here opens a socket; the phishing address is
//! a lookalike on `.example` and is never resolved.

use mailune_protocol::{Address, Envelope, Flags, MessageId, ThreadId, TransportSecurity};

/// Which kind of thread the generator built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadKind {
    /// A mailing list.
    Newsletter,
    /// A purchase receipt.
    Receipt,
    /// A short conversation in two languages.
    Conversation,
    /// A message whose subject asks the person to verify an account.
    Phishing,
}

/// One generated message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticMessage {
    /// Headers and flags.
    pub envelope: Envelope,
    /// Plain body. Not sent anywhere.
    pub body: String,
}

/// One generated thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticThread {
    /// Why this thread is in the corpus.
    pub kind: ThreadKind,
    /// Messages in chronological order.
    pub messages: Vec<SyntheticMessage>,
}

/// Builds the corpus for `seed`.
pub fn mailbox(seed: u64) -> Vec<SyntheticThread> {
    vec![
        thread(
            ThreadKind::Newsletter,
            vec![spec(
                seed,
                "newsletter",
                ("Notes", "news@example.com"),
                "Weekly notes",
                "Here are this week's notes.",
                true,
                0,
            )],
        ),
        thread(
            ThreadKind::Receipt,
            vec![spec(
                seed,
                "receipt",
                ("Shop", "billing@shop.example"),
                "Your receipt",
                "Order 1001 is paid.",
                true,
                1,
            )],
        ),
        thread(
            ThreadKind::Conversation,
            vec![
                spec(
                    seed,
                    "meet",
                    ("Sam", "sam@example.com"),
                    "Can we meet on Friday?",
                    "Can we meet on Friday?",
                    true,
                    0,
                ),
                spec(
                    seed,
                    "meet-reply",
                    ("Sam", "sam@example.com"),
                    "Re: Can we meet on Friday?",
                    "Oui, vendredi me convient.",
                    false,
                    0,
                ),
            ],
        ),
        thread(
            ThreadKind::Phishing,
            vec![spec(
                seed,
                "phish",
                ("Support", "support@paypa1.example"),
                "Urgent: verify your account now",
                "Verify your account or it will be closed.",
                false,
                0,
            )],
        ),
    ]
}

struct Spec {
    id: String,
    thread: String,
    from_email: String,
    from_name: String,
    subject: String,
    body: String,
    seen: bool,
    attachments: u32,
}

fn spec(
    seed: u64,
    slug: &str,
    from: (&str, &str),
    subject: &str,
    body: &str,
    seen: bool,
    attachments: u32,
) -> Spec {
    Spec {
        id: format!("{seed}-{slug}"),
        thread: format!("{seed}-{}", thread_slug(slug)),
        from_email: from.1.into(),
        from_name: from.0.into(),
        subject: subject.into(),
        body: body.into(),
        seen,
        attachments,
    }
}

fn thread_slug(slug: &str) -> &str {
    slug.strip_suffix("-reply").unwrap_or(slug)
}

fn thread(kind: ThreadKind, specs: Vec<Spec>) -> SyntheticThread {
    SyntheticThread {
        kind,
        messages: specs.into_iter().map(message).collect(),
    }
}

fn message(spec: Spec) -> SyntheticMessage {
    let snippet = spec.body.chars().take(80).collect();
    let envelope = Envelope {
        id: MessageId::new(spec.id),
        thread: ThreadId::new(spec.thread),
        from: Address {
            name: Some(spec.from_name),
            email: spec.from_email,
        },
        to: vec![Address {
            name: None,
            email: "me@mailune.local".into(),
        }],
        cc: Vec::new(),
        subject: spec.subject,
        stamp: "t0".into(),
        snippet,
        flags: Flags {
            seen: spec.seen,
            flagged: false,
            draft: false,
            answered: false,
            deleted: false,
            keywords: Vec::new(),
        },
        attachment_count: spec.attachments,
        transport: TransportSecurity::Clear,
    };
    SyntheticMessage {
        envelope,
        body: spec.body,
    }
}

#[cfg(test)]
mod tests {
    use super::{ThreadKind, mailbox};

    #[test]
    fn the_same_seed_is_the_same_mailbox() {
        assert_eq!(mailbox(7), mailbox(7));
        assert_ne!(mailbox(7), mailbox(8));
    }

    #[test]
    fn the_corpus_has_the_requested_threads() {
        let threads = mailbox(1);
        assert_eq!(
            threads.iter().map(|item| item.kind).collect::<Vec<_>>(),
            vec![
                ThreadKind::Newsletter,
                ThreadKind::Receipt,
                ThreadKind::Conversation,
                ThreadKind::Phishing,
            ]
        );
        let conversation = &threads[2];
        assert_eq!(conversation.messages.len(), 2);
        assert!(conversation.messages[0].body.contains("Friday"));
        assert!(conversation.messages[1].body.contains("vendredi"));
        assert_eq!(
            conversation.messages[0].envelope.thread,
            conversation.messages[1].envelope.thread
        );
        let phishing = &threads[3].messages[0];
        assert!(phishing.envelope.subject.contains("verify your account"));
        assert!(phishing.envelope.from.email.contains("paypa1"));
        assert!(!phishing.envelope.flags.seen);
        assert_eq!(threads[1].messages[0].envelope.attachment_count, 1);
    }
}
