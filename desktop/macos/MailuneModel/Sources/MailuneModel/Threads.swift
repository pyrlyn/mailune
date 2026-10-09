import Foundation

/// Triage tab, spelled as the contract's `Category`.
public enum ThreadCategory: String, CaseIterable, Decodable, Sendable {
    case primary, social, promotions, updates

    public var titleKey: String { "data.\(rawValue)" }
}

/// One row in the thread list: the fields of the contract's `ThreadRow` the
/// list shows. The rest of the row is ignored when decoding.
public struct ThreadItem: Identifiable, Equatable, Sendable, Decodable {
    public struct Sender: Equatable, Sendable, Decodable {
        public var name: String?
        public var email: String

        public var display: String { name ?? email }
    }

    public var id: String
    public var from: Sender
    public var subject: String
    public var snippet: String
    public var stamp: String
    public var unread: Bool
    public var hasAttachment: Bool
    public var category: ThreadCategory
}

/// Rows until the core is wired. `threads.json` is a contract `Event::Snapshot`;
/// a testkit test parses the same file, so it cannot drift from the contract.
public enum ThreadFixtures {
    public static let all: [ThreadItem] = load()

    /// Rows on one tab, minus anything the swipe action archived.
    public static func visible(category: ThreadCategory, archived: Set<String>) -> [ThreadItem] {
        all.filter { $0.category == category && !archived.contains($0.id) }
    }

    static func decode(_ data: Data) throws -> [ThreadItem] {
        struct Event: Decodable {
            struct Snapshot: Decodable {
                var threads: [ThreadItem]
            }

            var snapshot: Snapshot
        }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(Event.self, from: data).snapshot.threads
    }

    private static func load() -> [ThreadItem] {
        guard let url = Bundle.module.url(forResource: "threads", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let rows = try? decode(data)
        else {
            return []
        }
        return rows
    }
}
