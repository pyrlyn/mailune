import MailuneModel
import SwiftUI

/// Recipient, subject, and body. Send stays disabled until the writer confirms.
public struct PhoneComposer: View {
    @State private var recipient = ""
    @State private var subject = ""
    @State private var message = ""
    @State private var confirmed = false
    @State private var savedID: String?
    @State private var store = FakePhoneDrafts()

    public init() {}

    public var body: some View {
        Form {
            TextField("Recipient", text: $recipient)
                .accessibilityIdentifier("phone-recipient")
            TextField("Subject", text: $subject)
                .accessibilityIdentifier("phone-subject")
            TextField("Body", text: $message, axis: .vertical)
                .accessibilityIdentifier("phone-body")
            Toggle("Confirm send", isOn: $confirmed)
            Button("Save") { save() }
                .accessibilityIdentifier("phone-save")
            Button("Send") { send() }
                .disabled(!confirmed || recipient.isEmpty)
                .accessibilityIdentifier("phone-send")
        }
        .accessibilityIdentifier("phone-composer")
    }

    private func save() {
        let draft = PhoneDraft(recipient: recipient, subject: subject, body: message, confirmed: false)
        savedID = store.save(draft)
    }

    private func send() {
        guard let id = savedID else { return }
        store.confirm(id)
        _ = store.send(id)
    }
}
