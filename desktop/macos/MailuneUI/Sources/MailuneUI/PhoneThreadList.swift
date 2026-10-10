#if os(iOS)
    import MailuneModel
    import SwiftUI

    /// The phone list: tap pushes the thread, swipe archives, pull refreshes,
    /// and Edit selects several rows to archive together.
    struct PhoneThreadList: View {
        let feed: ThreadFeed
        @State private var category = ThreadCategory.primary
        @State private var selection: Set<String> = []
        @Environment(\.editMode) private var editMode

        var body: some View {
            // Bound only while editing: a list with a selection turns a tap into
            // a selection, and the row would never push its reader.
            List(feed.visible(category: category), selection: editMode?.wrappedValue.isEditing == true ? $selection : nil) { item in
                NavigationLink(value: item.id) {
                    ThreadRowView(item: item)
                }
                .swipeActions(edge: .trailing) {
                    Button(Copy.text("data.archive")) { feed.archive([item.id]) }
                        .tint(MailuneColor.accent)
                }
            }
            .listStyle(.plain)
            .refreshable { await feed.refresh() }
            .safeAreaInset(edge: .top) {
                VStack(spacing: MailuneSpace.s) {
                    Picker(Copy.text("mail_list.category"), selection: $category) {
                        ForEach(ThreadCategory.allCases, id: \.self) { tab in
                            Text(Copy.text(tab.titleKey)).tag(tab)
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    if feed.refreshFailed {
                        Text(Copy.text("mail_list.refresh_failed"))
                            .foregroundStyle(.red)
                    }
                }
                .padding(.horizontal, MailuneSpace.s)
                .background(.bar)
            }
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    EditButton()
                }
                if editMode?.wrappedValue.isEditing == true {
                    ToolbarItem(placement: .bottomBar) {
                        Button(Copy.text("data.archive")) {
                            feed.archive(selection)
                            selection = []
                        }
                        .disabled(selection.isEmpty)
                    }
                }
            }
        }
    }
#endif
