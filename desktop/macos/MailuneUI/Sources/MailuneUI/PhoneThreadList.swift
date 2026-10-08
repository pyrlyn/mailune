import MailuneModel
import SwiftUI

/// The phone list: swipe to archive, pull to refresh, and a single selection.
public struct PhoneThreadList: View {
    @Binding var state: PhoneListState

    public init(state: Binding<PhoneListState>) {
        _state = state
    }

    public var body: some View {
        List(state.rows(), selection: $state.selected) { item in
            Text(item.subject)
                .font(MailuneType.body)
                .tag(Optional(item.id))
                .accessibilityIdentifier("phone-thread-\(item.id)")
                .swipeActions(edge: .trailing) {
                    Button("Archive") { state.archive(item.id) }
                }
        }
        .refreshable { state.refresh() }
        .accessibilityIdentifier("phone-thread-list")
    }
}
