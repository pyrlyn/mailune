import MailuneModel
import SwiftUI

/// The phone list: swipe to archive, pull to refresh, and a single selection.
public struct PhoneThreadList: View {
    @Binding var state: PhoneListState
    @Binding var opened: String?

    public init(state: Binding<PhoneListState>, opened: Binding<String?>) {
        _state = state
        _opened = opened
    }

    public var body: some View {
        List(state.rows()) { item in
            Button {
                state.selected = item.id
                opened = item.id
            } label: {
                Text(item.subject)
                    .font(MailuneType.body)
                    .foregroundStyle(MailuneColor.ink)
            }
            .accessibilityIdentifier("phone-thread-\(item.id)")
            .swipeActions(edge: .trailing) {
                Button("Archive") { state.archive(item.id) }
            }
        }
        .refreshable { state.refresh() }
        .accessibilityIdentifier("phone-thread-list")
    }
}
