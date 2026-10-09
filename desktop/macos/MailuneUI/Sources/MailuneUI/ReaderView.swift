import MailuneModel
import SwiftUI

/// One message as plain text. There is deliberately no web view: the body is
/// the core's sanitized text, so no script runs and nothing remote loads.
public struct ReaderView: View {
    private let message: MailMessage
    @State private var quotesCollapsed = true

    public init(message: MailMessage) {
        self.message = message
    }

    public var body: some View {
        let presentation = ReaderPresentation(message: message, quotesCollapsed: quotesCollapsed)
        ScrollView {
            VStack(alignment: .leading, spacing: MailuneSpace.m) {
                HStack(alignment: .firstTextBaseline) {
                    Text(message.subject)
                        .font(MailuneType.title)
                    Spacer()
                    SecurityBadgeView(badge: presentation.badge)
                }
                Text(message.from.display)
                if let blocked = presentation.blockedRemote {
                    RemoteBanner(summary: blocked)
                }
                Text(presentation.text)
                    .textSelection(.enabled)
                if presentation.hasQuote {
                    Button(Copy.text(quotesCollapsed ? "reader.show_quote" : "reader.hide_quote")) {
                        quotesCollapsed.toggle()
                    }
                    .keyboardShortcut("q", modifiers: [.command, .shift])
                }
                ForEach(presentation.attachments, id: \.name) { attachment in
                    Label {
                        Text(verbatim: attachment.name + " · " + ByteCountFormatter.string(
                            fromByteCount: Int64(attachment.size),
                            countStyle: .file
                        ))
                    } icon: {
                        Image(systemName: "paperclip")
                    }
                    .accessibilityLabel(Copy.format("reader.attachment", attachment.name))
                }
            }
            .font(MailuneType.body)
            .foregroundStyle(MailuneColor.ink)
            .padding(MailuneSpace.m)
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .background(MailuneColor.canvas)
    }
}

struct SecurityBadgeView: View {
    let badge: SecurityBadge

    var body: some View {
        Label(Copy.text(badge.titleKey), systemImage: symbol)
            .padding(.horizontal, MailuneSpace.s)
            .background(tint.opacity(0.15), in: Capsule())
    }

    private var symbol: String {
        switch badge {
        case .failed: "exclamationmark.shield"
        case .encrypted: "lock"
        case .signed: "signature"
        case .authenticated: "checkmark.shield"
        case .unverified: "questionmark.circle"
        }
    }

    private var tint: Color {
        badge == .failed ? .red : MailuneColor.accent
    }
}

struct RemoteBanner: View {
    let summary: RemoteSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Label(Copy.text("thread.images_blocked"), systemImage: "photo.badge.exclamationmark")
            if summary.trackerPixels > 0 {
                Text(Copy.format("reader.trackers", String(summary.trackerPixels)))
            }
        }
        .padding(MailuneSpace.s)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MailuneColor.accent.opacity(0.08), in: RoundedRectangle(cornerRadius: MailuneRadius.card))
    }
}
