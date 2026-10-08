import MailuneModel
import SwiftUI

/// Local ICS suggestions from the bundled fixture. Not a JMAP calendar.
public struct CalendarView: View {
    private let suggestions: [CalendarSuggestion]

    public init(suggestions: [CalendarSuggestion] = CalendarSuggestions.load()) {
        self.suggestions = suggestions
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            Text("Suggestions")
                .font(MailuneType.title)
                .foregroundStyle(MailuneColor.ink)
            if suggestions.isEmpty {
                Text("No suggestions")
                    .font(MailuneType.body)
                    .foregroundStyle(MailuneColor.ink)
            } else {
                ForEach(suggestions) { item in
                    suggestionRow(item)
                }
            }
        }
        .padding(MailuneSpace.m)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(MailuneColor.canvas)
    }

    private func suggestionRow(_ item: CalendarSuggestion) -> some View {
        VStack(alignment: .leading, spacing: MailuneSpace.s) {
            Text(item.summary)
                .font(MailuneType.body)
                .foregroundStyle(MailuneColor.ink)
            Text(item.start)
                .font(MailuneType.body)
                .foregroundStyle(MailuneColor.accent)
            if !item.location.isEmpty {
                Text(item.location)
                    .font(MailuneType.body)
                    .foregroundStyle(MailuneColor.ink)
            }
        }
        .padding(MailuneSpace.s)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MailuneColor.canvas, in: RoundedRectangle(cornerRadius: MailuneRadius.card))
        .accessibilityElement(children: .combine)
    }
}
