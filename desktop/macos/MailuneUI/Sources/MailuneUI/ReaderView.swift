import MailuneModel
import SwiftUI

/// Renders one message as text. Quotes start collapsed. There is no web view,
/// so remote content and JavaScript cannot run.
public struct ReaderView: View {
    private let message: MailMessage
    @State private var quotesCollapsed = true

    public init(message: MailMessage) {
        self.message = message
    }

    public var body: some View {
        let presentation = ReaderPresentation(message: message, quotesCollapsed: quotesCollapsed)
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            HStack {
                Text(message.subject)
                    .font(MailuneType.title)
                    .foregroundStyle(MailuneColor.ink)
                Text(presentation.badge)
                    .font(MailuneType.body)
                    .padding(.horizontal, MailuneSpace.s)
                    .background(MailuneColor.accent.opacity(0.15), in: Capsule())
                    .accessibilityLabel("Security \(presentation.badge)")
            }
            Text(presentation.text)
                .font(MailuneType.body)
                .foregroundStyle(MailuneColor.ink)
                .accessibilityLabel("Message body")
            Button(quotesCollapsed ? "Show quote" : "Hide quote") {
                quotesCollapsed.toggle()
            }
            .font(MailuneType.body)
            .accessibilityLabel("Quoted text")
            .keyboardShortcut("q", modifiers: .command)
            Text(presentation.attachmentName)
                .font(MailuneType.body)
                .accessibilityLabel("Attachment \(presentation.attachmentName)")
            Text("Remote content off")
                .font(MailuneType.body)
            Text("JavaScript off")
                .font(MailuneType.body)
            if let assist = AssistFixtures.assist(forThread: message.id) {
                Text(assist.summary)
                    .font(MailuneType.body)
                    .accessibilityLabel("Summary")
                HStack(spacing: MailuneSpace.s) {
                    ForEach(assist.replies, id: \.self) { reply in
                        Text(reply)
                            .font(MailuneType.body)
                            .padding(.horizontal, MailuneSpace.s)
                            .background(MailuneColor.accent.opacity(0.15), in: Capsule())
                            .accessibilityLabel("Reply \(reply)")
                    }
                }
            }
        }
        .padding(MailuneSpace.m)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(MailuneColor.canvas)
    }
}
