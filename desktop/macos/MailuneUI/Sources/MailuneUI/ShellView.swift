import SwiftUI

/// Three panes, a toolbar, one shortcut, and a command palette.
public struct ShellView: View {
    @State private var mailbox = "Inbox"
    @State private var thread = "Hello"
    @State private var palette = false
    @State private var query = ""

    public init() {}

    public var body: some View {
        NavigationSplitView {
            List(selection: $mailbox) {
                Text(Copy.text("inbox.title", language: "en")).tag("Inbox")
                Text("Sent").tag("Sent")
            }
            .navigationSplitViewColumnWidth(min: 160, ideal: 200)
        } content: {
            List(selection: $thread) {
                Text("Hello").tag("Hello")
                    .font(MailuneType.body)
                    .padding(MailuneSpace.s)
            }
            .navigationSplitViewColumnWidth(min: 220, ideal: 280)
        } detail: {
            Text(thread)
                .font(MailuneType.title)
                .foregroundStyle(MailuneColor.ink)
                .padding(MailuneSpace.m)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .background(MailuneColor.canvas)
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Commands") { palette = true }
                    .keyboardShortcut("k", modifiers: .command)
            }
        }
        .sheet(isPresented: $palette) {
            CommandPalette(query: $query) { command in
                if command.id == "inbox" {
                    mailbox = "Inbox"
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
            TextField("Command", text: $query)
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
