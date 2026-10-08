//! Language labels from a script and function-word table.
//!
//! This workspace does not ship a language-detection crate, so the label is a
//! count of closed-class words plus a script majority. There is no translation
//! call and no network.

/// A language label. `code` is `und` when the text is not enough to choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// BCP 47 primary language subtag.
    pub code: &'static str,
}

struct Lexicon {
    code: &'static str,
    words: &'static [&'static str],
}

const LATIN: &[Lexicon] = &[
    Lexicon {
        code: "en",
        words: &["the", "and", "you", "with", "this", "have", "from", "that"],
    },
    Lexicon {
        code: "de",
        words: &["und", "der", "die", "das", "nicht", "ich", "ein", "ist"],
    },
    Lexicon {
        code: "fr",
        words: &[
            "vous", "bonjour", "merci", "avec", "pour", "dans", "nous", "cette",
        ],
    },
    Lexicon {
        code: "es",
        words: &[
            "hola", "gracias", "usted", "porque", "también", "está", "pero", "donde",
        ],
    },
];

struct ScriptCount {
    cyrillic: u32,
    kana: u32,
    hangul: u32,
    cjk: u32,
    arabic: u32,
    latin: u32,
}

/// Labels `text`. A tie, or text with no letters, is `und`.
#[must_use]
pub fn detect(text: &str) -> Language {
    let code = script_language(text)
        .or_else(|| latin_language(text))
        .unwrap_or("und");
    Language { code }
}

fn script_language(text: &str) -> Option<&'static str> {
    let mut counts = ScriptCount {
        cyrillic: 0,
        kana: 0,
        hangul: 0,
        cjk: 0,
        arabic: 0,
        latin: 0,
    };
    for character in text.chars() {
        if !character.is_alphabetic() {
            continue;
        }
        match character {
            '\u{0400}'..='\u{04FF}' => counts.cyrillic = counts.cyrillic.saturating_add(1),
            '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' => {
                counts.kana = counts.kana.saturating_add(1);
            }
            '\u{AC00}'..='\u{D7AF}' => counts.hangul = counts.hangul.saturating_add(1),
            '\u{4E00}'..='\u{9FFF}' => counts.cjk = counts.cjk.saturating_add(1),
            '\u{0600}'..='\u{06FF}' => counts.arabic = counts.arabic.saturating_add(1),
            'A'..='Z' | 'a'..='z' | '\u{00C0}'..='\u{024F}' => {
                counts.latin = counts.latin.saturating_add(1);
            }
            _ => {}
        }
    }
    // Kana marks Japanese even when most of the sentence is kanji.
    if counts.kana > 0
        && counts.kana >= counts.hangul
        && counts.kana >= counts.cyrillic
        && counts.kana >= counts.arabic
    {
        return Some("ja");
    }
    let ranked = [
        ("ru", counts.cyrillic),
        ("ko", counts.hangul),
        ("zh", counts.cjk),
        ("ar", counts.arabic),
    ];
    let mut best_code: Option<&str> = None;
    let mut best = 0u32;
    for (code, count) in ranked {
        if count > best {
            best_code = Some(code);
            best = count;
        }
    }
    if best == 0 || best < counts.latin {
        None
    } else {
        best_code
    }
}

fn latin_language(text: &str) -> Option<&'static str> {
    let tokens = tokens(text);
    let mut best_code: Option<&str> = None;
    let mut best_score = 0u32;
    let mut tied = false;
    for lexicon in LATIN {
        let score = word_score(&tokens, lexicon.words);
        if score == 0 {
            continue;
        }
        if score > best_score {
            best_code = Some(lexicon.code);
            best_score = score;
            tied = false;
        } else if score == best_score {
            // A tie is not evidence, so the label stays undetermined.
            tied = true;
        }
    }
    if tied { None } else { best_code }
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn word_score(tokens: &[String], words: &[&str]) -> u32 {
    let mut score = 0u32;
    for token in tokens {
        if words.contains(&token.as_str()) {
            score = score.saturating_add(1);
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::detect;

    #[test]
    fn a_message_is_labelled_with_a_language() {
        assert_eq!(
            detect("The meeting is tomorrow and we should review this with you").code,
            "en"
        );
        assert_eq!(
            detect("Das Treffen ist morgen und ich kann nicht kommen").code,
            "de"
        );
        assert_eq!(
            detect("Bonjour, merci pour votre aide et nous vous ecrivons").code,
            "fr"
        );
        assert_eq!(
            detect("Hola, gracias porque usted está aquí pero donde").code,
            "es"
        );
        assert_eq!(detect("Встреча завтра").code, "ru");
        assert_eq!(detect("会議は明日です").code, "ja");
        assert_eq!(detect("会议在明天").code, "zh");
        assert_eq!(detect("안녕하세요").code, "ko");
        assert_eq!(detect("مرحبا بك").code, "ar");
        assert_eq!(detect("12345").code, "und");
        assert_eq!(detect("").code, "und");
    }
}
