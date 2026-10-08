//! RFC 8058 one-click unsubscribe, parsed and not performed.
//!
//! `List-Unsubscribe` holds the URIs (RFC 2369 angle brackets).
//! `List-Unsubscribe-Post: List-Unsubscribe=One-Click` is what makes the
//! HTTPS URI a one-click POST. An HTTPS URI on its own is not one-click,
//! and a POST header with only a `mailto:` URI has nowhere to POST.
//! This function does not open a connection.

/// URIs from `List-Unsubscribe`, plus whether one-click POST is allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsubscribe {
    /// `https` URIs, in header order.
    pub https: Vec<String>,
    /// `mailto` URIs, in header order.
    pub mailto: Vec<String>,
    /// True only when the POST header is `List-Unsubscribe=One-Click` and `https` is non-empty.
    pub one_click: bool,
    /// The first `https` URI when `one_click` is set. The caller does not POST it here.
    pub one_click_uri: Option<String>,
}

/// Parse the two headers. Either may be absent.
pub fn parse_list_unsubscribe(
    list_unsubscribe: Option<&str>,
    list_unsubscribe_post: Option<&str>,
) -> Unsubscribe {
    let mut https = Vec::new();
    let mut mailto = Vec::new();
    if let Some(header) = list_unsubscribe {
        for uri in uris(header) {
            if starts_with_ignore(&uri, "https://") {
                https.push(uri);
            } else if starts_with_ignore(&uri, "mailto:") {
                mailto.push(uri);
            }
        }
    }
    let post = list_unsubscribe_post.is_some_and(|value| {
        value
            .trim()
            .eq_ignore_ascii_case("List-Unsubscribe=One-Click")
    });
    let one_click = post && !https.is_empty();
    let one_click_uri = if one_click {
        https.first().cloned()
    } else {
        None
    };
    Unsubscribe {
        https,
        mailto,
        one_click,
        one_click_uri,
    }
}

fn uris(header: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = header;
    let mut saw_bracket = false;
    while let Some(start) = rest.find('<') {
        saw_bracket = true;
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else {
            break;
        };
        let uri = after[..end].trim();
        if !uri.is_empty() {
            out.push(uri.to_string());
        }
        rest = &after[end + 1..];
    }
    if saw_bracket {
        return out;
    }
    header
        .split(',')
        .map(str::trim)
        .filter(|uri| !uri.is_empty())
        .map(str::to_string)
        .collect()
}

fn starts_with_ignore(value: &str, prefix: &str) -> bool {
    value.len() >= prefix.len() && value[..prefix.len()].eq_ignore_ascii_case(prefix)
}

#[cfg(test)]
mod tests {
    use super::parse_list_unsubscribe;

    #[test]
    fn one_click_needs_the_post_header_and_an_https_uri() {
        let header = "<mailto:unsub@example.com>, <https://example.com/unsub>";
        let parsed = parse_list_unsubscribe(Some(header), Some("List-Unsubscribe=One-Click"));
        assert!(parsed.one_click);
        assert_eq!(
            parsed.one_click_uri.as_deref(),
            Some("https://example.com/unsub")
        );
        assert_eq!(parsed.mailto, ["mailto:unsub@example.com"]);
        assert_eq!(parsed.https, ["https://example.com/unsub"]);

        let no_post = parse_list_unsubscribe(Some(header), None);
        assert!(!no_post.one_click);
        assert!(no_post.one_click_uri.is_none());
        assert_eq!(no_post.https, ["https://example.com/unsub"]);

        let mailto_only = parse_list_unsubscribe(
            Some("<mailto:unsub@example.com>"),
            Some("list-unsubscribe=one-click"),
        );
        assert!(!mailto_only.one_click);
        assert_eq!(mailto_only.mailto, ["mailto:unsub@example.com"]);

        let other = parse_list_unsubscribe(Some(header), Some("List-Unsubscribe=Two-Click"));
        assert!(!other.one_click);
    }
}
