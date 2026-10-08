import MailuneModel
import SwiftUI

/// Three panes, a toolbar, one shortcut, and a command palette.
public struct ShellView: View {
    @State private var mailbox = "Inbox"
    @State private var selected: Set<String> = []
    @State private var palette = false
    @State private var query = ""
    @State private var showSettings = false
    @State private var showComposer = false
    @State private var showSearch = false
    @State private var showOnboarding = false
    @State private var settingsStore = FakePreferencesStore()

    public init() {}

    public var body: some View {
        NavigationSplitView {
            mailboxList
        } content: {
            if mailbox == "Calendar" {
                CalendarView()
                    .navigationSplitViewColumnWidth(min: 220, ideal: 280)
            } else {
                ThreadList(selected: $selected)
                    .navigationSplitViewColumnWidth(min: 220, ideal: 280)
            }
        } detail: {
            if let id = selected.first, selected.count == 1, let message = MessageFixtures.message(forThread: id) {
                ReaderView(message: message)
            } else {
                Text(selected.isEmpty ? "No thread" : "\(selected.count) selected")
                    .font(MailuneType.title)
                    .foregroundStyle(MailuneColor.ink)
                    .padding(MailuneSpace.m)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .background(MailuneColor.canvas)
            }
        }
        .frame(minWidth: 880, minHeight: 520)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Add account") { showOnboarding = true }
            }
            ToolbarItem(placement: .primaryAction) {
                Button("Search") { showSearch = true }
            }
            ToolbarItem(placement: .primaryAction) {
                Button("Compose") { showComposer = true }
            }
            ToolbarItem(placement: .primaryAction) {
                Button("Settings") { showSettings = true }
            }
            ToolbarItem(placement: .primaryAction) {
                Button("Commands") { palette = true }
                    .keyboardShortcut("k", modifiers: .command)
            }
        }
        .sheet(isPresented: $showOnboarding) {
            OnboardingView()
        }
        .sheet(isPresented: $showSearch) {
            SearchView()
        }
        .sheet(isPresented: $showComposer) {
            ComposerView()
        }
        .sheet(isPresented: $showSettings) {
            SettingsView(store: settingsStore)
                .frame(minWidth: 420, minHeight: 360)
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

extension ShellView {
    @ViewBuilder
    private var mailboxList: some View {
        // The single-value selection initializer is a macOS sidebar API.
        #if os(macOS)
        List(selection: $mailbox) {
            Text(Copy.text("inbox.title", language: "en")).tag("Inbox")
            Text("Sent").tag("Sent")
            Text("Calendar").tag("Calendar")
        }
        .navigationSplitViewColumnWidth(min: 160, ideal: 200)
        #else
        List {
            Text(Copy.text("inbox.title", language: "en"))
            Text("Sent")
        }
        #endif
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
