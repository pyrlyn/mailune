import MailuneModel
import SwiftUI

/// A single column. This is the phone layout.
public struct PhoneStack: View {
    @State private var list = PhoneListState()
    @State private var showComposer = false

    public init() {}

    public var body: some View {
        NavigationStack {
            PhoneThreadList(state: $list)
                .navigationTitle("Inbox")
                .toolbar {
                    Button("Compose") { showComposer = true }
                }
                .navigationDestination(for: String.self) { id in
                    PhoneReader(threadID: id)
                }
        }
        .sheet(isPresented: $showComposer) {
            PhoneComposer()
        }
    }
}

/// Three columns. This is the iPad layout.
public struct PadSplit: View {
    public init() {}

    public var body: some View {
        NavigationSplitView {
            List {
                Text("Inbox")
            }
            .navigationTitle("Mailboxes")
        } content: {
            Text("Hello")
                .font(MailuneType.body)
        } detail: {
            Text("No thread")
                .font(MailuneType.title)
                .foregroundStyle(MailuneColor.ink)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .background(MailuneColor.canvas)
        }
    }
}

/// Compact width uses the phone stack. Regular width uses the iPad split.
public struct AdaptiveMailbox: View {
    @Environment(\.horizontalSizeClass) private var sizeClass

    public init() {}

    public var body: some View {
        if sizeClass == .compact {
            PhoneStack()
        } else {
            PadSplit()
        }
    }
}
