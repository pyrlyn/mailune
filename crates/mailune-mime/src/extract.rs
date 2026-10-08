//! Pull an OTP, a tracking number, an invoice total, or a flight code
//! from plain text.
//!
//! Each rule is a label next to a token shape. Nothing is sent to a model.

/// Which fact a span is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactKind {
    /// A one-time code.
    Otp,
    /// A parcel tracking id.
    Tracking,
    /// The amount a bill says is due.
    InvoiceTotal,
    /// An airline flight designator.
    Flight,
}

/// One extracted fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    /// Which rule matched.
    pub kind: FactKind,
    /// The token, as the text wrote it.
    pub value: String,
}

/// Facts in label order. A kind that is absent is omitted.
pub fn extract_facts(text: &str) -> Vec<Fact> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    if let Some(value) = otp(&lines) {
        found.push(Fact {
            kind: FactKind::Otp,
            value,
        });
    }
    if let Some(value) = tracking(text, &lines) {
        found.push(Fact {
            kind: FactKind::Tracking,
            value,
        });
    }
    if let Some(value) = invoice(&lines) {
        found.push(Fact {
            kind: FactKind::InvoiceTotal,
            value,
        });
    }
    if let Some(value) = flight(&lines) {
        found.push(Fact {
            kind: FactKind::Flight,
            value,
        });
    }
    found
}

fn otp(lines: &[&str]) -> Option<String> {
    for (index, line) in lines.iter().enumerate() {
        if !otp_label(line) {
            continue;
        }
        if let Some(code) = digit_token(line, 4, 8) {
            return Some(code.to_owned());
        }
        if let Some(code) = lines
            .get(index + 1)
            .and_then(|next| digit_token(next, 4, 8))
        {
            return Some(code.to_owned());
        }
    }
    None
}

fn otp_label(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    has_word(&lower, "otp")
        || has_word(&lower, "passcode")
        || has_word(&lower, "verification")
        || lower.contains("one-time")
        || lower.contains("one time")
}

fn tracking(text: &str, lines: &[&str]) -> Option<String> {
    if let Some(value) = ups(text) {
        return Some(value.to_owned());
    }
    for line in lines {
        if !has_word(&line.to_ascii_lowercase(), "tracking") {
            continue;
        }
        if let Some(value) = alnum_token(line, 10, 34) {
            return Some(value.to_owned());
        }
    }
    None
}

fn ups(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut start = 0;
    while start + 18 <= bytes.len() {
        let window = bytes.get(start..start + 18)?;
        let bounded = (start == 0
            || bytes
                .get(start - 1)
                .is_some_and(|byte| !byte.is_ascii_alphanumeric()))
            && bytes
                .get(start + 18)
                .is_none_or(|byte| !byte.is_ascii_alphanumeric());
        if bounded
            && window[0].eq_ignore_ascii_case(&b'1')
            && window[1].eq_ignore_ascii_case(&b'Z')
            && window[2..].iter().all(u8::is_ascii_alphanumeric)
        {
            return text.get(start..start + 18);
        }
        start += 1;
    }
    None
}

fn invoice(lines: &[&str]) -> Option<String> {
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if !has_word(&lower, "total") && !lower.contains("amount due") {
            continue;
        }
        if let Some(value) = money(line) {
            return Some(value);
        }
    }
    None
}

fn money(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'$'
            && let Some(end) = number_end(line, index + 1)
        {
            return line.get(index..end).map(str::to_owned);
        }
        if let Some(code_end) = currency_at(line, index) {
            let after = skip_space(line, code_end);
            if let Some(end) = number_end(line, after) {
                return line.get(index..end).map(str::to_owned);
            }
        }
        if let Some(end) = number_end(line, index) {
            let after = skip_space(line, end);
            if let Some(code_end) = currency_at(line, after) {
                return line.get(index..code_end).map(str::to_owned);
            }
            index = end;
            continue;
        }
        index += 1;
    }
    None
}

fn flight(lines: &[&str]) -> Option<String> {
    for line in lines {
        if !has_word(&line.to_ascii_lowercase(), "flight") {
            continue;
        }
        if let Some(value) = flight_code(line) {
            return Some(value);
        }
    }
    None
}

fn flight_code(line: &str) -> Option<String> {
    let words: Vec<&str> = line
        .split(|ch: char| ch.is_ascii_whitespace())
        .filter(|word| !word.is_empty())
        .collect();
    for (index, word) in words.iter().enumerate() {
        let token = trim_edges(word);
        if combined_flight(token) {
            return Some(token.to_owned());
        }
        let next = words.get(index + 1).map(|word| trim_edges(word));
        if airline(token)
            && let Some(number) = next.filter(|value| flight_number(value))
        {
            return Some(format!("{token}{number}"));
        }
    }
    None
}

fn combined_flight(token: &str) -> bool {
    let bytes = token.as_bytes();
    (3..=6).contains(&bytes.len()) && airline(&token[..2]) && flight_number(&token[2..])
}

fn airline(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() == 2 && bytes.iter().all(u8::is_ascii_alphabetic)
}

fn flight_number(token: &str) -> bool {
    let bytes = token.as_bytes();
    (1..=4).contains(&bytes.len()) && bytes.iter().all(u8::is_ascii_digit)
}

fn digit_token(text: &str, min: usize, max: usize) -> Option<&str> {
    bounded(text, min, max, |byte| byte.is_ascii_digit())
}

fn alnum_token(text: &str, min: usize, max: usize) -> Option<&str> {
    bounded(text, min, max, |byte| byte.is_ascii_alphanumeric())
}

fn bounded(text: &str, min: usize, max: usize, kind: impl Fn(u8) -> bool) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !kind(bytes[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && kind(bytes[index]) {
            index += 1;
        }
        let len = index - start;
        let left = start == 0
            || bytes
                .get(start - 1)
                .is_some_and(|byte| !byte.is_ascii_alphanumeric());
        let right = bytes
            .get(index)
            .is_none_or(|byte| !byte.is_ascii_alphanumeric());
        if left && right && (min..=max).contains(&len) {
            return text.get(start..index);
        }
    }
    None
}

fn number_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let first = bytes.get(start)?;
    if !first.is_ascii_digit() {
        return None;
    }
    let mut index = start + 1;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b',')
    {
        index += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        let dot = index;
        index += 1;
        let frac = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == frac {
            index = dot;
        }
    }
    Some(index)
}

fn currency_at(text: &str, start: usize) -> Option<usize> {
    let rest = text.get(start..)?;
    let code = rest.get(..3)?;
    if !matches!(code.to_ascii_uppercase().as_str(), "USD" | "EUR" | "GBP") {
        return None;
    }
    let end = start + 3;
    let left = start == 0
        || text
            .as_bytes()
            .get(start - 1)
            .is_some_and(|byte| !byte.is_ascii_alphabetic());
    let right = text
        .as_bytes()
        .get(end)
        .is_none_or(|byte| !byte.is_ascii_alphabetic());
    if left && right { Some(end) } else { None }
}

fn skip_space(text: &str, start: usize) -> usize {
    let mut index = start;
    while text
        .as_bytes()
        .get(index)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        index += 1;
    }
    index
}

fn has_word(lower: &str, word: &str) -> bool {
    lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|token| token == word)
}

fn trim_edges(word: &str) -> &str {
    word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulls_each_fact_and_ignores_plain_prose() {
        let text = "\
Your one-time passcode is 482913.\n\
Tracking number 1Z999AA10123456784 is on the way.\n\
Amount due: $42.50\n\
Flight BA249 leaves at noon.\n";
        let facts = extract_facts(text);
        assert_eq!(
            facts,
            vec![
                Fact {
                    kind: FactKind::Otp,
                    value: "482913".to_owned(),
                },
                Fact {
                    kind: FactKind::Tracking,
                    value: "1Z999AA10123456784".to_owned(),
                },
                Fact {
                    kind: FactKind::InvoiceTotal,
                    value: "$42.50".to_owned(),
                },
                Fact {
                    kind: FactKind::Flight,
                    value: "BA249".to_owned(),
                },
            ]
        );
        assert!(extract_facts("See you Thursday.").is_empty());
        assert_eq!(
            extract_facts("Invoice total EUR 18.00")
                .first()
                .unwrap()
                .value,
            "EUR 18.00"
        );
    }
}
