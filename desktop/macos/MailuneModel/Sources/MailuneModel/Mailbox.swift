/// A mailbox the shell can open. The title is a catalog key, so the model never
/// carries display text in one language.
public struct Mailbox: Sendable, Equatable, Hashable, Identifiable {
    public var id: String
    public var titleKey: String

    public init(id: String, titleKey: String) {
        self.id = id
        self.titleKey = titleKey
    }

    public static let inbox = Mailbox(id: "inbox", titleKey: "app.inbox")
    public static let sent = Mailbox(id: "sent", titleKey: "data.sent")

    /// The sidebar order until accounts come from the core.
    public static let fixtures: [Mailbox] = [.inbox, .sent]
}
