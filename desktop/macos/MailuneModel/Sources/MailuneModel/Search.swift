import Foundation

/// One filter chip. Several tokens narrow the fixture list together.
public struct SearchToken: Equatable, Sendable, Identifiable {
    public var id: String
    public var field: String
    public var value: String

    public init(id: String, field: String, value: String) {
        self.id = id
        self.field = field
        self.value = value
    }
}

/// A pointer back at the fixture a question was answered from.
public struct Citation: Equatable, Sendable {
    public var fixtureID: String
    public var excerpt: String

    public init(fixtureID: String, excerpt: String) {
        self.fixtureID = fixtureID
        self.excerpt = excerpt
    }
}

public enum ThreadSearch {
    /// Keeps rows that match every token. An empty token list returns the whole list.
    public static func narrow(_ items: [ThreadItem], tokens: [SearchToken]) -> [ThreadItem] {
        items.filter { item in
            tokens.allSatisfy { token in
                switch token.field {
                case "subject":
                    item.subject.localizedCaseInsensitiveContains(token.value)
                case "category":
                    item.category.compare(token.value, options: .caseInsensitive) == .orderedSame
                default:
                    false
                }
            }
        }
    }

    /// The first fixture whose subject appears in the question.
    public static func cite(question: String, in items: [ThreadItem]) -> Citation? {
        let folded = question.folding(options: .caseInsensitive, locale: .current)
        for item in items where folded.contains(item.subject.folding(options: .caseInsensitive, locale: .current)) {
            return Citation(fixtureID: item.id, excerpt: item.subject)
        }
        return nil
    }
}
