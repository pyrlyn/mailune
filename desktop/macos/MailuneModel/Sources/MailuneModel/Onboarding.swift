import CryptoKit
import Foundation

/// Socket protection, spelled as `mailune-auth`'s `SocketSecurity`.
public enum SocketSecurity: String, Codable, Sendable {
    case tls, starttls, plain
}

public enum ServerProtocol: String, Codable, Sendable {
    case imap, smtp
}

/// One server, as `mailune-auth`'s `ServerEndpoint`. The host is not looked up.
public struct ServerEndpoint: Codable, Equatable, Sendable {
    public var service: ServerProtocol
    public var host: String
    public var port: Int
    public var security: SocketSecurity
}

public struct AutoconfigResult: Decodable, Equatable, Sendable {
    public var incoming: [ServerEndpoint]
    public var outgoing: [ServerEndpoint]
}

/// Where server settings come from. The core's ISPDB, SRV and MX lookup (P14)
/// replaces the fixture once the binding exists.
public protocol AutoconfigSource: Sendable {
    func lookup(domain: String) async throws -> AutoconfigResult?
}

public struct FixtureAutoconfig: AutoconfigSource {
    public init() {}

    public func lookup(domain: String) async throws -> AutoconfigResult? {
        guard let url = Bundle.module.url(forResource: "autoconfig", withExtension: "json") else { return nil }
        let table = try JSONDecoder().decode([String: AutoconfigResult].self, from: Data(contentsOf: url))
        return table[domain]
    }
}

public struct Redirect: Sendable {
    public var code: String
    public var state: String

    public init(code: String, state: String) {
        self.code = code
        self.state = state
    }
}

/// Receives the authorization redirect, like `mailune-auth`'s `RedirectListener`.
public protocol RedirectListener: Sendable {
    func receive(for session: OAuthSession) async throws -> Redirect
}

/// Access and refresh tokens. The description is redacted so printing the
/// value is never how a token reaches a log.
public struct IssuedToken: Sendable, CustomStringConvertible {
    let access: String
    let refresh: String

    public init(access: String, refresh: String) {
        self.access = access
        self.refresh = refresh
    }

    public var description: String { "IssuedToken(redacted)" }
}

public protocol TokenIssuer: Sendable {
    func exchange(code: String, verifier: String) async throws -> IssuedToken
}

/// One sign-in: a random state and a PKCE S256 pair, as in P15.
public struct OAuthSession: Sendable {
    public let state: String
    let verifier: String

    public init(state: String, verifier: String) {
        self.state = state
        self.verifier = verifier
    }

    public static func start() -> OAuthSession {
        OAuthSession(state: random(), verifier: random())
    }

    public var challenge: String {
        Self.base64url(Data(SHA256.hash(data: Data(verifier.utf8))))
    }

    func complete(listener: RedirectListener, issuer: TokenIssuer) async throws -> IssuedToken {
        let redirect = try await listener.receive(for: self)
        guard redirect.state == state else { throw OnboardingError.stateMismatch }
        guard !redirect.code.isEmpty else { throw OnboardingError.emptyCode }
        return try await issuer.exchange(code: redirect.code, verifier: verifier)
    }

    private static func random() -> String {
        var generator = SystemRandomNumberGenerator()
        return base64url(Data((0 ..< 32).map { _ in UInt8.random(in: .min ... .max, using: &generator) }))
    }

    static func base64url(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}

/// Where tokens go. The core's keychain store (C1) replaces the in-memory one.
@MainActor
public protocol SecretVault: AnyObject {
    func store(_ token: IssuedToken, account: String)
}

@MainActor
public final class MemorySecretVault: SecretVault {
    private var tokens: [String: IssuedToken] = [:]

    public init() {}

    public func store(_ token: IssuedToken, account: String) {
        tokens[account] = token
    }

    public func holds(account: String) -> Bool {
        tokens[account] != nil
    }
}

/// The sign-in stand-in until the core's loopback listener is bound: it
/// answers with the session's own state, as a well-behaved server would.
public struct StubRedirectListener: RedirectListener {
    public init() {}

    public func receive(for session: OAuthSession) async throws -> Redirect {
        Redirect(code: "stub-code", state: session.state)
    }
}

public struct StubTokenIssuer: TokenIssuer {
    public init() {}

    public func exchange(code: String, verifier: String) async throws -> IssuedToken {
        IssuedToken(access: "stub-access", refresh: "stub-refresh")
    }
}

public enum OnboardingError: Error, Equatable {
    case notAnAddress
    case alreadyAdded
    case noConfiguration
    /// The server offers only unencrypted connections; Mailune does not use them.
    case insecureOnly
    case stateMismatch
    case emptyCode
}

public struct OnboardingServices: Sendable {
    public var autoconfig: AutoconfigSource
    public var listener: RedirectListener
    public var issuer: TokenIssuer

    public init(autoconfig: AutoconfigSource, listener: RedirectListener, issuer: TokenIssuer) {
        self.autoconfig = autoconfig
        self.listener = listener
        self.issuer = issuer
    }

    public static let stub = OnboardingServices(
        autoconfig: FixtureAutoconfig(),
        listener: StubRedirectListener(),
        issuer: StubTokenIssuer()
    )
}

public enum Onboarding {
    /// Finds encrypted servers, signs in, keeps the token in the vault and
    /// saves the account. Nothing is saved when a step fails.
    @MainActor
    public static func addAccount(
        address: String,
        displayName: String,
        services: OnboardingServices,
        vault: SecretVault,
        store: PreferencesStore
    ) async throws -> AccountSetting {
        let address = address.trimmingCharacters(in: .whitespaces)
        guard Composer.looksLikeAddress(address), let at = address.lastIndex(of: "@") else {
            throw OnboardingError.notAnAddress
        }
        var preferences = store.current()
        guard !preferences.accounts.contains(where: { $0.address.caseInsensitiveCompare(address) == .orderedSame }) else {
            throw OnboardingError.alreadyAdded
        }
        let domain = address[address.index(after: at)...].lowercased()
        guard let found = try await services.autoconfig.lookup(domain: domain) else {
            throw OnboardingError.noConfiguration
        }
        guard let incoming = found.incoming.first(where: { $0.security != .plain }),
              let outgoing = found.outgoing.first(where: { $0.security != .plain })
        else {
            throw OnboardingError.insecureOnly
        }
        let token = try await OAuthSession.start().complete(listener: services.listener, issuer: services.issuer)
        let account = AccountSetting(
            id: UUID().uuidString,
            address: address,
            displayName: displayName.isEmpty ? address : displayName,
            incoming: incoming,
            outgoing: outgoing
        )
        vault.store(token, account: account.id)
        preferences.accounts.append(account)
        try store.save(preferences)
        return account
    }
}
