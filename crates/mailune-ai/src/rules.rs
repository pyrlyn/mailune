//! Natural-language rules.
//!
//! A sentence becomes a typed rule the app can preview. The rule starts
//! disabled, so a match list is not an action. Sending is not a rule action.

use crate::Error;

/// What a rule would do once the person enables it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleAction {
    /// Move matching mail out of the inbox.
    Archive,
    /// Flag matching mail.
    Star,
    /// File matching mail in a named mailbox.
    Move {
        /// Destination mailbox name.
        mailbox: String,
    },
}

/// A parsed rule. It does not run until [`Rule::enable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// What the rule would do.
    pub action: RuleAction,
    /// Sender address the rule matches.
    pub from: String,
    enabled: bool,
}

impl Rule {
    /// Rules start off so a preview cannot apply them.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Lets the app apply the rule after the person has seen the preview.
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

/// One message a preview can consider. The body is not an input.
#[derive(Debug, Clone, Copy)]
pub struct RuleMessage<'a> {
    /// Stable id the preview returns.
    pub id: &'a str,
    /// `From`, display name and address.
    pub from: &'a str,
}

/// Parses `sentence` into a disabled rule.
///
/// # Errors
///
/// [`Error::RuleCannotSend`] when the sentence asks to send.
/// [`Error::NotARule`] when the sentence is not archive, star, or move-from.
pub fn parse_rule(sentence: &str) -> Result<Rule, Error> {
    let words: Vec<&str> = sentence.split_whitespace().collect();
    let Some(head) = words.first() else {
        return Err(Error::NotARule);
    };
    let head = head.to_ascii_lowercase();
    if head == "send" {
        return Err(Error::RuleCannotSend);
    }
    let action = match head.as_str() {
        "archive" => RuleAction::Archive,
        "star" => RuleAction::Star,
        "move" => {
            let mailbox = word_after(&words, "to").ok_or(Error::NotARule)?;
            RuleAction::Move {
                mailbox: (*mailbox).to_owned(),
            }
        }
        _ => return Err(Error::NotARule),
    };
    let email = word_after(&words, "from").ok_or(Error::NotARule)?;
    if !email.contains('@') || email.contains(' ') {
        return Err(Error::NotARule);
    }
    Ok(Rule {
        action,
        from: email.to_ascii_lowercase(),
        enabled: false,
    })
}

/// Ids of messages the rule would match. Disabled rules still preview.
#[must_use]
pub fn preview<'a>(rule: &Rule, messages: &[RuleMessage<'a>]) -> Vec<&'a str> {
    messages
        .iter()
        .filter(|message| sender_is(message.from, &rule.from))
        .map(|message| message.id)
        .collect()
}

fn word_after<'a>(words: &[&'a str], marker: &str) -> Option<&'a str> {
    let index = words
        .iter()
        .position(|word| word.eq_ignore_ascii_case(marker))?;
    words.get(index.saturating_add(1)).copied()
}

fn sender_is(from: &str, expected: &str) -> bool {
    let address = from
        .rsplit(['<', ' '])
        .next()
        .unwrap_or(from)
        .trim()
        .trim_end_matches('>');
    address.eq_ignore_ascii_case(expected)
}

#[cfg(test)]
mod tests {
    use super::{RuleAction, RuleMessage, parse_rule, preview};
    use crate::Error;

    #[test]
    fn a_sentence_previews_matches_before_it_is_enabled() {
        let mut rule = parse_rule("Archive mail from Ada@acme.io").unwrap();
        assert!(!rule.is_enabled());
        assert_eq!(rule.action, RuleAction::Archive);
        assert_eq!(rule.from, "ada@acme.io");
        let messages = [
            RuleMessage {
                id: "1",
                from: "Ada Lovelace <ada@acme.io>",
            },
            RuleMessage {
                id: "2",
                from: "Grace <grace@acme.io>",
            },
        ];
        assert_eq!(preview(&rule, &messages), vec!["1"]);
        rule.enable();
        assert!(rule.is_enabled());
        assert_eq!(preview(&rule, &messages), vec!["1"]);

        let starred = parse_rule("star messages from boss@acme.io").unwrap();
        assert_eq!(starred.action, RuleAction::Star);
        let moved = parse_rule("move mail from lists@acme.io to News").unwrap();
        assert_eq!(
            moved.action,
            RuleAction::Move {
                mailbox: "News".into()
            }
        );
        assert!(matches!(
            parse_rule("send mail from ada@acme.io"),
            Err(Error::RuleCannotSend)
        ));
        assert!(matches!(parse_rule("hello"), Err(Error::NotARule)));
    }
}
