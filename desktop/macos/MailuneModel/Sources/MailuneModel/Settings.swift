import Foundation

public enum Theme: String, Codable, CaseIterable, Sendable {
    case system, light, dark

    public var titleKey: String { "settings.\(rawValue)" }
}

public enum Density: String, Codable, CaseIterable, Sendable {
    case compact, cozy, roomy

    public var titleKey: String { "settings.\(rawValue)" }
}

/// An account's public settings. Its token lives in a `SecretVault`, never here.
public struct AccountSetting: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var address: String
    public var displayName: String
    public var incoming: ServerEndpoint? = nil
    public var outgoing: ServerEndpoint? = nil
}

/// Everything the settings screen edits, one group per section. No secret
/// lives here: passwords and tokens stay in the core's keychain store.
public struct Preferences: Codable, Equatable, Sendable {
    public var accounts: [AccountSetting]
    public var theme: Theme
    public var density: Density
    public var notifications: Bool
    public var onlyVIPs: Bool
    public var conversationView: Bool
    public var autoAdvance: Bool
    public var undoSendSeconds: Int
    public var signature: String
    public var syncDays: Int
    public var wifiOnly: Bool

    public static let standard = Preferences(
        accounts: [AccountSetting(id: "a1", address: "ana@acme.example", displayName: "Ana Ruiz")],
        theme: .system,
        density: .cozy,
        notifications: true,
        onlyVIPs: false,
        conversationView: true,
        autoAdvance: false,
        undoSendSeconds: 10,
        signature: "",
        syncDays: 90,
        wifiOnly: false
    )

    public static let undoSendChoices = [5, 10, 20, 30]
    public static let syncDayChoices = [30, 90, 365]
}

@MainActor
public protocol PreferencesStore: AnyObject {
    func load() throws -> Preferences
    func save(_ preferences: Preferences) throws
}

extension PreferencesStore {
    /// What the app starts from: the saved value, or the defaults when there
    /// is none or it no longer decodes.
    public func current() -> Preferences {
        (try? load()) ?? .standard
    }
}

/// Keeps the encoded bytes, so a round trip goes through the same JSON the
/// app's store writes.
@MainActor
public final class FakePreferencesStore: PreferencesStore {
    public var data: Data?

    public init(data: Data? = nil) {
        self.data = data
    }

    public func load() throws -> Preferences {
        guard let data else { return .standard }
        return try JSONDecoder().decode(Preferences.self, from: data)
    }

    public func save(_ preferences: Preferences) throws {
        data = try JSONEncoder().encode(preferences)
    }
}

@MainActor
public final class DefaultsPreferencesStore: PreferencesStore {
    private let defaults: UserDefaults
    private let key = "preferences"

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func load() throws -> Preferences {
        guard let data = defaults.data(forKey: key) else { return .standard }
        return try JSONDecoder().decode(Preferences.self, from: data)
    }

    public func save(_ preferences: Preferences) throws {
        defaults.set(try JSONEncoder().encode(preferences), forKey: key)
    }
}
