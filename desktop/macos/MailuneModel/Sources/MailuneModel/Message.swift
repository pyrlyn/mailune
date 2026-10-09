import Foundation

/// DKIM outcome, spelled as `mailune-mime`'s `DkimVerdict`.
public enum DkimVerdict: String, Decodable, Sendable {
    case pass, fail, none
    case tempError = "temp_error"
    case permError = "perm_error"
}

/// What the OpenPGP or S/MIME check said about a signature.
public enum SignatureState: String, Decodable, Sendable {
    case none, valid, invalid
}

/// The checks the core ran on one message. The reader only renders them.
public struct SecurityState: Equatable, Sendable, Decodable {
    public var dkim: DkimVerdict
    public var signature: SignatureState
    public var encrypted: Bool

    public init(dkim: DkimVerdict, signature: SignatureState, encrypted: Bool) {
        self.dkim = dkim
        self.signature = signature
        self.encrypted = encrypted
    }
}

/// The one badge the reader shows. A failed check wins over everything else,
/// so a forged message never looks better than an unsigned one.
public enum SecurityBadge: String, Equatable, Sendable {
    case failed, encrypted, signed, authenticated, unverified

    public init(_ state: SecurityState) {
        if state.signature == .invalid || state.dkim == .fail {
            self = .failed
        } else if state.encrypted {
            self = .encrypted
        } else if state.signature == .valid {
            self = .signed
        } else if state.dkim == .pass {
            self = .authenticated
        } else {
            self = .unverified
        }
    }

    public var titleKey: String { "reader.badge.\(rawValue)" }
}

/// What `mailune-mime`'s remote policy blocked, counted. The URLs stay in the
/// core: the shell never holds one, so it cannot fetch one.
public struct RemoteSummary: Equatable, Sendable, Decodable {
    public var blocked: Int
    public var trackerPixels: Int

    public init(blocked: Int, trackerPixels: Int) {
        self.blocked = blocked
        self.trackerPixels = trackerPixels
    }
}

public struct Attachment: Equatable, Sendable, Decodable {
    public var name: String
    public var size: Int

    public init(name: String, size: Int) {
        self.name = name
        self.size = size
    }
}

/// One message as the core hands it to the reader: the body is the sanitizer's
/// plain text, and the trailing quote is already split off.
public struct MailMessage: Identifiable, Equatable, Sendable, Decodable {
    public var id: String
    public var thread: String
    public var from: ThreadItem.Sender
    public var subject: String
    public var body: String
    public var quoted: String?
    public var attachments: [Attachment]
    public var security: SecurityState
    public var remote: RemoteSummary
}

/// How the reader lays a message out.
public struct ReaderPresentation: Equatable, Sendable {
    public var text: String
    public var hasQuote: Bool
    public var attachments: [Attachment]
    public var badge: SecurityBadge
    /// Nil when the policy blocked nothing, so no banner is drawn.
    public var blockedRemote: RemoteSummary?

    public init(message: MailMessage, quotesCollapsed: Bool) {
        let quote = message.quoted.flatMap { $0.isEmpty ? nil : $0 }
        hasQuote = quote != nil
        if let quote, !quotesCollapsed {
            text = message.body + "\n\n" + quote
        } else {
            text = message.body
        }
        attachments = message.attachments
        badge = SecurityBadge(message.security)
        blockedRemote = message.remote.blocked > 0 ? message.remote : nil
    }
}

/// Messages from `messages.json` until the core is wired.
public enum MessageFixtures {
    public static let all: [MailMessage] = load()

    public static func message(forThread id: String) -> MailMessage? {
        all.first { $0.thread == id }
    }

    static func decode(_ data: Data) throws -> [MailMessage] {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode([MailMessage].self, from: data)
    }

    private static func load() -> [MailMessage] {
        guard let url = Bundle.module.url(forResource: "messages", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let messages = try? decode(data)
        else {
            return []
        }
        return messages
    }
}
