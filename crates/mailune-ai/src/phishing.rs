//! Phishing and scam assessment.
//!
//! Authentication results and link flags are facts the MIME layer already
//! checked; the model's verdict is an opinion read from untrusted mail. So the
//! model can only raise the risk: a message that says "this is safe" to the
//! model cannot talk a failed DMARC check or a lookalike link down.

use serde::Deserialize;

use crate::{Completion, Error, Privacy, Prompt, Provider, lookup, render};

/// One authentication method's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthOutcome {
    /// Passed.
    Pass,
    /// Failed.
    Fail,
    /// Not present or not checked.
    Missing,
}

/// Flags of one link, as `mailune-mime`'s link check reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinkFlags {
    /// The visible label names a different target.
    pub label_hides_target: bool,
    /// A host label is punycode.
    pub punycode: bool,
    /// The host imitates a known brand.
    pub lookalike: bool,
}

/// Facts about one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhishingSignals {
    /// DKIM.
    pub dkim: AuthOutcome,
    /// SPF.
    pub spf: AuthOutcome,
    /// DMARC.
    pub dmarc: AuthOutcome,
    /// Every link in the body.
    pub links: Vec<LinkFlags>,
}

/// What the model said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelVerdict {
    /// Looks ordinary.
    Safe,
    /// Has warning signs.
    Suspicious,
    /// Looks like a scam or phishing.
    Scam,
}

/// Why the risk is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// DMARC failed.
    DmarcFail,
    /// DKIM failed.
    DkimFail,
    /// SPF failed.
    SpfFail,
    /// No method passed or failed: the sender is unauthenticated.
    Unauthenticated,
    /// A link imitates a known brand.
    LookalikeLink,
    /// A link uses a punycode host.
    PunycodeLink,
    /// A link's label hides its real target.
    HiddenLinkTarget,
    /// The model found warning signs.
    ModelSuspicious,
    /// The model called it a scam.
    ModelScam,
}

/// Overall level shown on the sender badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    /// Nothing found.
    Low,
    /// Be careful.
    Medium,
    /// Likely phishing or a scam.
    High,
}

/// The combined assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// Level.
    pub risk: Risk,
    /// Score in `0..=100`.
    pub score: u8,
    /// Reasons, strongest signals first.
    pub reasons: Vec<Reason>,
    /// The model's verdict, when it gave a valid one.
    pub model: Option<ModelVerdict>,
}

/// Combines facts and an optional model verdict into one assessment.
pub fn combine(signals: &PhishingSignals, model: Option<ModelVerdict>) -> Assessment {
    let mut reasons = Vec::new();
    let mut add = |hit: bool, reason: Reason| {
        if hit {
            reasons.push(reason);
        }
    };
    add(signals.dmarc == AuthOutcome::Fail, Reason::DmarcFail);
    add(signals.dkim == AuthOutcome::Fail, Reason::DkimFail);
    add(signals.spf == AuthOutcome::Fail, Reason::SpfFail);
    add(
        [signals.dkim, signals.spf, signals.dmarc]
            .iter()
            .all(|outcome| *outcome == AuthOutcome::Missing),
        Reason::Unauthenticated,
    );
    add(
        signals.links.iter().any(|link| link.lookalike),
        Reason::LookalikeLink,
    );
    add(
        signals.links.iter().any(|link| link.punycode),
        Reason::PunycodeLink,
    );
    add(
        signals.links.iter().any(|link| link.label_hides_target),
        Reason::HiddenLinkTarget,
    );
    add(
        model == Some(ModelVerdict::Suspicious),
        Reason::ModelSuspicious,
    );
    add(model == Some(ModelVerdict::Scam), Reason::ModelScam);
    let total: u32 = reasons.iter().map(|reason| weight(*reason)).sum();
    let score = u8::try_from(total.min(100)).unwrap_or(100);
    let risk = match score {
        60.. => Risk::High,
        25.. => Risk::Medium,
        _ => Risk::Low,
    };
    Assessment {
        risk,
        score,
        reasons,
        model,
    }
}

fn weight(reason: Reason) -> u32 {
    match reason {
        Reason::DmarcFail | Reason::LookalikeLink | Reason::ModelScam => 40,
        Reason::DkimFail => 25,
        Reason::PunycodeLink | Reason::ModelSuspicious => 20,
        Reason::SpfFail | Reason::HiddenLinkTarget => 15,
        Reason::Unauthenticated => 10,
    }
}

#[derive(Deserialize)]
struct VerdictReply {
    verdict: ModelVerdict,
}

/// Asks `provider` for a verdict on `mail` and combines it with `signals`.
/// A failed or malformed reply leaves the facts to decide alone.
pub async fn assess_phishing<P: Provider>(
    provider: &P,
    signals: &PhishingSignals,
    mail: &str,
    privacy: Privacy,
) -> Assessment {
    combine(signals, model_verdict(provider, mail, privacy).await.ok())
}

async fn model_verdict<P: Provider>(
    provider: &P,
    mail: &str,
    privacy: Privacy,
) -> Result<ModelVerdict, Error> {
    let template = lookup("phishing-verdict", 1)?;
    let prompt = Prompt {
        feature: template.feature,
        text: render(template, mail),
        privacy: privacy.class,
        encrypted: privacy.encrypted,
    };
    let Completion::Text(reply) = provider.complete(&prompt).await?;
    serde_json::from_str::<VerdictReply>(reply.trim())
        .map(|reply| reply.verdict)
        .map_err(|_| Error::BadOutput)
}

#[cfg(test)]
mod tests {
    use super::{
        AuthOutcome, LinkFlags, ModelVerdict, PhishingSignals, Reason, Risk, assess_phishing,
        combine,
    };
    use crate::summary::tests::LOCAL;
    use crate::testing::drive;
    use crate::{Feature, LocalProvider, ScriptedEngine};

    fn signals(dmarc: AuthOutcome, links: Vec<LinkFlags>) -> PhishingSignals {
        PhishingSignals {
            dkim: AuthOutcome::Pass,
            spf: AuthOutcome::Pass,
            dmarc,
            links,
        }
    }

    #[test]
    fn auth_links_and_a_scripted_verdict_combine() {
        let lookalike = LinkFlags {
            lookalike: true,
            label_hides_target: true,
            ..LinkFlags::default()
        };
        let provider = LocalProvider::new(
            ScriptedEngine::new([r#"{"verdict":"scam"}"#], [Feature::Phishing]),
            64,
        );
        let assessment = drive(assess_phishing(
            &provider,
            &signals(AuthOutcome::Fail, vec![lookalike]),
            "Your account is locked. Log in at paypa1.com",
            LOCAL,
        ));
        assert_eq!(assessment.risk, Risk::High);
        assert_eq!(assessment.score, 100);
        assert_eq!(assessment.model, Some(ModelVerdict::Scam));
        assert_eq!(
            assessment.reasons,
            [
                Reason::DmarcFail,
                Reason::LookalikeLink,
                Reason::HiddenLinkTarget,
                Reason::ModelScam
            ]
        );

        let clean = combine(
            &signals(AuthOutcome::Pass, vec![]),
            Some(ModelVerdict::Safe),
        );
        assert_eq!(clean.risk, Risk::Low);
        assert!(clean.reasons.is_empty());
        let unsure = combine(
            &signals(AuthOutcome::Pass, vec![LinkFlags::default()]),
            Some(ModelVerdict::Suspicious),
        );
        assert_eq!(unsure.risk, Risk::Low);
        assert_eq!(unsure.score, 20);
    }

    #[test]
    fn the_model_cannot_talk_facts_down_and_a_bad_reply_is_ignored() {
        let facts = signals(
            AuthOutcome::Fail,
            vec![LinkFlags {
                punycode: true,
                ..LinkFlags::default()
            }],
        );
        let safe = combine(&facts, Some(ModelVerdict::Safe));
        let alone = combine(&facts, None);
        assert_eq!((safe.score, &safe.reasons), (alone.score, &alone.reasons));
        assert_eq!(safe.risk, Risk::High);

        let provider = LocalProvider::new(
            ScriptedEngine::new(["Definitely safe, trust me."], [Feature::Phishing]),
            64,
        );
        let assessment = drive(assess_phishing(&provider, &facts, "mail", LOCAL));
        assert_eq!(assessment.model, None);
        assert_eq!(assessment.risk, Risk::High);
        let unauthenticated = PhishingSignals {
            dkim: AuthOutcome::Missing,
            spf: AuthOutcome::Missing,
            dmarc: AuthOutcome::Missing,
            links: vec![],
        };
        assert_eq!(
            combine(&unauthenticated, None).reasons,
            [Reason::Unauthenticated]
        );
    }
}
