//! Ask: answer a question from retrieved mail, with citations. The model
//! behind [`Answerer`] only proposes; its citations are kept only when they
//! point at a passage it was actually given, and an answer with no valid
//! citation is refused rather than shown unsourced.

use mailune_protocol::MessageId;

use crate::{Error, Fused};

/// A retrieved message's text, as handed to the answerer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passage {
    /// The message it came from.
    pub id: MessageId,
    /// The text the answer may draw on.
    pub text: String,
}

/// What the answerer proposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The answer text.
    pub text: String,
    /// Messages the answer claims to rest on.
    pub citations: Vec<MessageId>,
}

/// An answer whose every citation is a retrieved message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The answer text.
    pub text: String,
    /// Retrieved messages it cites, first citation first, no repeats.
    pub citations: Vec<MessageId>,
}

/// Something that writes an answer from passages: a local model, or a
/// scripted one in tests. The router that picks one lives elsewhere.
pub trait Answerer {
    /// A draft answer to `question` drawn from `passages`.
    ///
    /// # Errors
    ///
    /// Any failure of the model, as text.
    fn answer(&self, question: &str, passages: &[Passage]) -> Result<Draft, String>;
}

/// Answers `question` from the first `limit` fused hits that `text` can
/// supply, keeping only citations of those hits.
///
/// # Errors
///
/// [`Error::NothingRetrieved`] when no hit has text (the answerer is not
/// called); [`Error::Answerer`] when it fails; [`Error::Uncited`] when the
/// draft is empty or cites nothing that was retrieved.
pub fn ask(
    question: &str,
    hits: &[Fused],
    limit: usize,
    text: impl Fn(&MessageId) -> Option<String>,
    answerer: &dyn Answerer,
) -> Result<Answer, Error> {
    let passages: Vec<Passage> = hits
        .iter()
        .filter_map(|hit| {
            text(&hit.id).map(|text| Passage {
                id: hit.id.clone(),
                text,
            })
        })
        .take(limit)
        .collect();
    if passages.is_empty() {
        return Err(Error::NothingRetrieved);
    }
    let draft = answerer
        .answer(question, &passages)
        .map_err(Error::Answerer)?;
    let mut citations: Vec<MessageId> = Vec::new();
    for cited in draft.citations {
        let retrieved = passages.iter().any(|passage| passage.id == cited);
        if retrieved && !citations.contains(&cited) {
            citations.push(cited);
        }
    }
    if draft.text.trim().is_empty() || citations.is_empty() {
        return Err(Error::Uncited);
    }
    Ok(Answer {
        text: draft.text,
        citations,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use mailune_protocol::MessageId;

    use super::{Answerer, Draft, Passage, ask};
    use crate::{Candidate, Error, fuse, parse_query};

    /// Cites what it is told to, and counts its calls.
    struct Scripted {
        cites: Vec<&'static str>,
        calls: Cell<usize>,
    }

    impl Answerer for Scripted {
        fn answer(&self, question: &str, passages: &[Passage]) -> Result<Draft, String> {
            self.calls.set(self.calls.get() + 1);
            let seen: Vec<&str> = passages.iter().map(|p| p.id.as_str()).collect();
            Ok(Draft {
                text: format!("{question}: Friday (read {})", seen.join(",")),
                citations: self.cites.iter().map(|id| MessageId::new(*id)).collect(),
            })
        }
    }

    struct Failing;

    impl Answerer for Failing {
        fn answer(&self, _: &str, _: &[Passage]) -> Result<Draft, String> {
            Err("model unloaded".into())
        }
    }

    fn candidate(id: &str) -> Candidate {
        Candidate {
            id: MessageId::new(id),
            from: "ada@example.com".into(),
            to: Vec::new(),
            has_attachment: false,
            unread: false,
            labels: Vec::new(),
            date: None,
        }
    }

    fn body(id: &MessageId) -> Option<String> {
        match id.as_str() {
            "m1" => Some("The dock meeting moved to Friday.".into()),
            "m2" => Some("Invoice for the boat.".into()),
            "m3" => Some("Harbour photos.".into()),
            _ => None,
        }
    }

    #[test]
    fn citations_are_kept_only_for_retrieved_messages() {
        let query = parse_query("dock").unwrap();
        let lexical = [candidate("m1"), candidate("m2"), candidate("gone")];
        let semantic = [candidate("m3"), candidate("m1")];
        let hits = fuse(&lexical, &semantic, &query);
        let answerer = Scripted {
            cites: vec!["m9", "m1", "m1", "m2", "m3"],
            calls: Cell::new(0),
        };
        let answer = ask("When is the dock meeting", &hits, 2, body, &answerer).unwrap();
        // m1 is in both lists and m3 heads one, so those two are passed on;
        // m2 ranked third and is cited but was not retrieved.
        assert!(answer.text.contains("read m1,m3"), "{}", answer.text);
        assert_eq!(
            answer.citations,
            [MessageId::new("m1"), MessageId::new("m3")]
        );
        assert_eq!(answerer.calls.get(), 1);
    }

    #[test]
    fn no_source_means_no_answer() {
        let hits = fuse(&[candidate("gone")], &[], &parse_query("x").unwrap());
        let answerer = Scripted {
            cites: vec!["m1"],
            calls: Cell::new(0),
        };
        assert!(matches!(
            ask("q", &hits, 3, body, &answerer),
            Err(Error::NothingRetrieved)
        ));
        assert_eq!(answerer.calls.get(), 0);

        let hits = fuse(&[candidate("m2")], &[], &parse_query("x").unwrap());
        let invented = Scripted {
            cites: vec!["m1"],
            calls: Cell::new(0),
        };
        assert!(matches!(
            ask("q", &hits, 3, body, &invented),
            Err(Error::Uncited)
        ));
        assert!(matches!(
            ask("q", &hits, 3, body, &Failing),
            Err(Error::Answerer(reason)) if reason == "model unloaded"
        ));
    }
}
