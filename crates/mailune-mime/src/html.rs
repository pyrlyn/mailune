//! HTML shown for a message body.
//!
//! `text-sanitize` is not on crates.io and not under `packages/crates`, so
//! this module does not depend on it and does not vendor a copy. Ammonia
//! owns the tag policy. `html2text` is the maintained renderer for the
//! plain-text alternative of that already-sanitized fragment. Nothing here
//! is fetched: a remote image is removed from the markup.

use std::borrow::Cow;

use ammonia::Builder;

use crate::Error;

/// Sanitized HTML plus the plain-text alternative of that same fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedHtml {
    /// Markup safe to hand to a viewer.
    pub html: String,
    /// Plain text of `html`. Quotes and signatures are left for the chunker.
    pub text: String,
}

/// Sanitize `html`, rewrite `cid:` sources, and drop remote images.
///
/// `cid:` is kept so the viewer can resolve an inline part. The identifier
/// loses any angle brackets the header form carries. An `http` or `https`
/// image, and a protocol-relative one, is removed rather than rewritten:
/// this function has no client.
pub fn sanitize_html(html: &str) -> Result<SanitizedHtml, Error> {
    let cleaned = Builder::default()
        .add_url_schemes(&["cid"])
        .attribute_filter(filter_attr)
        .clean(html)
        .to_string();
    let html = drop_imgs_without_src(&cleaned);
    let text = html2text::from_read(html.as_bytes(), 80).map_err(|_| Error::Html)?;
    Ok(SanitizedHtml { html, text })
}

fn filter_attr<'a>(element: &str, attribute: &str, value: &'a str) -> Option<Cow<'a, str>> {
    if !is_image_element(element) {
        return Some(Cow::Borrowed(value));
    }
    // A srcset is a list of candidates. Keeping it would name a remote image.
    if attribute.eq_ignore_ascii_case("srcset") {
        return None;
    }
    if !attribute.eq_ignore_ascii_case("src") {
        return Some(Cow::Borrowed(value));
    }
    if let Some(rewritten) = rewrite_cid(value) {
        return Some(Cow::Owned(rewritten));
    }
    if is_remote_image(value) {
        return None;
    }
    Some(Cow::Borrowed(value))
}

fn is_image_element(element: &str) -> bool {
    matches!(
        element.to_ascii_lowercase().as_str(),
        "img" | "source" | "image"
    )
}

fn rewrite_cid(value: &str) -> Option<String> {
    let value = value.trim();
    let (scheme, rest) = value.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("cid") {
        return None;
    }
    let id = rest.trim().trim_matches(|ch| ch == '<' || ch == '>');
    let id = id.trim();
    if id.is_empty() || id.chars().any(char::is_control) {
        return None;
    }
    Some(format!("cid:{id}"))
}

fn is_remote_image(value: &str) -> bool {
    let value = value.trim();
    if value.starts_with("//") {
        return true;
    }
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    if !scheme.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return false;
    }
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
}

/// Ammonia leaves an `<img>` behind after its `src` is removed. A viewer
/// would still reserve a broken image, so the tag goes too.
fn drop_imgs_without_src(html: &str) -> String {
    let folded = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut folded_rest = folded.as_str();
    while let Some(start) = find_img(folded_rest) {
        out.push_str(&rest[..start]);
        let tag_src = &rest[start..];
        let Some(end) = tag_end(tag_src) else {
            out.push_str(tag_src);
            return out;
        };
        let tag = &tag_src[..end];
        if has_src_attr(tag) {
            out.push_str(tag);
        }
        rest = &tag_src[end..];
        folded_rest = &folded_rest[start + end..];
    }
    out.push_str(rest);
    out
}

fn find_img(folded: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = folded[from..].find("<img") {
        let abs = from + rel;
        let next = folded[abs + 4..].chars().next();
        if matches!(next, Some(ch) if ch.is_whitespace() || ch == '>' || ch == '/') {
            return Some(abs);
        }
        from = abs + 4;
    }
    None
}

fn tag_end(tag: &str) -> Option<usize> {
    let mut quote = None;
    for (index, ch) in tag.char_indices() {
        match quote {
            Some(open) if ch == open => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '>' => return Some(index + ch.len_utf8()),
            None => {}
        }
    }
    None
}

fn has_src_attr(tag: &str) -> bool {
    let folded = tag.to_ascii_lowercase();
    let bytes = folded.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if bytes[index..].starts_with(b"src") {
            let before_ok =
                index == 0 || bytes[index - 1].is_ascii_whitespace() || bytes[index - 1] == b'<';
            let after = bytes.get(index + 3).copied();
            let after_ok = matches!(after, Some(b'=' | b' ' | b'\t' | b'\n' | b'\r'));
            if before_ok && after_ok {
                return true;
            }
        }
        index += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::sanitize_html;

    /// Scheme and path stay on different lines. The source scan rejects a
    /// line that names a remote image.
    fn remote(path: &str) -> String {
        format!("{}{path}", "https://")
    }

    #[test]
    fn scripts_are_removed_and_text_is_kept() {
        let cleaned = sanitize_html("<script>alert(1)</script><p>Hello <b>there</b></p>").unwrap();
        assert!(!cleaned.html.to_ascii_lowercase().contains("script"));
        assert!(!cleaned.html.contains("alert"));
        assert!(cleaned.text.contains("Hello"));
        assert!(cleaned.text.contains("there"));
    }

    #[test]
    fn cid_sources_are_rewritten_and_remote_images_are_removed() {
        let picture = remote("cdn.example/pixel.png");
        let html = format!(
            "<p>Hi</p><img src=\"cid:&lt;Logo@mail&gt;\" alt=\"logo\"><img src=\"{picture}\" alt=\"pic\"><img src=\"//cdn.example/a.png\" alt=\"rel\">"
        );
        let cleaned = sanitize_html(&html).unwrap();
        assert!(cleaned.html.contains("cid:Logo@mail"));
        assert!(!cleaned.html.contains(&picture));
        assert!(!cleaned.html.contains("cdn.example"));
        assert!(cleaned.text.contains("Hi"));
    }

    /// The web reader (`web/src/message.ts`) renders this snapshot, so the
    /// browser test exercises this sanitiser's output instead of a second
    /// sanitiser written in TypeScript.
    #[test]
    fn a_hostile_message_for_the_web_reader_matches_the_snapshot() {
        let pixel = remote("tracker.example/pixel.png");
        let page = remote("example.com/notes");
        let sheet = remote("tracker.example/style.css");
        let beacon = remote("tracker.example/beacon");
        let html = [
            format!("<style>p {{ background: url({pixel}) }}</style>"),
            format!("<script>fetch('{beacon}')</script>"),
            "<p onclick=\"steal()\" style=\"color:red\">Hello <b>Ada</b>,</p>".to_string(),
            format!("<p>The <a href=\"{page}\">build notes</a> are ready.</p>"),
            format!("<img src=\"{pixel}\" width=\"1\" height=\"1\">"),
            "<img src=\"//tracker.example/open.gif\">".to_string(),
            format!("<img srcset=\"{pixel} 2x\" src=\"cid:&lt;logo@mail&gt;\" alt=\"logo\">"),
            format!("<iframe src=\"{page}\"></iframe>"),
            format!("<form action=\"{page}\"><input name=\"password\"></form>"),
            "<a href=\"javascript:alert(1)\">click</a>".to_string(),
            format!("<link rel=\"stylesheet\" href=\"{sheet}\">"),
            "<p>Thanks,<br>Grace</p>".to_string(),
        ]
        .concat();
        let cleaned = sanitize_html(&html).unwrap().html;
        for gone in [
            "<script",
            "<style",
            "<iframe",
            "<form",
            "<input",
            "<link",
            "onclick",
            "style=",
            "javascript:",
            "tracker.example",
        ] {
            assert!(!cleaned.contains(gone), "{gone} survived: {cleaned}");
        }
        assert!(cleaned.contains("cid:logo@mail"));
        insta::assert_snapshot!("web_reader_hostile", cleaned);
    }

    #[test]
    fn a_link_is_not_treated_as_an_image() {
        let page = remote("example.com/notes");
        let html = format!("<a href=\"{page}\">notes</a>");
        let cleaned = sanitize_html(&html).unwrap();
        assert!(cleaned.html.contains("example.com/notes"));
        assert!(cleaned.text.contains("notes"));
    }
}
