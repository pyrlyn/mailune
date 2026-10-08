import Foundation

/// Strings from the catalogs in this module. A key missing from the requested
/// catalog comes from English.
public enum Copy {
    /// The inbox title in the requested catalog, or English when that catalog has no row.
    public static func text(_ key: String, language: String) -> String {
        if let value = catalog(language)[key] {
            return value
        }
        return catalog("en")[key] ?? key
    }

    static func catalog(_ language: String) -> [String: String] {
        guard let url = Bundle.module.url(
            forResource: "Localizable",
            withExtension: "strings",
            subdirectory: "\(language).lproj"
        ), let text = try? String(contentsOf: url, encoding: .utf8) else {
            return [:]
        }
        return parse(text)
    }

    /// `"key" = "value";` lines. Catalogs are written by hand in that form.
    private static func parse(_ text: String) -> [String: String] {
        var entries: [String: String] = [:]
        for line in text.split(separator: "\n") {
            let parts = line.split(separator: "=", maxSplits: 1)
            guard parts.count == 2 else { continue }
            let key = parts[0].trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "\""))
            let raw = parts[1].trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "\";"))
            entries[key] = raw
        }
        return entries
    }
}
