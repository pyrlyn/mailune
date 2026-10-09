import MailuneModel
import SwiftUI

/// Three panes, a toolbar, ⌘K, and a command palette.
public struct ShellView: View {
    @State private var mailbox: Mailbox? = .inbox
    @State private var selected: Set<String> = []
    @State private var palette = false
    @State private var query = ""

    public init() {}

    private var detailTitle: String {
        guard selected.count == 1, let id = selected.first,
              let thread = ThreadFixtures.all.first(where: { $0.id == id })
        else {
            return Copy.text("app.no_conversation_open")
        }
        return thread.subject
    }

    public var body: some View {
        NavigationSplitView {
            List(Mailbox.fixtures, selection: $mailbox) { item in
                Text(Copy.text(item.titleKey)).tag(item)
            }
            .navigationSplitViewColumnWidth(min: 160, ideal: 200)
        } content: {
            ThreadList(selected: $selected)
                .navigationSplitViewColumnWidth(min: 260, ideal: 320)
        } detail: {
            Text(detailTitle)
                .font(MailuneType.title)
                .foregroundStyle(MailuneColor.ink)
                .padding(MailuneSpace.m)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .background(MailuneColor.canvas)
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button(Copy.text("shell.commands")) { palette = true }
                    .keyboardShortcut("k", modifiers: .command)
            }
        }
        .sheet(isPresented: $palette) {
            CommandPalette(query: $query) { command in
                switch command.action {
                case let .open(target):
                    mailbox = target
                }
                palette = false
                query = ""
            }
        }
    }
}

struct CommandPalette: View {
    @Binding var query: String
    var choose: (MailuneCommand) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.s) {
            TextField(Copy.text("shell.command_placeholder"), text: $query)
                .font(MailuneType.body)
            List(CommandList.filter(query)) { command in
                Button(command.title) { choose(command) }
                    .buttonStyle(.plain)
            }
        }
        .padding(MailuneSpace.m)
        .frame(minWidth: 320, minHeight: 220)
    }
}
