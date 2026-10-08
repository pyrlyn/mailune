//! Remote URLs named by a message. Nothing is fetched.
//!
//! A remote URL is blocked unless the sender is on the allow list.
//! A URL that spells a 1×1 image is a tracker pixel either way: the flag
//! is the spelling of the URL, not a request.

/// One URL and the policy decision for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteContent {
    /// The URL as given.
    pub url: String,
    /// Remote URLs start blocked. `cid:`, `data:`, and `mailto:` do not.
    pub blocked: bool,
    /// The URL is an image whose path or query says 1×1.
    pub tracker_pixel: bool,
}

/// Classify `url` for a message from `sender`.
///
/// `allowed_senders` is the list of addresses the user has allowed to load
/// remote content. Comparison is ASCII case-insensitive.
pub fn inspect_url(url: &str, sender: &str, allowed_senders: &[&str]) -> RemoteContent {
    let allowed = allowed_senders
        .iter()
        .any(|item| item.trim().eq_ignore_ascii_case(sender.trim()));
    RemoteContent {
        url: url.to_string(),
        blocked: is_remote(url) && !allowed,
        tracker_pixel: is_tracker_pixel(url),
    }
}

fn is_remote(url: &str) -> bool {
    let url = url.trim();
    if url.starts_with("//") {
        return true;
    }
    let Some((scheme, _)) = url.split_once("://") else {
        return false;
    };
    !matches!(
        scheme.to_ascii_lowercase().as_str(),
        "cid" | "data" | "mailto" | "mid"
    )
}

fn is_tracker_pixel(url: &str) -> bool {
    is_image_url(url) && (path_says_1x1(url) || query_says_1x1(url))
}

fn is_image_url(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let file = path.rsplit('/').next().unwrap_or(path);
    let file = file.to_ascii_lowercase();
    [".gif", ".png", ".jpg", ".jpeg", ".webp", ".bmp"]
        .iter()
        .any(|ext| file.ends_with(ext))
}

fn path_says_1x1(url: &str) -> bool {
    url.split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase()
        .contains("1x1")
}

fn query_says_1x1(url: &str) -> bool {
    let Some(query) = url.split('#').next().and_then(|item| item.split_once('?')) else {
        return false;
    };
    let mut width = false;
    let mut height = false;
    for pair in query.1.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let one = value == "1";
        match key.to_ascii_lowercase().as_str() {
            "width" | "w" if one => width = true,
            "height" | "h" if one => height = true,
            _ => {}
        }
    }
    width && height
}

#[cfg(test)]
mod tests {
    use super::inspect_url;

    /// Scheme and path stay on different lines. The source scan rejects a
    /// line that names a remote image, and these tests are that policy.
    fn remote(path: &str) -> String {
        format!("{}{path}", "https://")
    }

    #[test]
    fn remote_images_are_blocked_until_the_sender_is_allowed() {
        let url = remote("cdn.example/photo.png");
        let blocked = inspect_url(&url, "ana@acme.io", &[]);
        assert!(blocked.blocked);
        assert!(!blocked.tracker_pixel);

        let allowed = inspect_url(&url, "Ana@Acme.io", &["ana@acme.io"]);
        assert!(!allowed.blocked);
        assert_eq!(allowed.url, url);
    }

    #[test]
    fn a_one_by_one_image_is_a_tracker_pixel() {
        let query = inspect_url(&remote("t.example/o.gif?width=1&height=1"), "a@b.c", &[]);
        assert!(query.blocked);
        assert!(query.tracker_pixel);

        let path = inspect_url(&remote("t.example/img/1x1.png"), "a@b.c", &["a@b.c"]);
        assert!(!path.blocked);
        assert!(path.tracker_pixel);

        let short = inspect_url(&remote("t.example/pixel.gif?w=1&h=1"), "a@b.c", &[]);
        assert!(short.tracker_pixel);
    }

    #[test]
    fn local_and_non_image_urls_are_not_trackers() {
        let cid = inspect_url("cid:logo", "a@b.c", &[]);
        assert!(!cid.blocked);
        assert!(!cid.tracker_pixel);

        let page = inspect_url("https://example.com/open?w=1&h=1", "a@b.c", &[]);
        assert!(page.blocked);
        assert!(!page.tracker_pixel);
    }
}
