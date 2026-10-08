import Foundation

/// One message the reader can show. Plain text only, so a view cannot run a script.
public struct MailMessage: Equatable, Sendable, Identifiable {
    public var id: String
    public var subject: String
    public var body: String
    public var quoted: String
    public var attachmentName: String
    public var badge: String

    public init(
        id: String,
        subject: String,
        body: String,
        quoted: String,
        attachmentName: String,
        badge: String
    ) {
        self.id = id
        self.subject = subject
        self.body = body
        self.quoted = quoted
        self.attachmentName = attachmentName
        self.badge = badge
    }
}

/// How the reader lays a message out. Remote content and scripts stay off.
public struct ReaderPresentation: Equatable, Sendable {
    public var text: String
    public var quotesCollapsed: Bool
    public var attachmentName: String
    public var badge: String
    public var remoteContentEnabled: Bool
    public var javaScriptEnabled: Bool

    public init(message: MailMessage, quotesCollapsed: Bool) {
        text = quotesCollapsed ? message.body : message.body + "\n" + message.quoted
        self.quotesCollapsed = quotesCollapsed
        attachmentName = message.attachmentName
        badge = message.badge
        remoteContentEnabled = false
        javaScriptEnabled = false
    }
}

/// Fixture messages keyed by thread id.
public enum MessageFixtures {
    public static func message(forThread id: String) -> MailMessage? {
        switch id {
        case "1":
            MailMessage(
                id: "1",
                subject: "Hello",
                body: "See you tomorrow.",
                quoted: "On Monday Ana wrote: earlier note",
                attachmentName: "notes.txt",
                badge: "Authenticated"
            )
        default:
            nil
        }
    }
}
