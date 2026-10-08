import Foundation

/// Preferences the settings screen edits. One value per section.
public struct Preferences: Equatable, Sendable {
    public var accountName: String
    public var appearance: String
    public var notifications: Bool
    public var readingPane: Bool
    public var composeFontSize: Int
    public var syncIntervalMinutes: Int

    public init(
        accountName: String,
        appearance: String,
        notifications: Bool,
        readingPane: Bool,
        composeFontSize: Int,
        syncIntervalMinutes: Int
    ) {
        self.accountName = accountName
        self.appearance = appearance
        self.notifications = notifications
        self.readingPane = readingPane
        self.composeFontSize = composeFontSize
        self.syncIntervalMinutes = syncIntervalMinutes
    }

    public static let standard = Preferences(
        accountName: "Ana",
        appearance: "System",
        notifications: true,
        readingPane: true,
        composeFontSize: 14,
        syncIntervalMinutes: 5
    )

    public static let appearances = ["System", "Light", "Dark"]
}

/// Where settings are kept. The screen writes through this; tests use the fake.
public protocol PreferencesStore: Sendable {
    func load() -> Preferences
    func save(_ preferences: Preferences)
}

public final class FakePreferencesStore: PreferencesStore, @unchecked Sendable {
    private let lock = NSLock()
    private var value: Preferences

    public init(value: Preferences = .standard) {
        self.value = value
    }

    public func load() -> Preferences {
        lock.lock()
        defer { lock.unlock() }
        return value
    }

    public func save(_ preferences: Preferences) {
        lock.lock()
        value = preferences
        lock.unlock()
    }
}
