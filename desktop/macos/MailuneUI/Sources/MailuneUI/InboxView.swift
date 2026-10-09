import SwiftUI

/// The first screen until the shell lands. It uses every token family, so a
/// renamed token breaks the build here.
public struct InboxView: View {
    @State private var highlighted = false

    public init() {}

    public var body: some View {
        Text(Copy.text("app.inbox"))
            .font(MailuneType.title)
            .foregroundStyle(highlighted ? MailuneColor.accent : MailuneColor.ink)
            .padding(MailuneSpace.m)
            .background(MailuneColor.canvas, in: RoundedRectangle(cornerRadius: MailuneRadius.card))
            .animation(MailuneMotion.quick, value: highlighted)
            .onTapGesture { highlighted.toggle() }
    }
}
