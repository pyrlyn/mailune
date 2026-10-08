import Foundation

/// One row in the command palette.
public struct MailuneCommand: Identifiable, Equatable, Sendable {
    public var id: String
    public var title: String

    public init(id: String, title: String) {
        self.id = id
        self.title = title
    }
}

/// The commands the palette can run. Filtering stays out of the view so a test can cover it.
public enum CommandList {
    public static let all: [MailuneCommand] = [
        MailuneCommand(id: "inbox", title: "Go to Inbox"),
        MailuneCommand(id: "compose", title: "Compose"),
    ]

    public static func filter(_ query: String) -> [MailuneCommand] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return all }
        return all.filter { $0.title.localizedCaseInsensitiveContains(trimmed) }
    }
}
