import MailuneModel
import SwiftUI

/// Category tabs over a `List`, which only builds the rows on screen. Unread
/// rows carry a dot, a trailing swipe archives, and selection is multiple.
public struct ThreadList: View {
    @Binding var selected: Set<String>
    @State private var category = ThreadCategory.primary
    @State private var archived: Set<String> = []

    public init(selected: Binding<Set<String>>) {
        _selected = selected
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.s) {
            Picker(Copy.text("mail_list.category"), selection: $category) {
                ForEach(ThreadCategory.allCases, id: \.self) { tab in
                    Text(Copy.text(tab.titleKey)).tag(tab)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .padding(.horizontal, MailuneSpace.s)

            List(ThreadFixtures.visible(category: category, archived: archived), selection: $selected) { item in
                ThreadRowView(item: item)
                    .tag(item.id)
                    .swipeActions(edge: .trailing) {
                        Button(Copy.text("data.archive")) {
                            archived.insert(item.id)
                            selected.remove(item.id)
                        }
                    }
            }
        }
    }
}

struct ThreadRowView: View {
    let item: ThreadItem

    var body: some View {
        HStack(alignment: .top, spacing: MailuneSpace.s) {
            if item.unread {
                Circle()
                    .fill(MailuneColor.accent)
                    .frame(width: MailuneSpace.s, height: MailuneSpace.s)
                    .accessibilityLabel(Copy.text("search.unread"))
            } else {
                Color.clear
                    .frame(width: MailuneSpace.s, height: MailuneSpace.s)
                    .accessibilityHidden(true)
            }
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(item.from.display).fontWeight(item.unread ? .semibold : .regular)
                    Spacer()
                    if item.hasAttachment {
                        Image(systemName: "paperclip")
                            .accessibilityLabel(Copy.text("sheets.has_attachment"))
                    }
                    Text(item.stamp)
                }
                Text(item.subject)
                Text(item.snippet).lineLimit(1).opacity(0.7)
            }
            .font(MailuneType.body)
            .foregroundStyle(MailuneColor.ink)
        }
    }
}
