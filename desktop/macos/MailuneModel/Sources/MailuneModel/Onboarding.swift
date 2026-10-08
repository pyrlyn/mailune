import Foundation

/// An account created locally. Autoconfig guesses the host; it does not look it up.
public struct MailAccount: Equatable, Sendable, Identifiable {
    public var id: String
    public var email: String
    public var host: String
    public var token: String

    public init(id: String, email: String, host: String, token: String) {
        self.id = id
        self.email = email
        self.host = host
        self.token = token
    }
}

public enum Autoconfig {
    /// `imap.` plus the domain after `@`. There is no network call.
    public static func host(for email: String) -> String? {
        let parts = email.split(separator: "@", omittingEmptySubsequences: false)
        guard parts.count == 2, !parts[0].isEmpty, parts[1].contains(".") else { return nil }
        return "imap.\(parts[1])"
    }
}

/// A stand-in for the browser sign-in. It mints a token and does not open a session.
public struct OAuthStub: Sendable {
    public init() {}

    public func token(for email: String) -> String {
        "stub-\(email)"
    }
}

public final class FakeAccountStore: @unchecked Sendable {
    private let lock = NSLock()
    private var saved: [MailAccount] = []

    public init() {}

    public func create(email: String) -> MailAccount? {
        guard let host = Autoconfig.host(for: email) else { return nil }
        let account = MailAccount(
            id: email,
            email: email,
            host: host,
            token: OAuthStub().token(for: email)
        )
        lock.lock()
        saved.append(account)
        lock.unlock()
        return account
    }

    public func accounts() -> [MailAccount] {
        lock.lock()
        defer { lock.unlock() }
        return saved
    }
}
