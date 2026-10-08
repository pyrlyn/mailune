//! Strips quoted lines and a trailing signature before a cloud-bound string.
//!
//! The result is text for a caller to send later. This function does not send
//! it, and it does not open a socket.

/// Removes quoted lines and a trailing signature.
///
/// A signature starts at the last line that is `--` or `-- ` (the RFC 3676
/// delimiter, which ends with a space). Quoted lines start with `>`.
pub fn redact_for_cloud(body: &str) -> String {
    let mut lines: Vec<&str> = body.lines().collect();
    if let Some(index) = lines.iter().rposition(|line| is_signature_delimiter(line)) {
        lines.truncate(index);
    }
    lines.retain(|line| !line.trim_start().starts_with('>'));
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn is_signature_delimiter(line: &str) -> bool {
    // `trim_end` folds "-- " and "--" into the same check.
    line.trim_end() == "--"
}

#[cfg(test)]
mod tests {
    use super::redact_for_cloud;

    #[test]
    fn a_quoted_secret_and_the_signature_are_gone() {
        let body = "Hello\n\n> swordfish-secret\n\n-- \nAda Lovelace\nsecret-signature\n";
        let redacted = redact_for_cloud(body);
        assert!(!redacted.contains("swordfish"));
        assert!(!redacted.contains("secret-signature"));
        assert!(!redacted.contains("Ada"));
        assert_eq!(redacted, "Hello");
    }
}
