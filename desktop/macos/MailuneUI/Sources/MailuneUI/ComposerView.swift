import MailuneModel
import SwiftUI
import UniformTypeIdentifiers

/// Recipient chips, subject, body, one attachment and send later. Send asks
/// for confirmation first, then the draft is held so it can be undone.
public struct ComposerView: View {
    @State private var composer: Composer
    private let outbox: FakeOutbox
    @State private var typed = ""
    @State private var rejected: [String] = []
    @State private var picking = false
    @Environment(\.dismiss) private var dismiss

    /// The fake outbox is ticked here because nothing else runs its clock
    /// until the core's queue replaces it.
    public init(outbox: FakeOutbox, undoWindow: TimeInterval = 10, draft: Draft = Draft()) {
        self.outbox = outbox
        _composer = State(initialValue: Composer(outbox: outbox, undoWindow: undoWindow, draft: draft))
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            VStack(alignment: .leading, spacing: MailuneSpace.m) {
                recipients
                TextField(Copy.text("compose.subject"), text: $composer.draft.subject)
                TextEditor(text: $composer.draft.body)
                    .frame(minHeight: 160)
                    .overlay(alignment: .topLeading) {
                        if composer.draft.body.isEmpty {
                            Text(Copy.text("compose.write_your_message"))
                                .opacity(0.5)
                                .allowsHitTesting(false)
                        }
                    }
                attachment
                Toggle(Copy.text("sheets.schedule_send"), isOn: scheduled)
                if composer.draft.sendAt != nil {
                    DatePicker(Copy.text("sheets.schedule_send"), selection: sendAt, in: Date()...)
                        .labelsHidden()
                }
            }
            .disabled(isHeld)
            TimelineView(.periodic(from: .now, by: 1)) { context in
                footer(now: context.date)
                    .onChange(of: context.date) { _, now in
                        outbox.release(now: now)
                        if composer.settle(now: now) {
                            dismiss()
                        }
                    }
            }
        }
        .font(MailuneType.body)
        .padding(MailuneSpace.m)
        .frame(minWidth: 480, minHeight: 420)
        .interactiveDismissDisabled(isHeld)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button(Copy.text("compose.discard")) { dismiss() }
                    .disabled(isHeld)
            }
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.item]) { result in
            if case let .success(url) = result {
                attach(url)
            }
        }
        .confirmationDialog(Copy.text("compose.confirm_send"), isPresented: confirming) {
            Button(Copy.text("compose.send")) { composer.confirmSend(now: Date()) }
            Button(Copy.text("sheets.cancel"), role: .cancel) { composer.cancelSend() }
        }
    }

    private var recipients: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.s) {
            HStack(spacing: MailuneSpace.s) {
                Text(Copy.text("compose.to"))
                ForEach(composer.draft.to, id: \.self) { address in
                    HStack(spacing: 4) {
                        Text(verbatim: address)
                        Button {
                            composer.removeRecipient(address)
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel(Copy.format("compose.remove", address))
                    }
                    .padding(.horizontal, MailuneSpace.s)
                    .background(MailuneColor.accent.opacity(0.15), in: Capsule())
                }
                TextField(Copy.text("compose.recipients"), text: $typed)
                    .onSubmit {
                        rejected = composer.addRecipients(typed)
                        typed = rejected.joined(separator: ", ")
                    }
            }
            if !rejected.isEmpty {
                Text(Copy.format("compose.invalid_recipient", rejected.joined(separator: ", ")))
                    .foregroundStyle(.red)
            }
        }
    }

    @ViewBuilder
    private var attachment: some View {
        if let file = composer.draft.attachment {
            HStack(spacing: MailuneSpace.s) {
                Label {
                    Text(verbatim: file.name)
                } icon: {
                    Image(systemName: "paperclip")
                }
                Button {
                    composer.draft.attachment = nil
                } label: {
                    Image(systemName: "xmark.circle.fill")
                }
                .buttonStyle(.plain)
                .accessibilityLabel(Copy.format("compose.remove", file.name))
            }
        } else {
            Button(Copy.text("compose.attach")) { picking = true }
        }
    }

    @ViewBuilder
    private func footer(now: Date) -> some View {
        HStack(spacing: MailuneSpace.s) {
            if case .held = composer.stage {
                if let sendAt = composer.draft.sendAt, sendAt > now {
                    Text(Copy.format("compose.scheduled_for", sendAt.formatted(date: .abbreviated, time: .shortened)))
                } else {
                    Text(Copy.text("compose.sending"))
                }
                Button(Copy.text("auth.undo")) { composer.undo(now: now) }
                    .disabled(!composer.canUndo(now: now))
            } else {
                Spacer()
                Button(Copy.text("compose.send")) { composer.requestSend() }
                    .disabled(!composer.canSend)
                    .keyboardShortcut(.return, modifiers: .command)
            }
        }
    }

    private var isHeld: Bool {
        if case .held = composer.stage { true } else { false }
    }

    private var confirming: Binding<Bool> {
        Binding(
            get: { composer.stage == .confirming },
            set: { shown in
                if !shown {
                    composer.cancelSend()
                }
            }
        )
    }

    private var scheduled: Binding<Bool> {
        Binding(
            get: { composer.draft.sendAt != nil },
            set: { on in
                composer.draft.sendAt = on ? Date().addingTimeInterval(3600) : nil
            }
        )
    }

    private var sendAt: Binding<Date> {
        Binding(
            get: { composer.draft.sendAt ?? Date() },
            set: { composer.draft.sendAt = $0 }
        )
    }

    private func attach(_ url: URL) {
        let scoped = url.startAccessingSecurityScopedResource()
        defer {
            if scoped {
                url.stopAccessingSecurityScopedResource()
            }
        }
        let size = (try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0
        composer.draft.attachment = Attachment(name: url.lastPathComponent, size: size)
    }
}
