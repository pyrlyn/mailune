import SwiftUI

/// Every key the shell binds, in one table so a test can check that none
/// collides with another or with one macOS keeps for itself.
public enum MailuneShortcut: CaseIterable, Sendable {
    case compose, commands, reply, send, toggleQuote

    /// Reading to a sent reply without the pointer: arrow keys pick a thread,
    /// ⌘R replies, ⌘↩ asks to send, and Return confirms.
    public static let replyPath: [MailuneShortcut] = [.reply, .send]

    public var key: KeyEquivalent {
        switch self {
        case .compose: "n"
        case .commands: "k"
        case .reply: "r"
        case .send: .return
        case .toggleQuote: "q"
        }
    }

    public var modifiers: EventModifiers {
        self == .toggleQuote ? [.command, .shift] : .command
    }
}

extension View {
    func keyboardShortcut(_ shortcut: MailuneShortcut) -> some View {
        keyboardShortcut(shortcut.key, modifiers: shortcut.modifiers)
    }
}
