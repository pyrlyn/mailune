import MailuneModel
import SwiftUI

/// Filter tokens over the fixture list, and an ask box whose answer cites a fixture id.
public struct SearchView: View {
    @State private var field = "subject"
    @State private var value = ""
    @State private var tokens: [SearchToken] = []
    @State private var question = ""
    @State private var citation: Citation?
    @State private var nextID = 1

    public init() {}

    public var body: some View {
        let rows = ThreadSearch.narrow(ThreadFixtures.all, tokens: tokens)
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            HStack {
                Picker("Field", selection: $field) {
                    Text("Subject").tag("subject")
                    Text("Category").tag("category")
                }
                TextField("Value", text: $value)
                Button("Add token") { addToken() }
            }
            HStack {
                ForEach(tokens) { token in
                    Text("\(token.field): \(token.value)")
                        .font(MailuneType.body)
                        .padding(.horizontal, MailuneSpace.s)
                        .background(MailuneColor.canvas, in: Capsule())
                }
            }
            ForEach(rows) { row in
                Text(row.subject)
                    .font(MailuneType.body)
            }
            TextField("Ask", text: $question)
            Button("Ask") {
                citation = ThreadSearch.cite(question: question, in: rows)
            }
            if let citation {
                Text("Citation \(citation.fixtureID): \(citation.excerpt)")
                    .font(MailuneType.body)
                    .accessibilityLabel("Citation \(citation.fixtureID)")
            }
        }
        .padding(MailuneSpace.m)
        .frame(minWidth: 420, minHeight: 320)
    }

    private func addToken() {
        let trimmed = value.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        tokens.append(SearchToken(id: String(nextID), field: field, value: trimmed))
        nextID += 1
        value = ""
    }
}
