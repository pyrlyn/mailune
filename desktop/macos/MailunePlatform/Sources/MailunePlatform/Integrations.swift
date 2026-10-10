import Foundation
import MailuneModel

/// A `mailto:` link (RFC 6068) as a composer draft. Only addresses, subject
/// and body are read: a link never attaches a file or sends on its own, and
/// the draft still goes through the composer's confirm step.
public enum MailtoLink {
    public static func draft(from url: URL) -> Draft? {
        guard url.scheme?.lowercased() == "mailto",
              let parts = URLComponents(url: url, resolvingAgainstBaseURL: false)
        else {
            return nil
        }
        var draft = Draft()
        var addresses = parts.path.split(separator: ",").map(String.init)
        for item in parts.queryItems ?? [] {
            let value = item.value ?? ""
            switch item.name.lowercased() {
            case "to": addresses += value.split(separator: ",").map(String.init)
            case "subject": draft.subject = value
            case "body": draft.body = value
            default: continue
            }
        }
        for address in addresses.map({ $0.trimmingCharacters(in: .whitespaces) })
            where Composer.looksLikeAddress(address) && !draft.to.contains(address)
        {
            draft.to.append(address)
        }
        return draft
    }
}

@MainActor
public protocol DockBadge: AnyObject {
    var label: String? { get set }
}

public enum UnreadBadge {
    /// The Dock shows unread conversations, capped so the badge stays readable.
    @MainActor
    public static func update(_ dock: DockBadge, rows: [ThreadItem]) {
        let unread = rows.filter(\.unread).count
        dock.label = unread == 0 ? nil : unread > 99 ? "99+" : String(unread)
    }
}

@MainActor
public protocol Sharing: AnyObject {
    func share(_ text: String)
}

public enum ShareContent {
    public static func text(for message: MailMessage) -> String {
        message.subject + "\n\n" + message.body
    }
}

public struct SpotlightItem: Equatable, Sendable {
    public var id: String
    public var title: String
    public var sender: String
    public var text: String
}

@MainActor
public protocol SpotlightIndex: AnyObject {
    func replace(with items: [SpotlightItem])
}

public enum SpotlightItems {
    /// Spotlight's store lives outside the app's encrypted database, so the
    /// text of an encrypted message never goes there: decrypting it into the
    /// index would leave it readable on disk.
    public static func items(for rows: [ThreadItem], messages: [MailMessage]) -> [SpotlightItem] {
        rows.map { row in
            let text = messages
                .filter { $0.thread == row.id && !$0.security.encrypted }
                .map(\.body)
                .joined(separator: "\n\n")
            return SpotlightItem(id: row.id, title: row.subject, sender: row.from.display, text: text)
        }
    }
}

/// The services the shell asks the host for, live or fake.
@MainActor
public struct HostServices {
    public var dock: DockBadge
    public var sharing: Sharing
    public var spotlight: SpotlightIndex

    public init(dock: DockBadge, sharing: Sharing, spotlight: SpotlightIndex) {
        self.dock = dock
        self.sharing = sharing
        self.spotlight = spotlight
    }
}
