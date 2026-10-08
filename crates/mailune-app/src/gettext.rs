//! Gettext-style catalogs for the shell.
//!
//! The msgid is the English source string. A missing entry, or a language
//! this module does not ship, returns that msgid. There is no extraction tool.

const EN: &[(&str, &str)] = &[
    ("Inbox", "Inbox"),
    ("Settings", "Settings"),
    ("Compose", "Compose"),
    ("Send", "Send"),
];

const RU: &[(&str, &str)] = &[
    ("Inbox", "Входящие"),
    ("Settings", "Настройки"),
    ("Compose", "Написать"),
    ("Send", "Отправить"),
];

const DE: &[(&str, &str)] = &[
    ("Inbox", "Posteingang"),
    ("Settings", "Einstellungen"),
    ("Compose", "Schreiben"),
    ("Send", "Senden"),
];

const FR: &[(&str, &str)] = &[
    ("Inbox", "Boîte de réception"),
    ("Settings", "Réglages"),
    ("Compose", "Rédiger"),
    ("Send", "Envoyer"),
];

const JA: &[(&str, &str)] = &[
    ("Inbox", "受信トレイ"),
    ("Settings", "設定"),
    ("Compose", "作成"),
    ("Send", "送信"),
];

/// Loads `language` and returns the translation of `msgid`.
///
/// `en`, `ru`, `de`, `fr`, and `ja` have catalogs. Any other code, and any
/// msgid those catalogs do not list, yields `msgid` unchanged.
#[must_use]
pub fn gettext(language: &str, msgid: &str) -> String {
    let catalog = match language {
        "en" => EN,
        "ru" => RU,
        "de" => DE,
        "fr" => FR,
        "ja" => JA,
        _ => &[],
    };
    catalog
        .iter()
        .find(|(id, _)| *id == msgid)
        .map(|(_, text)| (*text).to_string())
        .unwrap_or_else(|| msgid.to_string())
}

#[cfg(test)]
mod tests {
    use super::gettext;

    #[test]
    fn catalogs_load_and_a_missing_key_is_the_msgid() {
        assert_eq!(gettext("en", "Inbox"), "Inbox");
        assert_eq!(gettext("ru", "Inbox"), "Входящие");
        assert_eq!(gettext("de", "Settings"), "Einstellungen");
        assert_eq!(gettext("fr", "Compose"), "Rédiger");
        assert_eq!(gettext("ja", "Send"), "送信");
        assert_eq!(gettext("ru", "Not a catalog key"), "Not a catalog key");
        assert_eq!(gettext("zz", "Inbox"), "Inbox");
    }
}
