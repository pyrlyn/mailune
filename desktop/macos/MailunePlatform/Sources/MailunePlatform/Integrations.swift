import Foundation

/// Secret storage. The macOS host would use the keychain; tests use a fake so
/// they never open one.
public protocol Keychain: Sendable {
    func set(account: String, secret: String)
    func secret(for account: String) -> String?
}

/// Local notifications. The fake records them; it does not ask the system.
public protocol Notifier: Sendable {
    func post(title: String, body: String)
    func posted() -> [(title: String, body: String)]
}

/// Whether the machine can reach the network. A snapshot, not a live monitor.
public protocol NetworkPath: Sendable {
    var isOnline: Bool { get }
}

/// A browser sign-in. The fake returns a token it was given.
public protocol WebAuth: Sendable {
    func authenticate(callback: String) -> String?
}

/// Opens a URL in the user's browser. The fake records the request.
public protocol URLOpener: Sendable {
    func open(_ url: URL) -> Bool
    func opened() -> [URL]
}

public final class FakeKeychain: Keychain, @unchecked Sendable {
    private let lock = NSLock()
    private var values: [String: String] = [:]

    public init() {}

    public func set(account: String, secret: String) {
        lock.lock()
        values[account] = secret
        lock.unlock()
    }

    public func secret(for account: String) -> String? {
        lock.lock()
        defer { lock.unlock() }
        return values[account]
    }
}

public final class FakeNotifier: Notifier, @unchecked Sendable {
    private let lock = NSLock()
    private var notes: [(title: String, body: String)] = []

    public init() {}

    public func post(title: String, body: String) {
        lock.lock()
        notes.append((title, body))
        lock.unlock()
    }

    public func posted() -> [(title: String, body: String)] {
        lock.lock()
        defer { lock.unlock() }
        return notes
    }
}

public struct FakeNetworkPath: NetworkPath, Sendable {
    public var isOnline: Bool
    public init(isOnline: Bool) {
        self.isOnline = isOnline
    }
}

public struct FakeWebAuth: WebAuth, Sendable {
    public var token: String?
    public init(token: String?) {
        self.token = token
    }

    public func authenticate(callback: String) -> String? {
        guard !callback.isEmpty else { return nil }
        return token
    }
}

public final class FakeURLOpener: URLOpener, @unchecked Sendable {
    private let lock = NSLock()
    private var urls: [URL] = []

    public init() {}

    public func open(_ url: URL) -> Bool {
        lock.lock()
        urls.append(url)
        lock.unlock()
        return true
    }

    public func opened() -> [URL] {
        lock.lock()
        defer { lock.unlock() }
        return urls
    }
}
