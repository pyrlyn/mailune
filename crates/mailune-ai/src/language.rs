//! The language a message is written in, detected on the device by
//! whatlang's trigram model. Nothing is translated and nothing leaves the
//! device, so this runs for encrypted mail too.

use whatlang::detect;

use crate::{MailText, redact_for_cloud};

/// The language a message is labelled with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LanguageLabel {
    /// ISO 639-3 code, such as `eng` or `rus`.
    pub code: &'static str,
    /// English name of the language.
    pub name: &'static str,
    /// Detector confidence from 0 to 1.
    pub confidence: f64,
}

/// The language of `text`, or `None` when the detector is not reliable:
/// a short "ok" or a mix of languages gets no label rather than a guess.
pub fn detect_language(text: &str) -> Option<LanguageLabel> {
    let info = detect(text)?;
    info.is_reliable().then(|| LanguageLabel {
        code: info.lang().code(),
        name: info.lang().eng_name(),
        confidence: info.confidence(),
    })
}

/// The language `message` is written in. Quoted lines and the signature
/// are left out, so a reply is not labelled with the language it quotes.
pub fn label_language(message: &MailText) -> Option<LanguageLabel> {
    detect_language(&format!(
        "{}\n{}",
        message.subject,
        redact_for_cloud(&message.body)
    ))
}

#[cfg(test)]
mod tests {
    use super::{detect_language, label_language};
    use crate::summary::tests::mail;

    #[test]
    fn a_message_is_labelled_with_its_language() {
        let samples = [
            (
                "eng",
                "Could we move the launch review to Friday afternoon? The team needs one more day to finish testing.",
            ),
            (
                "rus",
                "Можем ли мы перенести обсуждение запуска на пятницу? Команде нужен ещё один день для тестирования.",
            ),
            (
                "deu",
                "Können wir die Besprechung auf Freitag verschieben? Das Team braucht noch einen Tag für die Tests.",
            ),
        ];
        for (code, body) in samples {
            let label = label_language(&mail("m1", 1, body)).unwrap();
            assert_eq!(label.code, code, "{body}");
            assert!(label.confidence > 0.5);
        }
        assert_eq!(detect_language(samples[0].1).unwrap().name, "English");
    }

    #[test]
    fn quoted_text_does_not_decide_and_short_text_gets_no_label() {
        let reply = "Ja, Freitag passt mir gut. Ich schicke heute Abend die Einladung \
                     und morgen früh die Tagesordnung.\n\n\
                     > Could we move the launch review to Friday afternoon? The team needs one\n\
                     > more day to finish testing, and the vendor has not confirmed the date.\n\
                     > Please let me know which slot works for everyone on your side.\n";
        let mut message = mail("m2", 2, reply);
        message.subject = "Re:".into();
        assert_eq!(label_language(&message).unwrap().code, "deu");
        assert!(detect_language("ok").is_none());
        assert!(detect_language("").is_none());
    }
}
