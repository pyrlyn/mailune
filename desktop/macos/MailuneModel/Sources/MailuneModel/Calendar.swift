import Foundation

/// One local ICS suggestion. The view shows these; it does not open a calendar account.
public struct CalendarSuggestion: Equatable, Sendable, Identifiable {
    public var id: String
    public var summary: String
    public var start: String
    public var location: String

    public init(id: String, summary: String, start: String, location: String) {
        self.id = id
        self.summary = summary
        self.start = start
        self.location = location
    }
}

/// Reads a bundled ICS fixture. No network and no JMAP calendar source.
public enum CalendarSuggestions {
    public static func load() -> [CalendarSuggestion] {
        guard
            let url = Bundle.module.url(forResource: "suggestions", withExtension: "ics"),
            let text = try? String(contentsOf: url, encoding: .utf8)
        else {
            return []
        }
        return parse(text)
    }

    public static func parse(_ text: String) -> [CalendarSuggestion] {
        var suggestions: [CalendarSuggestion] = []
        for part in unfold(text).components(separatedBy: "BEGIN:VEVENT").dropFirst() {
            let body = part.components(separatedBy: "END:VEVENT").first ?? ""
            var summary = ""
            var start = ""
            var location = ""
            var uid = ""
            for line in body.split(whereSeparator: \.isNewline) {
                guard let (name, value) = field(String(line)) else { continue }
                switch name {
                case "SUMMARY":
                    summary = value
                case "DTSTART":
                    start = stamp(value) ?? ""
                case "LOCATION":
                    location = value
                case "UID":
                    uid = value
                default:
                    break
                }
            }
            guard !summary.isEmpty, !start.isEmpty else { continue }
            suggestions.append(
                CalendarSuggestion(
                    id: uid.isEmpty ? summary : uid,
                    summary: summary,
                    start: start,
                    location: location
                )
            )
        }
        return suggestions
    }

    private static func unfold(_ text: String) -> String {
        var lines: [String] = []
        for lineSub in text.split(whereSeparator: \.isNewline) {
            let line = String(lineSub)
            if line.hasPrefix(" ") || line.hasPrefix("\t"), let last = lines.popLast() {
                lines.append(last + line.dropFirst())
            } else {
                lines.append(line)
            }
        }
        return lines.joined(separator: "\n")
    }

    private static func field(_ line: String) -> (String, String)? {
        guard let colon = line.firstIndex(of: ":") else { return nil }
        let rawName = line[..<colon]
        let name = String(rawName.split(separator: ";").first ?? Substring(rawName))
        let value = String(line[line.index(after: colon)...]).trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty, !value.isEmpty else { return nil }
        return (name.uppercased(), value)
    }

    private static func stamp(_ value: String) -> String? {
        let digits = value.filter(\.isNumber)
        guard digits.count >= 8 else { return nil }
        let chars = Array(digits)
        func take(_ start: Int, _ count: Int) -> String {
            String(chars[start..<(start + count)])
        }
        let day = "\(take(0, 4))-\(take(4, 2))-\(take(6, 2))"
        guard digits.count >= 12 else { return day }
        let zone = value.hasSuffix("Z") ? " UTC" : ""
        return "\(day) \(take(8, 2)):\(take(10, 2))\(zone)"
    }
}
