import Foundation

/// Selection, archive, and pull-to-refresh for the phone list.
public struct PhoneListState: Equatable, Sendable {
    public var selected: String?
    public var archived: Set<String>
    public var refreshCount: Int

    public init(selected: String? = nil, archived: Set<String> = [], refreshCount: Int = 0) {
        self.selected = selected
        self.archived = archived
        self.refreshCount = refreshCount
    }

    public mutating func archive(_ id: String) {
        archived.insert(id)
        if selected == id {
            selected = nil
        }
    }

    public mutating func refresh() {
        refreshCount += 1
    }

    public func rows() -> [ThreadItem] {
        ThreadFixtures.visible(category: "Primary", archived: archived)
    }
}
