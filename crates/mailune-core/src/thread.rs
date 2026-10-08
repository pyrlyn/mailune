//! JWZ-style threading from Message-Id, In-Reply-To, and References.
//!
//! Subject normalisation is only a fallback for messages that never named each
//! other. An empty subject does not match: otherwise every blank subject would
//! become one conversation. A provider thread id is a server hint and links
//! messages before that fallback runs.

use std::collections::HashMap;

/// One message the threader can see. Headers only; no body and no I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Threadable {
    /// `Message-Id` without the caller needing a particular bracket style.
    pub message_id: String,
    /// `In-Reply-To`, when the header was present.
    pub in_reply_to: Option<String>,
    /// `References`, oldest first.
    pub references: Vec<String>,
    /// Raw `Subject`, before normalisation.
    pub subject: String,
    /// Server thread id, when the provider already grouped this message.
    pub provider_thread: Option<String>,
}

/// One node in a thread tree. `message_id` is absent for a subject-only parent
/// that no message claimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    /// `Message-Id`, or `None` for an empty parent created by subject grouping.
    pub message_id: Option<String>,
    /// Replies, in the order they were linked.
    pub children: Vec<Container>,
}

/// Subject after reply prefixes and a trailing `(fwd)` are removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedSubject {
    /// Comparison key. Lower case, internal whitespace collapsed.
    pub base: String,
    /// A reply prefix or a trailing `(fwd)` was stripped.
    pub is_reply: bool,
}

const PREFIXES: &[&str] = &["re:", "fw:", "fwd:", "aw:", "sv:", "vs:", "antw:"];

/// Strips reply markers. The base is what subject grouping compares.
pub fn normalize_subject(subject: &str) -> NormalizedSubject {
    let mut text = subject.trim().to_string();
    let mut is_reply = false;
    loop {
        let mut stripped = false;
        for prefix in PREFIXES {
            if let Some(rest) = strip_prefix_ci(&text, prefix) {
                text = rest.trim().to_string();
                is_reply = true;
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }
    if let Some(rest) = strip_suffix_ci(&text, "(fwd)") {
        text = rest.trim().to_string();
        is_reply = true;
    }
    let base = collapse(&text);
    if base.is_empty() {
        is_reply = false;
    }
    NormalizedSubject { base, is_reply }
}

/// Builds a forest. Input order is the order children are linked.
pub fn thread_messages(messages: &[Threadable]) -> Vec<Container> {
    let mut nodes = Vec::new();
    let mut by_id = HashMap::new();
    for message in messages {
        let normalized = normalize_subject(&message.subject);
        let child = ensure(&mut nodes, &mut by_id, &message.message_id);
        nodes[child].base = normalized.base;
        nodes[child].is_reply = normalized.is_reply;
        let mut chain: Vec<&str> = message.references.iter().map(String::as_str).collect();
        if let Some(reply_to) = message.in_reply_to.as_deref()
            && chain.last().copied() != Some(reply_to)
        {
            chain.push(reply_to);
        }
        let mut previous = None;
        for id in chain {
            let index = ensure(&mut nodes, &mut by_id, id);
            if let Some(parent) = previous {
                link(&mut nodes, parent, index);
            }
            previous = Some(index);
        }
        if let Some(parent) = previous {
            link(&mut nodes, parent, child);
        }
    }
    group_by_provider(&mut nodes, &by_id, messages);
    group_by_subject(&mut nodes);
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.parent.is_none())
        .map(|(index, _)| build(&nodes, index))
        .collect()
}

struct Node {
    id: Option<String>,
    parent: Option<usize>,
    children: Vec<usize>,
    base: String,
    is_reply: bool,
}

fn ensure(nodes: &mut Vec<Node>, by_id: &mut HashMap<String, usize>, id: &str) -> usize {
    if let Some(index) = by_id.get(id).copied() {
        return index;
    }
    let index = nodes.len();
    nodes.push(Node {
        id: Some(id.to_string()),
        parent: None,
        children: Vec::new(),
        base: String::new(),
        is_reply: false,
    });
    by_id.insert(id.to_string(), index);
    index
}

fn link(nodes: &mut [Node], parent: usize, child: usize) {
    if parent == child || nodes[child].parent.is_some() || ancestor(nodes, child, parent) {
        return;
    }
    nodes[child].parent = Some(parent);
    nodes[parent].children.push(child);
}

fn ancestor(nodes: &[Node], maybe_ancestor: usize, node: usize) -> bool {
    let mut current = Some(node);
    while let Some(index) = current {
        if index == maybe_ancestor {
            return true;
        }
        current = nodes[index].parent;
    }
    false
}

fn root_of(nodes: &[Node], mut index: usize) -> usize {
    while let Some(parent) = nodes[index].parent {
        index = parent;
    }
    index
}

fn group_by_provider(nodes: &mut [Node], by_id: &HashMap<String, usize>, messages: &[Threadable]) {
    let mut first: HashMap<&str, usize> = HashMap::new();
    for message in messages {
        let Some(provider) = message.provider_thread.as_deref() else {
            continue;
        };
        let Some(&index) = by_id.get(&message.message_id) else {
            continue;
        };
        if let Some(&earlier) = first.get(provider) {
            let parent = root_of(nodes, earlier);
            let child = root_of(nodes, index);
            link(nodes, parent, child);
        } else {
            first.insert(provider, index);
        }
    }
}

fn group_by_subject(nodes: &mut Vec<Node>) {
    let roots: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.parent.is_none() && !node.base.is_empty())
        .map(|(index, _)| index)
        .collect();
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for root in roots {
        let base = nodes[root].base.clone();
        if let Some((_, members)) = groups.iter_mut().find(|(key, _)| key == &base) {
            members.push(root);
        } else {
            groups.push((base, vec![root]));
        }
    }
    for (_, members) in groups {
        if members.len() < 2 {
            continue;
        }
        let originals: Vec<usize> = members
            .iter()
            .copied()
            .filter(|index| !nodes[*index].is_reply)
            .collect();
        if originals.len() == 1 {
            let parent = originals[0];
            for child in members {
                if child != parent {
                    link(nodes, parent, child);
                }
            }
        } else if originals.is_empty() {
            let base = nodes[members[0]].base.clone();
            let parent = nodes.len();
            nodes.push(Node {
                id: None,
                parent: None,
                children: Vec::new(),
                base,
                is_reply: false,
            });
            for child in members {
                link(nodes, parent, child);
            }
        } else {
            let parent = originals[0];
            for child in members {
                if nodes[child].is_reply {
                    link(nodes, parent, child);
                }
            }
        }
    }
}

fn build(nodes: &[Node], index: usize) -> Container {
    Container {
        message_id: nodes[index].id.clone(),
        children: nodes[index]
            .children
            .iter()
            .copied()
            .map(|child| build(nodes, child))
            .collect(),
    }
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let mut rest = text.chars();
    for expected in prefix.chars() {
        let got = rest.next()?;
        if !got.eq_ignore_ascii_case(&expected) {
            return None;
        }
    }
    Some(rest.as_str())
}

fn strip_suffix_ci<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    let text_chars: Vec<char> = text.chars().collect();
    let suffix_chars: Vec<char> = suffix.chars().collect();
    if text_chars.len() < suffix_chars.len() {
        return None;
    }
    let start = text_chars.len() - suffix_chars.len();
    let matches = text_chars[start..]
        .iter()
        .zip(suffix_chars.iter())
        .all(|(got, expected)| got.eq_ignore_ascii_case(expected));
    if !matches {
        return None;
    }
    let bytes = text_chars[..start].iter().map(|ch| ch.len_utf8()).sum();
    Some(&text[..bytes])
}

fn collapse(text: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Threadable, normalize_subject, thread_messages};

    fn msg(id: &str, reply_to: Option<&str>, refs: &[&str], subject: &str) -> Threadable {
        Threadable {
            message_id: id.into(),
            in_reply_to: reply_to.map(str::to_string),
            references: refs.iter().map(|item| (*item).to_string()).collect(),
            subject: subject.into(),
            provider_thread: None,
        }
    }

    fn shape(messages: &[Threadable]) -> String {
        thread_messages(messages)
            .iter()
            .map(one)
            .collect::<Vec<_>>()
            .join(",")
    }

    fn one(node: &super::Container) -> String {
        let id = node.message_id.clone().unwrap_or_else(|| "*".into());
        if node.children.is_empty() {
            id
        } else {
            let kids = node.children.iter().map(one).collect::<Vec<_>>().join(",");
            format!("{id}({kids})")
        }
    }

    #[test]
    fn threads_match_the_table() {
        let cases = [
            (
                "in-reply-to",
                vec![
                    msg("A", None, &[], "Hello"),
                    msg("B", Some("A"), &[], "Re: Hello"),
                ],
                "A(B)",
            ),
            (
                "references",
                vec![msg("C", None, &["A", "B"], "Re: Hello")],
                "A(B(C))",
            ),
            (
                "subject",
                vec![
                    msg("A", None, &[], "Hello"),
                    msg("B", None, &[], "Re: Hello"),
                ],
                "A(B)",
            ),
            (
                "fwd and aw",
                vec![
                    msg("A", None, &[], "Hello"),
                    msg("B", None, &[], "Fwd: Hello"),
                    msg("C", None, &[], "Aw: Hello"),
                ],
                "A(B,C)",
            ),
            (
                "distinct subjects",
                vec![msg("A", None, &[], "Hello"), msg("B", None, &[], "Other")],
                "A,B",
            ),
            (
                "blank subjects stay apart",
                vec![msg("A", None, &[], ""), msg("B", None, &[], "   ")],
                "A,B",
            ),
            (
                "replies without an original",
                vec![
                    msg("A", None, &[], "Re: Hello"),
                    msg("B", None, &[], "Re: Re: Hello"),
                ],
                "*(A,B)",
            ),
            (
                "trailing fwd",
                vec![
                    msg("A", None, &[], "Hello"),
                    msg("B", None, &[], "Hello (fwd)"),
                ],
                "A(B)",
            ),
        ];
        for (name, messages, expect) in cases {
            assert_eq!(shape(&messages), expect, "{name}");
        }
    }

    #[test]
    fn a_provider_thread_id_links_unrelated_subjects() {
        let mut first = msg("A", None, &[], "Invoice");
        let mut second = msg("B", None, &[], "Question");
        first.provider_thread = Some("srv-1".into());
        second.provider_thread = Some("srv-1".into());
        assert_eq!(shape(&[first, second]), "A(B)");
    }

    #[test]
    fn normalisation_strips_nested_prefixes() {
        let normalized = normalize_subject("  Re: Aw: Hello   World (fwd) ");
        assert_eq!(normalized.base, "hello world");
        assert!(normalized.is_reply);
        assert_eq!(normalize_subject("Hello").base, "hello");
        assert!(!normalize_subject("Hello").is_reply);
    }
}
