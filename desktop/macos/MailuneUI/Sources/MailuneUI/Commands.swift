import MailuneModel

/// One row in the command palette.
public struct MailuneCommand: Identifiable, Equatable, Sendable {
    public enum Action: Equatable, Sendable {
        case open(Mailbox)
    }

    public var id: String
    public var title: String
    public var action: Action
}

/// The palette's commands. Filtering stays out of the view so a test can cover it.
public enum CommandList {
    public static func all(language: String? = nil) -> [MailuneCommand] {
        Mailbox.fixtures.map { mailbox in
            MailuneCommand(
                id: "open.\(mailbox.id)",
                title: Copy.format("shell.go_to", Copy.text(mailbox.titleKey, language: language), language: language),
                action: .open(mailbox)
            )
        }
    }

    public static func filter(_ query: String, language: String? = nil) -> [MailuneCommand] {
        let commands = all(language: language)
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return commands }
        return commands.filter { $0.title.localizedCaseInsensitiveContains(trimmed) }
    }
}
