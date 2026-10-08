//! Cut plain text into windows a later reader can embed.
//!
//! A token is one whitespace-separated word. Lines that quote an earlier
//! message, and a signature after a `--` delimiter, are left out: they are
//! not the words the sender wrote. A window holds 400 to 600 tokens. A
//! message shorter than that stays one window. A remainder that cannot join
//! the previous window without passing 600 stays a short tail. The span is
//! the byte range in the original text from the first kept word to the last.

const MIN_TOKENS: usize = 400;
const MAX_TOKENS: usize = 600;
const TARGET_TOKENS: usize = 500;

/// One window and where it sat in the original text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextChunk {
    /// Kept words, separated by a single space.
    pub text: String,
    /// Byte offset of the first kept word.
    pub start: usize,
    /// Byte offset just past the last kept word.
    pub end: usize,
}

/// Windows for `text`. No words means no windows.
pub fn chunk_plain(text: &str) -> Vec<TextChunk> {
    let words = kept_words(text);
    if words.is_empty() {
        return Vec::new();
    }
    windows(&words)
        .into_iter()
        .filter_map(|range| {
            let slice = words.get(range.clone())?;
            let first = slice.first()?;
            let last = slice.last()?;
            let text = slice
                .iter()
                .map(|(start, end)| text.get(*start..*end).unwrap_or(""))
                .collect::<Vec<_>>()
                .join(" ");
            Some(TextChunk {
                text,
                start: first.0,
                end: last.1,
            })
        })
        .collect()
}

fn kept_words(text: &str) -> Vec<(usize, usize)> {
    let mut words = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let trimmed = body.trim();
        if trimmed == "--" || trimmed == "-- " {
            break;
        }
        if !trimmed.starts_with('>') {
            words.extend(tokens(text, offset, offset + body.len()));
        }
        offset += line.len();
    }
    words
}

fn tokens(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let Some(slice) = text.get(start..end) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut token_at: Option<usize> = None;
    for (index, ch) in slice.char_indices() {
        if ch.is_whitespace() {
            if let Some(begin) = token_at.take() {
                found.push((start + begin, start + index));
            }
        } else if token_at.is_none() {
            token_at = Some(index);
        }
    }
    if let Some(begin) = token_at {
        found.push((start + begin, start + slice.len()));
    }
    found
}

fn windows(words: &[(usize, usize)]) -> Vec<std::ops::Range<usize>> {
    let mut spans: Vec<std::ops::Range<usize>> = Vec::new();
    if words.len() <= MAX_TOKENS {
        spans.push(0..words.len());
        return spans;
    }
    let mut index = 0;
    while index < words.len() {
        let left = words.len() - index;
        if left <= MAX_TOKENS {
            if left < MIN_TOKENS
                && let Some(previous) = spans.last_mut()
            {
                let previous_len = previous.end - previous.start;
                if previous_len + left <= MAX_TOKENS {
                    previous.end = words.len();
                    break;
                }
            }
            spans.push(index..words.len());
            break;
        }
        spans.push(index..index + TARGET_TOKENS);
        index += TARGET_TOKENS;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_one_window_without_quotes_or_signature() {
        let text = "Hello there\n> older line\n-- \nSent from a phone\n";
        let chunks = chunk_plain(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello there");
        assert_eq!(&text[chunks[0].start..chunks[0].end], "Hello there");
        assert!(chunk_plain("").is_empty());
        assert!(chunk_plain("> only a quote\n").is_empty());
    }

    #[test]
    fn long_text_splits_inside_the_token_window() {
        let words = (0..700)
            .map(|index| format!("w{index}"))
            .collect::<Vec<_>>();
        let text = words.join(" ");
        let chunks = chunk_plain(&text);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text.split_whitespace().count(), 500);
        assert_eq!(chunks[1].text.split_whitespace().count(), 200);
        assert_eq!(&text[chunks[0].start..chunks[0].end], chunks[0].text);
        assert!(chunks[0].end <= chunks[1].start);
    }
}
