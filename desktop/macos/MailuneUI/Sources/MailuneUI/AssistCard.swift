import MailuneModel
import SwiftUI

/// The summary, its action items, where it was made, and reply chips.
struct AssistCard: View {
    let assist: ThreadAssist
    let reply: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.s) {
            Label(Copy.text("ai.summary"), systemImage: "sparkles")
                .fontWeight(.semibold)
            Text(verbatim: assist.summary)
                .accessibilityIdentifier("assist-summary")
            if !assist.actionItems.isEmpty {
                Text(Copy.text("ai.action_items"))
                    .fontWeight(.semibold)
                ForEach(assist.actionItems, id: \.self) { item in
                    Label {
                        Text(verbatim: item)
                    } icon: {
                        // Read aloud as "Selected", which an action item is not.
                        Image(systemName: "checkmark.circle")
                            .accessibilityHidden(true)
                    }
                }
            }
            Text(Copy.text(assist.source == .device ? Self.deviceKey : "ai.made_in_cloud"))
                .opacity(0.7)
            // Three chips squeezed into a phone-wide row wrap to a few words a
            // line, so they stack when the row does not fit.
            ViewThatFits(in: .horizontal) {
                HStack(spacing: MailuneSpace.s) { chips }
                VStack(alignment: .leading, spacing: MailuneSpace.s) { chips }
            }
        }
        .padding(MailuneSpace.s)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MailuneColor.accent.opacity(0.08), in: RoundedRectangle(cornerRadius: MailuneRadius.card))
        // One named group, so VoiceOver can skip past the AI text in one move.
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Copy.text("ai.summary"))
    }

    /// The on-device line names the Mac only where it is true.
    static var deviceKey: String {
        #if os(macOS)
            "ai.made_on_device"
        #else
            "ai.made_on_this_device"
        #endif
    }

    private var chips: some View {
        ForEach(assist.replies, id: \.self) { text in
            Button {
                reply(text)
            } label: {
                Text(verbatim: text)
            }
            .buttonStyle(.bordered)
            .accessibilityLabel(Copy.format("ai.reply_with", text))
        }
    }
}
