/// A mailbox the shell can open. Later tasks fill this in.
public struct Mailbox: Sendable, Equatable {
    /// Name shown in the sidebar.
    public var title: String

    public init(title: String) {
        self.title = title
    }

    /// The mailbox a new window opens.
    public static let inbox = Mailbox(title: "Inbox")
}
