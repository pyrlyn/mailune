//! Log fields with secret values removed.
//!
//! A shared telemetry package is not in this repo, so this crate only rewrites
//! fields. It does not print, and it does not export traces.

/// One name/value pair on a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogField<'a> {
    /// Field name, such as `account` or `token`.
    pub name: &'a str,
    /// Field value. A secret value is replaced before the line is returned.
    pub value: &'a str,
}

/// Builds one log line. A field named like a credential, or whose value
/// contains one of `secrets`, is written as `redacted`.
///
/// Empty entries in `secrets` are ignored so they cannot match every value.
#[must_use]
pub fn redact_fields(fields: &[LogField<'_>], secrets: &[&str]) -> String {
    let mut line = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            line.push(' ');
        }
        line.push_str(field.name);
        line.push('=');
        if should_redact(field, secrets) {
            line.push_str("redacted");
        } else {
            line.push_str(field.value);
        }
    }
    line
}

fn should_redact(field: &LogField<'_>, secrets: &[&str]) -> bool {
    const NAMES: &[&str] = &["token", "password", "secret", "authorization", "db_key"];
    if NAMES
        .iter()
        .any(|name| field.name.eq_ignore_ascii_case(name))
    {
        return true;
    }
    secrets
        .iter()
        .any(|secret| !secret.is_empty() && field.value.contains(secret))
}

#[cfg(test)]
mod tests {
    use super::{LogField, redact_fields};

    #[test]
    fn a_field_that_contains_a_token_is_redacted() {
        let token = "s3cret-token-9f";
        let fields = [
            LogField {
                name: "account",
                value: "ada",
            },
            LogField {
                name: "token",
                value: token,
            },
            LogField {
                name: "message",
                value: "header had s3cret-token-9f",
            },
        ];
        let line = redact_fields(&fields, &[token]);
        assert!(!line.contains(token), "{line}");
        assert!(line.contains("account=ada"), "{line}");
        assert!(line.contains("token=redacted"), "{line}");
        assert!(line.contains("message=redacted"), "{line}");
    }
}
