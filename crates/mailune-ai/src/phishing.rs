//! Phishing assessment from auth results, link flags, and a model verdict.
//!
//! The three inputs are combined here. Auth results and link flags are values
//! the caller already computed; this module does not look them up on the network.

use crate::{Error, LocalEngine, lookup, render};

/// One authentication result the caller already has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthValue {
    /// The check succeeded.
    Pass,
    /// The check failed.
    Fail,
    /// The check was absent.
    None,
}

/// SPF, DKIM, and DMARC as plain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthFacts {
    /// SPF result.
    pub spf: AuthValue,
    /// DKIM result.
    pub dkim: AuthValue,
    /// DMARC result.
    pub dmarc: AuthValue,
}

/// A link property the caller already flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkFlag {
    /// The host looks like a well-known name but is not that name.
    Lookalike,
    /// The visible label and the target disagree.
    Mismatched,
}

/// How urgently the person should treat the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    /// Nothing in the three inputs raised a concern.
    Low,
    /// A weak signal. The message is not marked as a scam by itself.
    Review,
    /// A strong signal: a failed DMARC, a lookalike, or a scam verdict.
    High,
}

/// The combined assessment. Reasons name the signals, not the mail text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// Highest risk among the signals.
    pub risk: Risk,
    /// Why `risk` was chosen, strongest first.
    pub reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Benign,
    Suspicious,
    Scam,
}

/// Combines `auth`, `links`, and a scripted verdict for `message`.
///
/// # Errors
///
/// [`Error::UnknownPrompt`] when the assessment template is missing.
/// [`Error::Unscripted`] when the engine has no verdict.
/// [`Error::BadVerdict`] when the JSON is not a known verdict.
pub fn assess_message(
    engine: &impl LocalEngine,
    message: &str,
    auth: &AuthFacts,
    links: &[LinkFlag],
) -> Result<Assessment, Error> {
    let verdict = model_verdict(engine, message)?;
    Ok(combine(auth, links, verdict))
}

fn model_verdict(engine: &impl LocalEngine, message: &str) -> Result<Verdict, Error> {
    let template = lookup("assess-phishing", 1)?;
    let value = engine.structured(&render(template, message))?;
    match value.get("verdict").and_then(serde_json::Value::as_str) {
        Some("benign") => Ok(Verdict::Benign),
        Some("suspicious") => Ok(Verdict::Suspicious),
        Some("scam") => Ok(Verdict::Scam),
        _ => Err(Error::BadVerdict),
    }
}

fn combine(auth: &AuthFacts, links: &[LinkFlag], verdict: Verdict) -> Assessment {
    let mut reasons = Vec::new();
    // DMARC, a lookalike host, or an explicit scam verdict are enough on their own.
    if auth.dmarc == AuthValue::Fail {
        reasons.push("dmarc failed");
    }
    if links.contains(&LinkFlag::Lookalike) {
        reasons.push("lookalike link");
    }
    if verdict == Verdict::Scam {
        reasons.push("model verdict is scam");
    }
    if !reasons.is_empty() {
        return Assessment {
            risk: Risk::High,
            reasons,
        };
    }
    if auth.spf == AuthValue::Fail {
        reasons.push("spf failed");
    }
    if auth.dkim == AuthValue::Fail {
        reasons.push("dkim failed");
    }
    if links.contains(&LinkFlag::Mismatched) {
        reasons.push("mismatched link");
    }
    if verdict == Verdict::Suspicious {
        reasons.push("model verdict is suspicious");
    }
    if reasons.is_empty() {
        Assessment {
            risk: Risk::Low,
            reasons,
        }
    } else {
        Assessment {
            risk: Risk::Review,
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AuthFacts, AuthValue, LinkFlag, Risk, assess_message};
    use crate::{Error, ScriptedEngine, lookup, render};

    fn passing() -> AuthFacts {
        AuthFacts {
            spf: AuthValue::Pass,
            dkim: AuthValue::Pass,
            dmarc: AuthValue::Pass,
        }
    }

    fn script(engine: &mut ScriptedEngine, message: &str, verdict: &str) {
        let template = lookup("assess-phishing", 1).unwrap();
        engine.script_json(render(template, message), json!({ "verdict": verdict }));
    }

    #[test]
    fn auth_links_and_a_scripted_verdict_become_one_assessment() {
        let mut engine = ScriptedEngine::new();
        script(&mut engine, "hello", "benign");
        script(&mut engine, "invoice", "benign");
        script(&mut engine, "pay", "scam");
        script(&mut engine, "odd", "suspicious");

        let clear = assess_message(&engine, "hello", &passing(), &[]).unwrap();
        assert_eq!(clear.risk, Risk::Low);
        assert!(clear.reasons.is_empty());

        let mut dmarc = passing();
        dmarc.dmarc = AuthValue::Fail;
        let failed = assess_message(&engine, "invoice", &dmarc, &[]).unwrap();
        assert_eq!(failed.risk, Risk::High);
        assert_eq!(failed.reasons, vec!["dmarc failed"]);

        let lookalike =
            assess_message(&engine, "invoice", &passing(), &[LinkFlag::Lookalike]).unwrap();
        assert_eq!(lookalike.risk, Risk::High);
        assert!(lookalike.reasons.contains(&"lookalike link"));

        let scam = assess_message(&engine, "pay", &passing(), &[]).unwrap();
        assert_eq!(scam.risk, Risk::High);
        assert!(scam.reasons.contains(&"model verdict is scam"));

        let mut spf = passing();
        spf.spf = AuthValue::Fail;
        let review = assess_message(&engine, "odd", &spf, &[LinkFlag::Mismatched]).unwrap();
        assert_eq!(review.risk, Risk::Review);
        assert!(review.reasons.contains(&"spf failed"));
        assert!(review.reasons.contains(&"mismatched link"));
        assert!(review.reasons.contains(&"model verdict is suspicious"));

        assert!(matches!(
            assess_message(&engine, "missing", &passing(), &[]),
            Err(Error::Unscripted)
        ));
    }
}
