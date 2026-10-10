import MailuneModel
import MailunePlatform
import SwiftUI

/// Three panes, a toolbar, ⌘K, and a command palette.
public struct ShellView: View {
    @State private var mailbox: Mailbox? = .inbox
    @State private var selected: Set<String> = []
    @State private var palette = false
    @State private var query = ""
    @State private var composing = false
    @State private var outbox = FakeOutbox()
    @State private var search = ""
    @State private var asking = false
    @State private var startDraft = Draft()
    private let preferences: any PreferencesStore
    private let host: HostServices

    public init(preferences: any PreferencesStore = FakePreferencesStore(), host: HostServices = .fake()) {
        self.preferences = preferences
        self.host = host
    }

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
            ThreadList(selected: $selected, query: search)
                .searchable(text: $search, placement: .toolbar, prompt: Text(Copy.text("search.search_in_mail")))
                .navigationSplitViewColumnWidth(min: 260, ideal: 320)
        } detail: {
            if let message = openMessage {
                ReaderView(
                    message: message,
                    assist: AssistPolicy.visible(AssistFixtures.assist(forThread: message.thread), for: message)
                ) { text in
                    startDraft = .reply(to: message, body: text)
                    composing = true
                }
                .id(message.id)
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
                    startDraft = Draft()
                    composing = true
                } label: {
                    // The app target's asset catalog holds this image.
                    Image("ToolbarCompose")
                }
                .accessibilityLabel(Copy.text("app.compose"))
                .keyboardShortcut("n", modifiers: .command)
            }
            ToolbarItem(placement: .primaryAction) {
                Button(Copy.text("search.ask")) { asking = true }
            }
            ToolbarItem(placement: .primaryAction) {
                Button(Copy.text("sheets.share")) {
                    if let message = openMessage {
                        host.sharing.share(ShareContent.text(for: message))
                    }
                }
                .disabled(openMessage == nil)
            }
            ToolbarItem(placement: .primaryAction) {
                Button(Copy.text("shell.commands")) { palette = true }
                    .keyboardShortcut("k", modifiers: .command)
            }
        }
        .sheet(isPresented: $composing) {
            ComposerView(
                outbox: outbox,
                undoWindow: TimeInterval(preferences.current().undoSendSeconds),
                draft: startDraft
            )
        }
        .onOpenURL { url in
            if let draft = MailtoLink.draft(from: url) {
                startDraft = draft
                composing = true
            }
        }
        .task {
            UnreadBadge.update(host.dock, rows: ThreadFixtures.all)
            host.spotlight.replace(with: SpotlightItems.items(for: ThreadFixtures.all, messages: MessageFixtures.all))
        }
        .sheet(isPresented: $asking) {
            AskView { citation in
                search = ""
                selected = [citation.thread]
                asking = false
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
