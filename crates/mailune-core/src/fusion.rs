//! Hybrid retrieval: reciprocal rank fusion of a lexical (FTS5 BM25) list
//! and a semantic (vector KNN) list, then the field filters of the query.
//!
//! The lists arrive already ranked. Nothing here touches a database, so the
//! fusion rule is the same for the native store and the WASM build.

use std::collections::HashMap;

use mailune_protocol::MessageId;

use crate::search::{Date, Query, Term};

/// The RRF constant from Cormack et al.; it damps the head of each list.
pub const RRF_K: u32 = 60;

/// A ranked hit with the fields the query filters need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Message the hit points at.
    pub id: MessageId,
    /// Sender, as `Name <addr>` or just the address.
    pub from: String,
    /// Recipients in the same form.
    pub to: Vec<String>,
    /// The message has an attachment.
    pub has_attachment: bool,
    /// The message is unread.
    pub unread: bool,
    /// Labels and keywords.
    pub labels: Vec<String>,
    /// Calendar day the message arrived, if known.
    pub date: Option<Date>,
}

/// One fused result.
#[derive(Debug, Clone, PartialEq)]
pub struct Fused {
    /// Message id.
    pub id: MessageId,
    /// Sum of `1 / (RRF_K + rank)` over the lists it appears in.
    pub score: f64,
}

/// Fuses the two ranked lists and drops hits the query's field terms
/// exclude. Free-text terms are not re-checked: they produced the lists.
pub fn fuse(lexical: &[Candidate], semantic: &[Candidate], query: &Query) -> Vec<Fused> {
    let mut scores: HashMap<&MessageId, f64> = HashMap::new();
    let mut order: Vec<&Candidate> = Vec::new();
    for list in [lexical, semantic] {
        let mut seen_here = Vec::new();
        for (rank, candidate) in list.iter().enumerate() {
            // A list that repeats an id counts it once, at its best rank.
            if seen_here.contains(&&candidate.id) {
                continue;
            }
            seen_here.push(&candidate.id);
            let rank = u32::try_from(rank).unwrap_or(u32::MAX).saturating_add(1);
            let entry = scores.entry(&candidate.id).or_insert_with(|| {
                order.push(candidate);
                0.0
            });
            *entry += 1.0 / f64::from(RRF_K.saturating_add(rank));
        }
    }
    let mut fused: Vec<Fused> = order
        .into_iter()
        .filter(|candidate| passes(candidate, query))
        .map(|candidate| Fused {
            id: candidate.id.clone(),
            score: scores.get(&candidate.id).copied().unwrap_or(0.0),
        })
        .collect();
    fused.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
    fused
}

fn passes(candidate: &Candidate, query: &Query) -> bool {
    query.terms.iter().all(|term| match term {
        Term::From(who) => contains(&candidate.from, who),
        Term::To(who) => candidate.to.iter().any(|to| contains(to, who)),
        Term::HasAttachment => candidate.has_attachment,
        Term::Unread => candidate.unread,
        Term::Label(label) => candidate
            .labels
            .iter()
            .any(|have| have.eq_ignore_ascii_case(label)),
        Term::Before(limit) => candidate.date.is_some_and(|date| day(date) < day(*limit)),
        Term::Text(_) => true,
    })
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn day(date: Date) -> (u16, u8, u8) {
    (date.year, date.month, date.day)
}

#[cfg(test)]
mod tests {
    use mailune_protocol::MessageId;

    use super::{Candidate, RRF_K, fuse};
    use crate::search::{Date, parse_query};

    fn hit(id: &str) -> Candidate {
        Candidate {
            id: MessageId::new(id),
            from: "Ada <ada@example.com>".into(),
            to: vec!["me@example.com".into()],
            has_attachment: false,
            unread: false,
            labels: Vec::new(),
            date: Some(Date {
                year: 2026,
                month: 3,
                day: 1,
            }),
        }
    }

    fn ids(fused: &[super::Fused]) -> Vec<&str> {
        fused.iter().map(|item| item.id.as_str()).collect()
    }

    #[test]
    fn rrf_rewards_agreement_with_k_sixty() {
        let lexical = [hit("a"), hit("b"), hit("c")];
        let semantic = [hit("c"), hit("a"), hit("d")];
        let fused = fuse(&lexical, &semantic, &parse_query("").unwrap());
        assert_eq!(ids(&fused), ["a", "c", "b", "d"]);
        let k = f64::from(RRF_K);
        let a = 1.0 / (k + 1.0) + 1.0 / (k + 2.0);
        assert!((fused[0].score - a).abs() < 1e-12);
        assert!((fused[3].score - 1.0 / (k + 3.0)).abs() < 1e-12);
    }

    #[test]
    fn a_repeated_id_in_one_list_counts_once() {
        let fused = fuse(&[hit("a"), hit("a")], &[], &parse_query("").unwrap());
        assert_eq!(fused.len(), 1);
        assert!((fused[0].score - 1.0 / 61.0).abs() < 1e-12);
    }

    #[test]
    fn field_filters_from_the_parser_still_apply() {
        let mut bob = hit("bob");
        bob.from = "Bob <bob@example.com>".into();
        let mut attached = hit("att");
        attached.has_attachment = true;
        attached.unread = true;
        attached.labels = vec!["Work".into()];
        let mut old = hit("old");
        old.date = Some(Date {
            year: 2020,
            month: 1,
            day: 1,
        });
        let mut undated = hit("undated");
        undated.date = None;
        let lexical = [hit("ada"), bob, attached, old, undated];
        let semantic = [];

        let from = fuse(
            &lexical,
            &semantic,
            &parse_query("from:ADA invoice").unwrap(),
        );
        assert_eq!(ids(&from), ["ada", "att", "old", "undated"]);
        let has = fuse(
            &lexical,
            &semantic,
            &parse_query("has:attachment is:unread label:work").unwrap(),
        );
        assert_eq!(ids(&has), ["att"]);
        let before = fuse(
            &lexical,
            &semantic,
            &parse_query("before:2021-01-01").unwrap(),
        );
        assert_eq!(ids(&before), ["old"]);
        let to = fuse(&lexical, &semantic, &parse_query("to:nobody").unwrap());
        assert!(to.is_empty());
    }
}
