//! Answer a question from ranked hits already in memory.
//!
//! Each citation points at one of those ids. No model is called and no
//! database is opened.

use crate::search::SearchDoc;

const EXCERPT_CHARS: usize = 240;

/// One hit the answer is willing to point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// Id from the ranked hit.
    pub id: String,
    /// Text from that hit, shortened so a citation stays a pointer.
    pub excerpt: String,
}

/// Citations for a question. The order is the ranked hit order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// Hits whose text contains a word from the question.
    pub citations: Vec<Citation>,
}

/// Cites the ranked hits that mention the question.
///
/// An empty question cites nothing. Hits stay in the order they were given.
pub fn ask(question: &str, hits: &[SearchDoc]) -> Answer {
    let words: Vec<String> = question
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return Answer {
            citations: Vec::new(),
        };
    }
    let citations = hits
        .iter()
        .filter(|hit| mentions(&hit.text, &words))
        .map(|hit| Citation {
            id: hit.id.clone(),
            excerpt: excerpt(&hit.text),
        })
        .collect();
    Answer { citations }
}

fn mentions(text: &str, words: &[String]) -> bool {
    let text = text.to_lowercase();
    words.iter().any(|word| text.contains(word))
}

fn excerpt(text: &str) -> String {
    if text.chars().count() <= EXCERPT_CHARS {
        return text.to_string();
    }
    text.chars().take(EXCERPT_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::ask;
    use crate::search::{Date, SearchDoc};

    fn hit(id: &str, text: &str) -> SearchDoc {
        SearchDoc {
            id: id.to_string(),
            from: String::new(),
            to: Vec::new(),
            has_attachment: false,
            on: Date {
                year: 2026,
                month: 3,
                day: 1,
            },
            unread: false,
            labels: Vec::new(),
            text: text.to_string(),
        }
    }

    #[test]
    fn citations_point_at_the_ranked_ids_that_mention_the_question() {
        let hits = [
            hit("m1", "See you at the dock"),
            hit("m2", "A different note"),
            hit("m3", "The dock again"),
        ];
        let answer = ask("Dock", &hits);
        assert_eq!(answer.citations.len(), 2);
        assert_eq!(answer.citations[0].id, "m1");
        assert_eq!(answer.citations[1].id, "m3");
        assert_eq!(answer.citations[0].excerpt, "See you at the dock");
        assert!(ask("", &hits).citations.is_empty());
    }
}
