//! Search query parser and reciprocal rank fusion.
//!
//! Field tokens are the lowercase keys from the search language. Anything else
//! with a colon is an error, so a typo is not silently treated as text.
//! Fusion ranks two id lists and still applies those filters. It does not
//! open a database.

use crate::Error;

/// A parsed query, in the order the tokens were written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Field tokens and free-text words.
    pub terms: Vec<Term>,
}

/// One search token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// `from:`.
    From(String),
    /// `to:`.
    To(String),
    /// `has:attachment`.
    HasAttachment,
    /// `before:` as a calendar date.
    Before(Date),
    /// `is:unread`.
    Unread,
    /// `label:`.
    Label(String),
    /// A word that is not a field token.
    Text(String),
}

/// A calendar day from `before:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    /// Year, as written.
    pub year: u16,
    /// Month, 1–12.
    pub month: u8,
    /// Day within that month.
    pub day: u8,
}

/// Parses `input`. An empty string is an empty query.
///
/// # Errors
///
/// [`Error::BadQuery`] for an unknown key, an empty value, a bad `has:` or
/// `is:` value, a bad date, or an unterminated quote.
pub fn parse_query(input: &str) -> Result<Query, Error> {
    let mut terms = Vec::new();
    let mut index = 0;
    while let Some(next) = skip_ws(input, index) {
        let (term, consumed) = one_term(input, next)?;
        terms.push(term);
        index = consumed;
    }
    Ok(Query { terms })
}

fn one_term(input: &str, index: usize) -> Result<(Term, usize), Error> {
    let rest = &input[index..];
    if rest.starts_with('"') {
        let (text, consumed) = quoted(input, index)?;
        return Ok((Term::Text(text), consumed));
    }
    let key_end = rest
        .find([' ', '\t', '\n', '\r', ':'])
        .map_or(input.len(), |offset| index + offset);
    if input.as_bytes().get(key_end).copied() != Some(b':') {
        let word = &input[index..key_end];
        return Ok((Term::Text(word.to_string()), key_end));
    }
    let key = &input[index..key_end];
    let value_at = key_end + 1;
    let (value, consumed) = if input[value_at..].starts_with('"') {
        quoted(input, value_at)?
    } else {
        let end = input[value_at..]
            .find(char::is_whitespace)
            .map_or(input.len(), |offset| value_at + offset);
        (input[value_at..end].to_string(), end)
    };
    if value.is_empty() {
        return Err(bad(key));
    }
    Ok((field(key, &value)?, consumed))
}

fn field(key: &str, value: &str) -> Result<Term, Error> {
    match key {
        "from" => Ok(Term::From(value.to_string())),
        "to" => Ok(Term::To(value.to_string())),
        "label" => Ok(Term::Label(value.to_string())),
        "has" if value == "attachment" => Ok(Term::HasAttachment),
        "is" if value == "unread" => Ok(Term::Unread),
        "before" => Ok(Term::Before(parse_date(value)?)),
        _ => Err(bad(&format!("{key}:{value}"))),
    }
}

fn parse_date(value: &str) -> Result<Date, Error> {
    let mut parts = value.split('-');
    let year = parts.next().ok_or_else(|| bad(value))?;
    let month = parts.next().ok_or_else(|| bad(value))?;
    let day = parts.next().ok_or_else(|| bad(value))?;
    if parts.next().is_some() {
        return Err(bad(value));
    }
    let year: u16 = year.parse().map_err(|_| bad(value))?;
    let month: u8 = month.parse().map_err(|_| bad(value))?;
    let day: u8 = day.parse().map_err(|_| bad(value))?;
    if month == 0 || month > 12 || day == 0 || day > days_in_month(year, month) {
        return Err(bad(value));
    }
    Ok(Date { year, month, day })
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn leap(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn quoted(input: &str, index: usize) -> Result<(String, usize), Error> {
    let mut text = String::new();
    let mut chars = input[index + 1..].char_indices();
    for (offset, ch) in chars.by_ref() {
        if ch == '"' {
            return Ok((text, index + 1 + offset + ch.len_utf8()));
        }
        text.push(ch);
    }
    Err(bad(&input[index..]))
}

fn skip_ws(input: &str, index: usize) -> Option<usize> {
    let rest = input[index..].trim_start();
    if rest.is_empty() {
        None
    } else {
        Some(input.len() - rest.len())
    }
}

/// One document fusion can keep or drop.
///
/// The lists being fused only carry ids. The filters need these fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDoc {
    /// Id shared with the ranked lists.
    pub id: String,
    /// `from:` haystack.
    pub from: String,
    /// `to:` haystacks.
    pub to: Vec<String>,
    /// `has:attachment`.
    pub has_attachment: bool,
    /// Calendar day compared with `before:`.
    pub on: Date,
    /// `is:unread`.
    pub unread: bool,
    /// `label:` values.
    pub labels: Vec<String>,
    /// Free-text haystack: subject, addresses, and body.
    pub text: String,
}

/// Reciprocal rank fusion constant from the retrieval design.
const RRF_K: f64 = 60.0;

/// Fuses `lexical` and `vector` with reciprocal rank fusion at k = 60.
///
/// Rank is the 1-based first occurrence in a list. A document that fails
/// `query` is dropped, including when the query is empty of field tokens and
/// only free-text terms remain. An id with no [`SearchDoc`] is dropped.
/// Higher score comes first. Equal scores break ties by id.
pub fn fuse(
    lexical: &[String],
    vector: &[String],
    query: &Query,
    docs: &[SearchDoc],
) -> Vec<String> {
    let mut scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for list in [lexical, vector] {
        let mut seen = std::collections::HashSet::new();
        for (index, id) in list.iter().enumerate() {
            if !seen.insert(id) {
                continue;
            }
            let Some(doc) = docs.iter().find(|doc| &doc.id == id) else {
                continue;
            };
            if !matches_doc(doc, query) {
                continue;
            }
            let Ok(rank) = u32::try_from(index + 1) else {
                continue;
            };
            let rank = f64::from(rank);
            *scores.entry(id.clone()).or_default() += 1.0 / (RRF_K + rank);
        }
    }
    let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    ranked.into_iter().map(|(id, _)| id).collect()
}

fn matches_doc(doc: &SearchDoc, query: &Query) -> bool {
    query.terms.iter().all(|term| match term {
        Term::From(value) => contains(&doc.from, value),
        Term::To(value) => doc.to.iter().any(|addr| contains(addr, value)),
        Term::HasAttachment => doc.has_attachment,
        Term::Before(date) => {
            (doc.on.year, doc.on.month, doc.on.day) < (date.year, date.month, date.day)
        }
        Term::Unread => doc.unread,
        Term::Label(value) => doc.labels.iter().any(|label| contains(label, value)),
        Term::Text(value) => contains(&doc.text, value),
    })
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn bad(token: &str) -> Error {
    Error::BadQuery {
        token: token.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Date, Term, parse_query};
    use crate::Error;

    #[test]
    fn a_mixed_query_parses() {
        let query = parse_query(
            "from:ada to:\"Bob Smith\" has:attachment before:2024-02-29 is:unread label:work hello",
        )
        .unwrap();
        assert_eq!(
            query.terms,
            vec![
                Term::From("ada".into()),
                Term::To("Bob Smith".into()),
                Term::HasAttachment,
                Term::Before(Date {
                    year: 2024,
                    month: 2,
                    day: 29
                }),
                Term::Unread,
                Term::Label("work".into()),
                Term::Text("hello".into()),
            ]
        );
    }

    #[test]
    fn bad_tokens_are_errors() {
        let cases = [
            "has:image",
            "is:read",
            "nope:x",
            "before:yesterday",
            "before:2023-02-29",
            "from:",
            "\"unterminated",
        ];
        for input in cases {
            assert!(
                matches!(parse_query(input), Err(Error::BadQuery { .. })),
                "{input}"
            );
        }
        assert!(parse_query("").unwrap().terms.is_empty());
    }

    fn doc(id: &str, unread: bool, text: &str) -> super::SearchDoc {
        super::SearchDoc {
            id: id.to_string(),
            from: "ada@example.com".into(),
            to: vec!["bob@example.com".into()],
            has_attachment: false,
            on: Date {
                year: 2024,
                month: 1,
                day: 2,
            },
            unread,
            labels: vec!["work".into()],
            text: text.to_string(),
        }
    }

    #[test]
    fn fusion_uses_reciprocal_rank_and_query_filters() {
        let docs = vec![
            doc("a", false, "hello"),
            doc("b", true, "hello"),
            doc("c", true, "hello"),
            doc("d", true, "other"),
        ];
        let lexical = ["a", "b", "c"].map(str::to_string);
        let vector = ["b", "d"].map(str::to_string);
        let open = parse_query("").unwrap();
        assert_eq!(
            super::fuse(&lexical, &vector, &open, &docs),
            vec![
                "b".to_string(),
                "a".to_string(),
                "d".to_string(),
                "c".to_string()
            ]
        );
        let filtered = parse_query("is:unread hello").unwrap();
        assert_eq!(
            super::fuse(&lexical, &vector, &filtered, &docs),
            vec!["b".to_string(), "c".to_string()]
        );

        let mut rich = doc("b", true, "hello");
        rich.has_attachment = true;
        let lexical = vec!["b".to_string()];
        let query =
            parse_query("from:ada to:bob has:attachment before:2024-06-01 label:work hello")
                .unwrap();
        assert_eq!(
            super::fuse(&lexical, &[], &query, &[rich]),
            vec!["b".to_string()]
        );
    }
}
