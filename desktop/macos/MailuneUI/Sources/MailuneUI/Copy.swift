import Foundation

/// Strings from the catalogs generated out of `i18n/`. Keys are the msgctxt of
/// each message. Foundation returns the key itself when a language's table
/// lacks it, so the English fallback is done here.
public enum Copy {
    static let base = "en"

    /// The text for `key` in `language`, or in the user's language when nil.
    public static func text(_ key: String, language: String? = nil) -> String {
        text(key, language: language ?? preferredLanguage, in: .module)
    }

    /// `{0}`, `{1}` in the catalog arrive as `%1$@`, `%2$@`.
    public static func format(_ key: String, _ arguments: String..., language: String? = nil) -> String {
        String(format: text(key, language: language), arguments: arguments)
    }

    static func text(_ key: String, language: String, in bundle: Bundle) -> String {
        for candidate in [language, base] {
            if let value = lookup(key, language: candidate, in: bundle) {
                return value
            }
        }
        return key
    }

    static var preferredLanguage: String {
        Bundle.preferredLocalizations(from: Bundle.module.localizations).first ?? base
    }

    private static func lookup(_ key: String, language: String, in bundle: Bundle) -> String? {
        guard let path = bundle.path(forResource: language, ofType: "lproj"),
              let table = Bundle(path: path)
        else {
            return nil
        }
        // A value no catalog can contain, so a miss is told apart from a hit.
        let missing = "\u{1}"
        let value = table.localizedString(forKey: key, value: missing, table: nil)
        return value == missing ? nil : value
    }
}
