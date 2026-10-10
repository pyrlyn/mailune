import MailuneModel
import SwiftUI

/// Category tabs over a `List`, which only builds the rows on screen. Unread
/// rows carry a dot, a trailing swipe archives, and selection is multiple.
/// A non-empty `query` searches every tab, as a mail search does.
public struct ThreadList: View {
    @Binding var selected: Set<String>
    var feed: ThreadFeed
    var query: String
    @State private var category = ThreadCategory.primary

    public init(selected: Binding<Set<String>>, feed: ThreadFeed, query: String = "") {
        _selected = selected
        self.feed = feed
        self.query = query
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.s) {
            if query.allSatisfy(\.isWhitespace) {
                Picker(Copy.text("mail_list.category"), selection: $category) {
                    ForEach(ThreadCategory.allCases, id: \.self) { tab in
                        Text(Copy.text(tab.titleKey)).tag(tab)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .padding(.horizontal, MailuneSpace.s)
                rows(feed.visible(category: category))
            } else {
                switch Result(catching: { () throws(SearchError) in try SearchQuery.parse(query) }) {
                case let .success(search):
                    Text(Copy.format("search.results_for", query))
                        .padding(.horizontal, MailuneSpace.s)
                    rows(search.narrow(feed.visible(category: nil)))
                case let .failure(error):
                    Text(Self.message(for: error))
                        .foregroundStyle(.red)
                        .padding(.horizontal, MailuneSpace.s)
                    Spacer()
                }
            }
        }
        .font(MailuneType.body)
    }

    private func rows(_ items: [ThreadItem]) -> some View {
        List(items, selection: $selected) { item in
            ThreadRowView(item: item)
                .tag(item.id)
                .swipeActions(edge: .trailing) {
                    Button(Copy.text("data.archive")) {
                        feed.archive([item.id])
                        selected.remove(item.id)
                    }
                }
        }
    }

    nonisolated static func message(for error: SearchError) -> String {
        switch error {
        case let .unsupported(token): Copy.format("search.unsupported", token)
        case let .bad(token): Copy.format("search.bad_query", token)
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
                    .accessibilityIdentifier("thread-\(item.id)")
                Text(item.snippet).lineLimit(1).opacity(0.7)
            }
            .font(MailuneType.body)
            .foregroundStyle(MailuneColor.ink)
        }
    }
}
