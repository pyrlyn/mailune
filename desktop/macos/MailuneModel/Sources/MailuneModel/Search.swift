import Foundation

/// One search token, spelled as the core's `parse_query` spells it.
public enum SearchTerm: Equatable, Sendable {
    case from(String)
    case label(String)
    case hasAttachment
    case unread
    case text(String)
}

public enum SearchError: Error, Equatable {
    /// A key the core accepts that a `ThreadRow` cannot answer, so the shell
    /// says so instead of silently ignoring it.
    case unsupported(String)
    /// An unknown key, an empty value, a bad `has:` or `is:` value, or an
    /// unterminated quote: the cases the core rejects as `BadQuery`.
    case bad(String)
}

/// The core's search grammar over the fixture rows until the core is wired.
public struct SearchQuery: Equatable, Sendable {
    public var terms: [SearchTerm]

    public static func parse(_ input: String) throws(SearchError) -> SearchQuery {
        var terms: [SearchTerm] = []
        var rest = Substring(input)
        while true {
            rest = rest.drop(while: \.isWhitespace)
            guard !rest.isEmpty else { break }
            if rest.first == "\"" {
                let (text, after) = try quoted(rest)
                terms.append(.text(text))
                rest = after
                continue
            }
            let keyEnd = rest.firstIndex { $0.isWhitespace || $0 == ":" } ?? rest.endIndex
            guard keyEnd < rest.endIndex, rest[keyEnd] == ":" else {
                terms.append(.text(String(rest[..<keyEnd])))
                rest = rest[keyEnd...]
                continue
            }
            let key = String(rest[..<keyEnd])
            var value = rest[rest.index(after: keyEnd)...]
            let text: String
            if value.first == "\"" {
                (text, value) = try quoted(value)
            } else {
                let end = value.firstIndex(where: \.isWhitespace) ?? value.endIndex
                text = String(value[..<end])
                value = value[end...]
            }
            guard !text.isEmpty else { throw .bad(key) }
            terms.append(try field(key, text))
            rest = value
        }
        return SearchQuery(terms: terms)
    }

    public func matches(_ item: ThreadItem) -> Bool {
        terms.allSatisfy { term in
            switch term {
            case let .from(who):
                item.from.email.localizedCaseInsensitiveContains(who)
                    || (item.from.name?.localizedCaseInsensitiveContains(who) ?? false)
            case let .label(name):
                item.labels.contains { $0.caseInsensitiveCompare(name) == .orderedSame }
            case .hasAttachment:
                item.hasAttachment
            case .unread:
                item.unread
            case let .text(word):
                [item.subject, item.snippet, item.from.display].contains { $0.localizedCaseInsensitiveContains(word) }
            }
        }
    }

    public func narrow(_ items: [ThreadItem]) -> [ThreadItem] {
        items.filter(matches)
    }

    private static func field(_ key: String, _ value: String) throws(SearchError) -> SearchTerm {
        switch (key, value) {
        case ("from", _): .from(value)
        case ("label", _): .label(value)
        case ("has", "attachment"): .hasAttachment
        case ("is", "unread"): .unread
        case ("to", _), ("before", _): throw .unsupported("\(key):\(value)")
        default: throw .bad("\(key):\(value)")
        }
    }

    private static func quoted(_ input: Substring) throws(SearchError) -> (String, Substring) {
        let body = input.dropFirst()
        guard let close = body.firstIndex(of: "\"") else { throw .bad(String(input)) }
        return (String(body[..<close]), body[body.index(after: close)...])
    }
}

/// A message an answer rests on.
public struct Citation: Equatable, Sendable {
    public var message: String
    public var thread: String
}

public struct AskAnswer: Equatable, Sendable {
    public var text: String
    public var citations: [Citation]
}

/// Ask over the fixture messages with no model. Like the core's `ask`, there
/// is no answer without a citation of a message it was given.
public enum FixtureAsk {
    /// The sentence that shares the most words of four or more letters with
    /// the question. Shorter words are too common to tie a sentence to it.
    public static func answer(_ question: String, from messages: [MailMessage]) -> AskAnswer? {
        let asked = words(question)
        guard !asked.isEmpty else { return nil }
        var best: (score: Int, sentence: String, message: MailMessage)?
        for message in messages {
            for sentence in message.body.split(whereSeparator: { ".?!\n".contains($0) }) {
                let score = words(String(sentence)).intersection(asked).count
                if score > (best?.score ?? 0) {
                    best = (score, sentence.trimmingCharacters(in: .whitespaces), message)
                }
            }
        }
        guard let best else { return nil }
        return AskAnswer(
            text: best.sentence,
            citations: [Citation(message: best.message.id, thread: best.message.thread)]
        )
    }

    private static func words(_ text: String) -> Set<String> {
        Set(text.lowercased().split { !$0.isLetter }.map(String.init).filter { $0.count >= 4 })
    }
}
