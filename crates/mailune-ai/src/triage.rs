//! Header heuristics for the four triage tabs.
//!
//! Social networks, machine mail, and bulk lists are separated before a person
//! is. No model is called: the class comes from the sender and a few headers.

use mailune_protocol::Category;

/// Headers the heuristic reads. The body is ignored on purpose.
#[derive(Debug, Clone, Copy)]
pub struct TriageInput<'a> {
    /// `From`, display name and address.
    pub from: &'a str,
    /// `Subject`.
    pub subject: &'a str,
    /// The message has a `List-Unsubscribe` header.
    pub list_unsubscribe: bool,
    /// `Precedence`, when present.
    pub precedence: Option<&'a str>,
    /// `Auto-Submitted`, when present. RFC 3834's `no` means a person wrote it.
    pub auto_submitted: Option<&'a str>,
}

/// Assigns Primary, Social, Promotions, or Updates.
pub fn categorize(input: &TriageInput<'_>) -> Category {
    // A social network's own mailer stays Social even when it is also a list.
    if social_sender(input.from) {
        return Category::Social;
    }
    // A receipt is a record. An unsubscribe link on the same message does not
    // make it an advertisement.
    if automated(input.auto_submitted) || transactional(input.subject) {
        return Category::Updates;
    }
    if input.list_unsubscribe || bulk(input.precedence) {
        return Category::Promotions;
    }
    Category::Primary
}

fn social_sender(from: &str) -> bool {
    const DOMAINS: &[&str] = &[
        "facebookmail.com",
        "linkedin.com",
        "twitter.com",
        "instagram.com",
        "tiktok.com",
        "pinterest.com",
        "reddit.com",
    ];
    let Some(domain) = sender_domain(from) else {
        return false;
    };
    DOMAINS
        .iter()
        .any(|known| domain == *known || is_subdomain(&domain, known))
}

fn is_subdomain(domain: &str, known: &str) -> bool {
    domain
        .strip_suffix(known)
        .is_some_and(|prefix| prefix.ends_with('.'))
}

fn sender_domain(from: &str) -> Option<String> {
    let address = from
        .rsplit(['<', ' '])
        .next()
        .unwrap_or(from)
        .trim()
        .trim_end_matches('>');
    let domain = address.rsplit_once('@')?.1.trim();
    if domain.is_empty() {
        None
    } else {
        Some(domain.to_ascii_lowercase())
    }
}

fn automated(auto_submitted: Option<&str>) -> bool {
    auto_submitted
        .is_some_and(|value| !value.trim().eq_ignore_ascii_case("no") && !value.trim().is_empty())
}

fn transactional(subject: &str) -> bool {
    const MARKERS: &[&str] = &[
        "receipt",
        "invoice",
        "shipped",
        "password",
        "your order",
        "order confirmation",
    ];
    let subject = subject.to_ascii_lowercase();
    MARKERS.iter().any(|marker| subject.contains(marker))
}

fn bulk(precedence: Option<&str>) -> bool {
    precedence.is_some_and(|value| {
        let value = value.trim();
        value.eq_ignore_ascii_case("bulk")
            || value.eq_ignore_ascii_case("list")
            || value.eq_ignore_ascii_case("junk")
    })
}

#[cfg(test)]
mod tests {
    use mailune_protocol::Category;

    use super::{TriageInput, categorize};

    fn input<'a>(
        from: &'a str,
        subject: &'a str,
        list_unsubscribe: bool,
        precedence: Option<&'a str>,
        auto_submitted: Option<&'a str>,
    ) -> TriageInput<'a> {
        TriageInput {
            from,
            subject,
            list_unsubscribe,
            precedence,
            auto_submitted,
        }
    }

    #[test]
    fn headers_pick_a_category_without_a_model() {
        assert_eq!(
            categorize(&input(
                "Ada Lovelace <ada@acme.io>",
                "Lunch",
                false,
                None,
                None
            )),
            Category::Primary
        );
        assert_eq!(
            categorize(&input(
                "Facebook <notify@facebookmail.com>",
                "Ada liked your post",
                true,
                Some("bulk"),
                None
            )),
            Category::Social
        );
        assert_eq!(
            categorize(&input(
                "Shop <deals@shop.example>",
                "A sale for you",
                true,
                None,
                None
            )),
            Category::Promotions
        );
        assert_eq!(
            categorize(&input(
                "List <list@example>",
                "Weekly notes",
                false,
                Some("list"),
                None
            )),
            Category::Promotions
        );
        assert_eq!(
            categorize(&input(
                "Billing <billing@shop.example>",
                "Your receipt",
                true,
                Some("bulk"),
                None
            )),
            Category::Updates
        );
        assert_eq!(
            categorize(&input(
                "Bot <bot@shop.example>",
                "Status",
                false,
                None,
                Some("auto-generated")
            )),
            Category::Updates
        );
        assert_eq!(
            categorize(&input(
                "Ada <ada@acme.io>",
                "Hello",
                false,
                None,
                Some("no")
            )),
            Category::Primary
        );
    }
}
