import MailuneModel
import SwiftUI

/// A question over the fixture messages. The answer is shown only with the
/// message it came from, and opening the citation selects that thread.
struct AskView: View {
    var open: (Citation) -> Void
    @State private var question = ""
    @State private var answer: AskAnswer?
    @State private var asked = false

    var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            HStack {
                TextField(Copy.text("search.ask_prompt"), text: $question)
                    .onSubmit(ask)
                Button(Copy.text("search.ask"), action: ask)
                    .disabled(question.allSatisfy(\.isWhitespace))
            }
            if let answer {
                Text(verbatim: answer.text)
                ForEach(answer.citations, id: \.message) { citation in
                    Button {
                        open(citation)
                    } label: {
                        Label(source(citation), systemImage: "quote.bubble")
                    }
                    .accessibilityIdentifier("citation-\(citation.message)")
                }
            } else if asked {
                Text(Copy.text("search.no_answer"))
            }
            Spacer()
        }
        .font(MailuneType.body)
        .padding(MailuneSpace.m)
        .frame(minWidth: 420, minHeight: 220)
    }

    private func ask() {
        answer = FixtureAsk.answer(question, from: MessageFixtures.all)
        asked = true
    }

    private func source(_ citation: Citation) -> String {
        let subject = MessageFixtures.all.first { $0.id == citation.message }?.subject ?? citation.message
        return Copy.format("search.source", subject)
    }
}
