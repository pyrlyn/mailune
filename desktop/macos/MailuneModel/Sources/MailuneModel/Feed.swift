import Foundation
import Observation

/// Where thread rows come from. The core's change feed replaces the fixture
/// source once the binding exists.
public protocol ThreadSource: Sendable {
    func fetch() async throws -> [ThreadItem]
}

public struct FixtureThreadSource: ThreadSource {
    public init() {}

    public func fetch() async throws -> [ThreadItem] {
        ThreadFixtures.all
    }
}

/// The rows both shells list, with the archive and refresh rules in one place.
@MainActor
@Observable
public final class ThreadFeed {
    public private(set) var rows: [ThreadItem]
    public private(set) var archived: Set<String> = []
    /// The last refresh failed; the rows from before it are still shown.
    public private(set) var refreshFailed = false
    private let source: ThreadSource

    public init(source: ThreadSource = FixtureThreadSource(), rows: [ThreadItem] = ThreadFixtures.all) {
        self.source = source
        self.rows = rows
    }

    /// Rows on one tab, or on every tab when `category` is nil, minus archived ones.
    public func visible(category: ThreadCategory?) -> [ThreadItem] {
        rows.filter { row in !archived.contains(row.id) && (category == nil || row.category == category) }
    }

    public func archive(_ ids: some Sequence<String>) {
        archived.formUnion(ids)
    }

    public func refresh() async {
        do {
            rows = try await source.fetch()
            refreshFailed = false
        } catch {
            refreshFailed = true
        }
    }
}
