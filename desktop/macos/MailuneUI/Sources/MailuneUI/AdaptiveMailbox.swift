#if os(iOS)
    import MailuneModel
    import MailunePlatform
    import SwiftUI

    /// A phone gets one column that pushes; an iPad gets the shared three-pane
    /// shell. Size class, not device, decides, so iPad split view and slide
    /// over get the phone layout when they are narrow.
    public struct AdaptiveMailbox: View {
        public enum Layout: Equatable, Sendable {
            case stack, split
        }

        @Environment(\.horizontalSizeClass) private var sizeClass
        private let preferences: any PreferencesStore
        private let host: HostServices

        public init(preferences: any PreferencesStore, host: HostServices) {
            self.preferences = preferences
            self.host = host
        }

        nonisolated public static func layout(for sizeClass: UserInterfaceSizeClass?) -> Layout {
            sizeClass == .compact ? .stack : .split
        }

        public var body: some View {
            switch Self.layout(for: sizeClass) {
            case .stack:
                PhoneStack(preferences: preferences)
            case .split:
                ShellView(preferences: preferences, host: host)
            }
        }
    }

    struct PhoneStack: View {
        let preferences: any PreferencesStore
        @State private var feed = ThreadFeed()
        @State private var outbox = FakeOutbox()
        @State private var composing = false
        @State private var startDraft = Draft()

        var body: some View {
            NavigationStack {
                PhoneThreadList(feed: feed)
                    .navigationTitle(Copy.text("app.inbox"))
                    .navigationDestination(for: String.self) { id in
                        if let message = MessageFixtures.message(forThread: id) {
                            ReaderView(message: message, assist: AssistPolicy.fixture(for: message)) { text in
                                compose(.reply(to: message, body: text))
                            }
                            .navigationBarTitleDisplayMode(.inline)
                        } else {
                            Text(Copy.text("app.no_conversation_open"))
                        }
                    }
                    .toolbar {
                        ToolbarItem(placement: .topBarLeading) {
                            Button {
                                compose(Draft())
                            } label: {
                                Image(systemName: "square.and.pencil")
                            }
                            .accessibilityLabel(Copy.text("app.compose"))
                        }
                    }
            }
            .sheet(isPresented: $composing) {
                // A sheet on iOS shows toolbar items only inside a navigation container.
                NavigationStack {
                    ComposerView(
                        outbox: outbox,
                        undoWindow: TimeInterval(preferences.current().undoSendSeconds),
                        draft: startDraft
                    )
                }
            }
        }

        private func compose(_ draft: Draft) {
            startDraft = draft
            composing = true
        }
    }
#endif
