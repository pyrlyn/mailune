import MailuneModel
import SwiftUI

/// Three panes, a toolbar, ⌘K, and a command palette.
public struct ShellView: View {
    @State private var mailbox: Mailbox? = .inbox
    @State private var selected: Set<String> = []
    @State private var palette = false
    @State private var query = ""
    @State private var composing = false
    @State private var outbox = FakeOutbox()

    public init() {}

    private var openMessage: MailMessage? {
        guard selected.count == 1, let id = selected.first else { return nil }
        return MessageFixtures.message(forThread: id)
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
            if let message = openMessage {
                ReaderView(message: message).id(message.id)
            } else {
                Text(Copy.text("app.no_conversation_open"))
                    .font(MailuneType.title)
                    .foregroundStyle(MailuneColor.ink)
                    .padding(MailuneSpace.m)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .background(MailuneColor.canvas)
            }
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    composing = true
                } label: {
                    // The app target's asset catalog holds this image.
                    Image("ToolbarCompose")
                }
                .accessibilityLabel(Copy.text("app.compose"))
                .keyboardShortcut("n", modifiers: .command)
            }
            ToolbarItem(placement: .primaryAction) {
                Button(Copy.text("shell.commands")) { palette = true }
                    .keyboardShortcut("k", modifiers: .command)
            }
        }
        .sheet(isPresented: $composing) {
            ComposerView(outbox: outbox)
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
