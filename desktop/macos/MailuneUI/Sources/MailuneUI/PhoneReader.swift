import MailuneModel
import SwiftUI

/// A fixture message on the phone. There is no web view, so remote content stays off.
public struct PhoneReader: View {
    private let message: MailMessage

    public init(threadID: String) {
        message = MessageFixtures.message(forThread: threadID) ?? MailMessage(
            id: threadID,
            subject: "Message",
            body: "No body",
            quoted: "",
            attachmentName: "",
            badge: "Unchecked"
        )
    }

    public var body: some View {
        let presentation = ReaderPresentation(message: message, quotesCollapsed: true)
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            Text(message.subject)
                .font(MailuneType.title)
                .foregroundStyle(MailuneColor.ink)
            Text(presentation.text)
                .font(MailuneType.body)
                .accessibilityIdentifier("phone-reader-body")
            Text(presentation.remoteContentEnabled ? "Remote content on" : "Remote content off")
                .font(MailuneType.body)
                .accessibilityIdentifier("phone-remote-content")
        }
        .padding(MailuneSpace.m)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(MailuneColor.canvas)
        .accessibilityIdentifier("phone-reader")
    }
}
