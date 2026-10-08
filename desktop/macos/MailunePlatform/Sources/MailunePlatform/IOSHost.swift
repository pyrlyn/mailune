import Foundation

/// A background refresh the phone would schedule. The fake only records the request.
public protocol BackgroundRefresh: Sendable {
    func schedule()
    func isScheduled() -> Bool
}

public final class FakeBackgroundRefresh: BackgroundRefresh, @unchecked Sendable {
    private let lock = NSLock()
    private var scheduled = false

    public init() {}

    public func schedule() {
        lock.lock()
        scheduled = true
        lock.unlock()
    }

    public func isScheduled() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return scheduled
    }
}

/// The three host services the phone uses. Tests pass fakes, so nothing reaches the system.
public struct IOSHostServices: Sendable {
    public var keychain: any Keychain
    public var refresh: any BackgroundRefresh
    public var notifier: any Notifier

    public init(keychain: any Keychain, refresh: any BackgroundRefresh, notifier: any Notifier) {
        self.keychain = keychain
        self.refresh = refresh
        self.notifier = notifier
    }

    public static func fakes() -> IOSHostServices {
        IOSHostServices(keychain: FakeKeychain(), refresh: FakeBackgroundRefresh(), notifier: FakeNotifier())
    }
}
