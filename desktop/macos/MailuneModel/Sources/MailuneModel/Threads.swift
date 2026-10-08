import Foundation

/// One row in the thread list. Fixture data, not a live mailbox.
public struct ThreadItem: Identifiable, Equatable, Sendable {
    public var id: String
    public var subject: String
    public var unread: Bool
    public var category: String

    public init(id: String, subject: String, unread: Bool, category: String) {
        self.id = id
        self.subject = subject
        self.unread = unread
        self.category = category
    }
}

/// The rows and tabs the list shows until a real mailbox is wired.
public enum ThreadFixtures {
    public static let categories = ["Primary", "Updates"]

    public static let all: [ThreadItem] = [
        ThreadItem(id: "1", subject: "Hello", unread: true, category: "Primary"),
        ThreadItem(id: "2", subject: "Notes", unread: false, category: "Primary"),
        ThreadItem(id: "3", subject: "Receipt", unread: true, category: "Updates"),
    ]

    /// Rows on one tab, minus anything the swipe action archived.
    public static func visible(category: String, archived: Set<String>) -> [ThreadItem] {
        all.filter { $0.category == category && !archived.contains($0.id) }
    }
}
