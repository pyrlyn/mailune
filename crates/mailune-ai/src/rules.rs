//! Natural-language rules: a sentence becomes a typed rule.
//!
//! The model only fills a closed JSON shape. Actions are the ones a rule may
//! run without asking: no send, forward or delete. A proposed rule is off
//! until the app has shown a preview of the messages it would match and the
//! person turns it on with that preview in hand.

use mailune_protocol::{Category, MessageId};
use serde::Deserialize;

use crate::{Completion, Error, Privacy, Prompt, Provider, lookup, render};

/// Longest text a condition may hold. Longer values are model noise.
const MAX_VALUE: usize = 200;

/// One test against a message. All conditions of a rule must hold.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum Condition {
    /// `From` contains this text, case-insensitive (an address or a domain).
    From(String),
    /// `Subject` contains this text, case-insensitive.
    Subject(String),
    /// The triage tab is this one.
    Category(Category),
}

/// What a rule does. Send, forward and delete are absent on purpose.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum RuleAction {
    /// Archive the message.
    Archive,
    /// Mark it read.
    MarkRead,
    /// Star it.
    Star,
    /// Add a label.
    Label(String),
}

/// A rule the model proposed. It is off.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedRule {
    /// Tests, all of which must hold.
    pub when: Vec<Condition>,
    /// What to do.
    pub then: RuleAction,
}

/// The fields a rule reads.
#[derive(Debug, Clone, Copy)]
pub struct RuleMessage<'a> {
    /// Message id.
    pub id: &'a MessageId,
    /// `From`.
    pub from: &'a str,
    /// `Subject`.
    pub subject: &'a str,
    /// Triage tab.
    pub category: Category,
}

/// Messages a proposed rule would match today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulePreview {
    rule: ProposedRule,
    /// Matching messages, in the order given.
    pub matches: Vec<MessageId>,
}

/// A rule the person turned on after seeing its preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnabledRule {
    /// Tests.
    pub when: Vec<Condition>,
    /// Action.
    pub then: RuleAction,
}

impl ProposedRule {
    /// Whether `message` meets every condition.
    pub fn matches(&self, message: &RuleMessage<'_>) -> bool {
        self.when.iter().all(|condition| match condition {
            Condition::From(text) => contains(message.from, text),
            Condition::Subject(text) => contains(message.subject, text),
            Condition::Category(category) => message.category == *category,
        })
    }

    /// Lists the messages this rule would match.
    pub fn preview(&self, messages: &[RuleMessage<'_>]) -> RulePreview {
        RulePreview {
            rule: self.clone(),
            matches: messages
                .iter()
                .filter(|message| self.matches(message))
                .map(|message| message.id.clone())
                .collect(),
        }
    }

    fn validate(self) -> Result<Self, Error> {
        let bad_text = |text: &String| text.trim().is_empty() || text.len() > MAX_VALUE;
        let bad_condition = self.when.iter().any(|condition| match condition {
            Condition::From(text) | Condition::Subject(text) => bad_text(text),
            Condition::Category(_) => false,
        });
        let bad_action = matches!(&self.then, RuleAction::Label(label) if bad_text(label));
        // A rule with no condition would act on every message.
        if self.when.is_empty() || bad_condition || bad_action {
            return Err(Error::BadOutput);
        }
        Ok(self)
    }
}

impl RulePreview {
    /// Turns the rule on. Only a preview can do this, so the person has seen
    /// what it matches first.
    pub fn enable(self) -> EnabledRule {
        EnabledRule {
            when: self.rule.when,
            then: self.rule.then,
        }
    }
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack
        .to_lowercase()
        .contains(&needle.trim().to_lowercase())
}

/// Asks `provider` to turn `sentence` into a rule.
///
/// # Errors
///
/// [`Error::BadOutput`] when the reply is not a valid rule, including one
/// with no condition or an action outside the closed set. Provider errors as is.
pub async fn rule_from_sentence<P: Provider>(
    provider: &P,
    sentence: &str,
    privacy: Privacy,
) -> Result<ProposedRule, Error> {
    let template = lookup("rule-from-sentence", 1)?;
    let prompt = Prompt {
        feature: template.feature,
        text: render(template, sentence),
        privacy: privacy.class,
        encrypted: privacy.encrypted,
    };
    let Completion::Text(reply) = provider.complete(&prompt).await?;
    serde_json::from_str::<ProposedRule>(reply.trim())
        .map_err(|_| Error::BadOutput)?
        .validate()
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{Category, MessageId};

    use super::{Condition, RuleAction, RuleMessage, rule_from_sentence};
    use crate::summary::tests::LOCAL;
    use crate::testing::drive;
    use crate::{Error, Feature, LocalProvider, ScriptedEngine};

    #[test]
    fn a_sentence_becomes_a_rule_and_a_preview_comes_before_enabling() {
        let reply = r#"{"when":[{"field":"from","value":"@shop.example"},
            {"field":"category","value":"promotions"}],
            "then":{"action":"label","value":"Deals"}}"#;
        let provider = LocalProvider::new(ScriptedEngine::new([reply], [Feature::Rules]), 128);
        let rule = drive(rule_from_sentence(
            &provider,
            "Label promos from shop.example as Deals",
            LOCAL,
        ))
        .unwrap();
        assert_eq!(rule.then, RuleAction::Label("Deals".into()));
        assert!(
            provider.engine().seen()[0]
                .prompt
                .contains("shop.example as Deals")
        );

        let ids = [
            MessageId::new("m1"),
            MessageId::new("m2"),
            MessageId::new("m3"),
        ];
        let messages = [
            RuleMessage {
                id: &ids[0],
                from: "Shop <News@Shop.example>",
                subject: "Sale",
                category: Category::Promotions,
            },
            RuleMessage {
                id: &ids[1],
                from: "Shop <billing@shop.example>",
                subject: "Receipt",
                category: Category::Updates,
            },
            RuleMessage {
                id: &ids[2],
                from: "Ana <ana@acme.io>",
                subject: "Sale",
                category: Category::Promotions,
            },
        ];
        let preview = rule.preview(&messages);
        assert_eq!(preview.matches, [MessageId::new("m1")]);
        let enabled = preview.enable();
        assert_eq!(enabled.when[0], Condition::From("@shop.example".into()));
    }

    #[test]
    fn send_delete_empty_and_extra_fields_are_not_rules() {
        for reply in [
            r#"{"when":[{"field":"from","value":"a"}],"then":{"action":"send","value":"x@y"}}"#,
            r#"{"when":[{"field":"from","value":"a"}],"then":{"action":"delete"}}"#,
            r#"{"when":[],"then":{"action":"archive"}}"#,
            r#"{"when":[{"field":"from","value":"  "}],"then":{"action":"archive"}}"#,
            r#"{"when":[{"field":"from","value":"a"}],"then":{"action":"archive"},"enabled":true}"#,
            "Sure! Here is your rule.",
        ] {
            let provider = LocalProvider::new(ScriptedEngine::new([reply], [Feature::Rules]), 64);
            assert!(
                matches!(
                    drive(rule_from_sentence(&provider, "x", LOCAL)),
                    Err(Error::BadOutput)
                ),
                "{reply}"
            );
        }
    }
}
