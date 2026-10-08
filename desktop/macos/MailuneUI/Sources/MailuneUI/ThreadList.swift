import MailuneModel
import SwiftUI

/// A lazy thread list: category tabs, an unread mark, swipe to archive, and multi-select.
public struct ThreadList: View {
    @Binding var selected: Set<String>
    @State private var category = "Primary"
    @State private var archived: Set<String> = []

    public init(selected: Binding<Set<String>>) {
        _selected = selected
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.s) {
            Picker("Category", selection: $category) {
                ForEach(ThreadFixtures.categories, id: \.self) { name in
                    Text(name).tag(name)
                }
            }
            .pickerStyle(.segmented)
            .padding(.horizontal, MailuneSpace.s)

            List(ThreadFixtures.visible(category: category, archived: archived), selection: $selected) { item in
                HStack(spacing: MailuneSpace.s) {
                    if item.unread {
                        Circle()
                            .fill(MailuneColor.accent)
                            .frame(width: MailuneSpace.s, height: MailuneSpace.s)
                            .accessibilityLabel("Unread")
                    }
                    Text(item.subject)
                        .font(MailuneType.body)
                        .foregroundStyle(MailuneColor.ink)
                }
                .tag(item.id)
                .swipeActions(edge: .trailing) {
                    Button("Archive") { archived.insert(item.id) }
                }
            }
        }
    }
}
