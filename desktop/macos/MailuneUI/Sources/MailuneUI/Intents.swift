import AppIntents
import MailuneModel

/// Fixture-backed actions the intents call. No model and no network.
public enum MailIntents {
    public static func summarise(threadID: String) -> String {
        MessageFixtures.message(forThread: threadID)?.body ?? ""
    }

    public static func search(query: String) -> [String] {
        let token = SearchToken(id: "query", field: "subject", value: query)
        return ThreadSearch.narrow(ThreadFixtures.all, tokens: [token]).map(\.id)
    }

    public static func compose(recipient: String, body: String) -> ComposerDraft {
        ComposerDraft(
            recipients: [recipient],
            body: body,
            attachmentName: nil,
            sendLater: nil,
            confirmed: false
        )
    }
}

public struct SummariseIntent: AppIntent {
    public static var title: LocalizedStringResource { "Summarise" }
    public static var openAppWhenRun: Bool { false }

    @Parameter(title: "Thread")
    public var threadID: String

    public init() {
        threadID = ""
    }

    public init(threadID: String) {
        self.threadID = threadID
    }

    public func perform() async throws -> some ReturnsValue<String> {
        .result(value: MailIntents.summarise(threadID: threadID))
    }
}

public struct SearchIntent: AppIntent {
    public static var title: LocalizedStringResource { "Search" }
    public static var openAppWhenRun: Bool { false }

    @Parameter(title: "Query")
    public var query: String

    public init() {
        query = ""
    }

    public init(query: String) {
        self.query = query
    }

    public func perform() async throws -> some ReturnsValue<String> {
        .result(value: MailIntents.search(query: query).joined(separator: ","))
    }
}

public struct ComposeIntent: AppIntent {
    public static var title: LocalizedStringResource { "Compose" }
    public static var openAppWhenRun: Bool { false }

    @Parameter(title: "Recipient")
    public var recipient: String

    @Parameter(title: "Body")
    public var body: String

    public init() {
        recipient = ""
        body = ""
    }

    public init(recipient: String, body: String) {
        self.recipient = recipient
        self.body = body
    }

    public func perform() async throws -> some ReturnsValue<String> {
        let draft = MailIntents.compose(recipient: recipient, body: body)
        return .result(value: draft.confirmed ? "" : draft.body)
    }
}
