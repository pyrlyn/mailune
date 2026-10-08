import MailuneModel
import SwiftUI

/// Recipient chips, a body, one attachment, and send later. Send stays disabled
/// until the writer confirms.
public struct ComposerView: View {
    @State private var recipients: [String] = []
    @State private var chip = ""
    @State private var message = ""
    @State private var attachmentName = "agenda.txt"
    @State private var sendLater = false
    @State private var confirmed = false
    @State private var stagedID: String?
    @State private var restored = false
    @State private var outbox = FakeOutbox()

    public init() {}

    public var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            HStack {
                TextField("Recipient", text: $chip)
                    .font(MailuneType.body)
                    .accessibilityLabel("Recipient")
                Button("Add") { addChip() }
                    .accessibilityLabel("Add recipient")
            }
            FlowChips(recipients: recipients)
            TextEditor(text: $message)
                .font(MailuneType.body)
                .frame(minHeight: 120)
                .accessibilityLabel("Message body")
            Text(attachmentName)
                .font(MailuneType.body)
                .accessibilityLabel("Attachment \(attachmentName)")
            Toggle("Send later", isOn: $sendLater)
            Toggle("Confirm send", isOn: $confirmed)
            HStack {
                Button("Send") { send() }
                    .disabled(!confirmed || recipients.isEmpty)
                    .accessibilityLabel("Send message")
                Button("Undo send") { undo() }
                    .disabled(stagedID == nil)
            }
            if restored {
                Text("Send undone")
                    .font(MailuneType.body)
            }
        }
        .padding(MailuneSpace.m)
        .frame(minWidth: 420, minHeight: 360)
    }

    private func addChip() {
        let trimmed = chip.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        recipients.append(trimmed)
        chip = ""
    }

    private func send() {
        let draft = ComposerDraft(
            recipients: recipients,
            body: message,
            attachmentName: attachmentName,
            sendLater: sendLater ? Date(timeIntervalSince1970: 1_700_000_000) : nil,
            confirmed: false
        )
        let id = outbox.stage(draft)
        outbox.confirm(id)
        if outbox.send(id) {
            stagedID = id
            restored = false
        }
    }

    private func undo() {
        guard let id = stagedID, outbox.undo(id) != nil else { return }
        stagedID = nil
        restored = true
    }
}

private struct FlowChips: View {
    var recipients: [String]

    var body: some View {
        HStack(spacing: MailuneSpace.s) {
            ForEach(recipients, id: \.self) { recipient in
                Text(recipient)
                    .font(MailuneType.body)
                    .padding(.horizontal, MailuneSpace.s)
                    .background(MailuneColor.canvas, in: Capsule())
            }
        }
    }
}
