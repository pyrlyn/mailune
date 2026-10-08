import SwiftUI

/// The first screen. It is also the view that proves the tokens compile.
public struct InboxView: View {
    @State private var highlighted = false

    public init() {}

    public var body: some View {
        Text("Inbox")
            .font(MailuneType.title)
            .foregroundStyle(highlighted ? MailuneColor.accent : MailuneColor.ink)
            .padding(MailuneSpace.m)
            .background(MailuneColor.canvas, in: RoundedRectangle(cornerRadius: MailuneRadius.card))
            .animation(MailuneMotion.quick, value: highlighted)
            .onTapGesture { highlighted.toggle() }
    }
}
