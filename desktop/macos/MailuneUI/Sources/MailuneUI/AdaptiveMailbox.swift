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

        var body: some View {
            NavigationStack {
                PhoneThreadList(feed: feed)
                    .navigationTitle(Copy.text("app.inbox"))
                    .navigationDestination(for: String.self) { id in
                        Text(verbatim: feed.rows.first { $0.id == id }?.subject ?? "")
                    }
            }
        }
    }
#endif
