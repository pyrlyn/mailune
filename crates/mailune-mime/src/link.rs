//! The href is the real target. Callers show [`LinkCheck::target`], not the
//! anchor text.
//!
//! Punycode is an A-label (`xn--`) on the host. A lookalike writes `rn`
//! where a known brand writes `m` (`rnicrosoft.com` for `microsoft.com`).
//! Nothing is resolved over the network.

/// What to show for one link, and the flags beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCheck {
    /// The href. This is the real target.
    pub target: String,
    /// The visible label is a different string, so it must not replace `target`.
    pub label_hides_target: bool,
    /// A host label starts with `xn--`.
    pub punycode: bool,
    /// Replacing `rn` with `m` turns the host into a known brand.
    pub lookalike: bool,
    /// The brand `lookalike` matched.
    pub brand: Option<&'static str>,
}

const BRANDS: &[&str] = &[
    "microsoft.com",
    "amazon.com",
    "google.com",
    "gmail.com",
    "apple.com",
    "icloud.com",
    "yahoo.com",
    "outlook.com",
    "paypal.com",
];

/// Inspect `href`. `visible` is the anchor text, when the message has one.
pub fn inspect_link(href: &str, visible: Option<&str>) -> LinkCheck {
    let host = host_of(href).unwrap_or("");
    let brand = lookalike_brand(host);
    LinkCheck {
        target: href.to_string(),
        label_hides_target: label_hides(href, visible),
        punycode: is_punycode(host),
        lookalike: brand.is_some(),
        brand,
    }
}

fn label_hides(href: &str, visible: Option<&str>) -> bool {
    let Some(visible) = visible.map(str::trim).filter(|label| !label.is_empty()) else {
        return false;
    };
    if visible.eq_ignore_ascii_case(href.trim()) {
        return false;
    }
    match (host_of(href), host_of(visible)) {
        (Some(href_host), Some(visible_host)) => !href_host.eq_ignore_ascii_case(visible_host),
        _ => true,
    }
}

fn is_punycode(host: &str) -> bool {
    host.split('.')
        .any(|label| label.to_ascii_lowercase().starts_with("xn--"))
}

fn lookalike_brand(host: &str) -> Option<&'static str> {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if !host.contains("rn") {
        return None;
    }
    let folded = host.replace("rn", "m");
    BRANDS
        .iter()
        .copied()
        .find(|brand| host_is_brand(&folded, brand))
}

fn host_is_brand(host: &str, brand: &str) -> bool {
    host == brand
        || host
            .strip_suffix(brand)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

fn host_of(url: &str) -> Option<&str> {
    let url = url.trim();
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if rest.is_empty() {
        return None;
    }
    let rest = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    let host = if let Some(inner) = rest.strip_prefix('[') {
        inner.split_once(']').map_or(inner, |(host, _)| host)
    } else {
        rest.split_once(':').map_or(rest, |(host, _)| host)
    };
    let host = host.trim_end_matches('.');
    if host.is_empty() { None } else { Some(host) }
}

#[cfg(test)]
mod tests {
    use super::inspect_link;

    #[test]
    fn the_href_is_the_target_when_the_label_differs() {
        let href = "https://microsoft.com/login";
        let check = inspect_link(href, Some("Click here"));
        assert_eq!(check.target, href);
        assert!(check.label_hides_target);
        assert!(!check.punycode);
        assert!(!check.lookalike);
    }

    #[test]
    fn punycode_and_rn_lookalikes_are_flagged() {
        let puny = inspect_link("https://www.xn--pple-43d.com/login", None);
        assert!(puny.punycode);
        assert!(!puny.lookalike);
        assert_eq!(puny.target, "https://www.xn--pple-43d.com/login");

        let microsoft = inspect_link("https://rnicrosoft.com/a", Some("https://rnicrosoft.com/a"));
        assert!(!microsoft.label_hides_target);
        assert!(microsoft.lookalike);
        assert_eq!(microsoft.brand, Some("microsoft.com"));

        let amazon = inspect_link("https://www.arnazon.com/x", None);
        assert_eq!(amazon.brand, Some("amazon.com"));

        let other = inspect_link("https://arnazon.co.uk/", None);
        assert!(!other.lookalike);

        let real = inspect_link("https://microsoft.com/", None);
        assert!(!real.lookalike);
        assert!(!real.punycode);
    }
}
