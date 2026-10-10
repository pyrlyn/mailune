import AppIntents
import Foundation
import MailuneModel

/// Lists the intents below so an app that links this package exposes them.
public struct MailuneIntentsPackage: AppIntentsPackage {}

// Titles are catalog keys looked up in this package's bundle, which holds the
// catalogs; the app bundles hold none. AppIntents reads them at build time, so
// each must be a literal initializer call.

/// A thread as Shortcuts and Siri pick it. It carries the subject and sender
/// only, so no message text leaves the app inside an entity.
public struct MailThreadEntity: AppEntity, Equatable {
    public static let typeDisplayRepresentation = TypeDisplayRepresentation(name: LocalizedStringResource("intent.conversation", bundle: #bundle))
    public static var defaultQuery: MailThreadQuery { MailThreadQuery() }

    public let id: String
    public let subject: String
    public let sender: String

    public init(_ item: ThreadItem) {
        id = item.id
        subject = item.subject
        sender = item.from.display
    }

    public var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "\(subject)", subtitle: "\(sender)")
    }
}

public struct MailThreadQuery: EntityStringQuery {
    public init() {}

    public func entities(for identifiers: [String]) async throws -> [MailThreadEntity] {
        MailIntents.rows().filter { identifiers.contains($0.id) }.map(MailThreadEntity.init)
    }

    public func entities(matching string: String) async throws -> [MailThreadEntity] {
        try MailIntents.search(string)
    }

    public func suggestedEntities() async throws -> [MailThreadEntity] {
        MailIntents.rows().map(MailThreadEntity.init)
    }
}

public enum MailIntentError: Error, Equatable, CustomLocalizedStringResourceConvertible {
    case noSummary
    case search(SearchError)

    public var localizedStringResource: LocalizedStringResource {
        switch self {
        case .noSummary: LocalizedStringResource("intent.no_summary", bundle: #bundle)
        case let .search(error): "\(ThreadList.message(for: error))"
        }
    }
}

/// What the intents do, apart from the AppIntents types, so a test can call
/// it on fixture data. The rows are the fixtures until the core is wired.
public enum MailIntents {
    static func rows() -> [ThreadItem] {
        ThreadFixtures.all
    }

    /// Through `AssistPolicy`, so cloud text about encrypted mail is never handed out.
    public static func summary(of thread: String) throws(MailIntentError) -> String {
        guard let message = MessageFixtures.message(forThread: thread),
              let assist = AssistPolicy.fixture(for: message)
        else {
            throw .noSummary
        }
        return assist.summary
    }

    /// The thread list's own grammar over the fields the list shows, so an
    /// encrypted body is never searched here.
    public static func search(_ query: String) throws(MailIntentError) -> [MailThreadEntity] {
        do {
            return try SearchQuery.parse(query).narrow(rows()).map(MailThreadEntity.init)
        } catch {
            throw .search(error)
        }
    }

    /// Typed addresses go through the composer's own splitter, and what is not
    /// an address is dropped, as a `mailto:` link's would be.
    public static func draft(to: String?, subject: String?, body: String?) -> Draft {
        var draft = Draft()
        draft.to = Composer.addresses(in: to ?? "").accepted
        draft.subject = subject ?? ""
        draft.body = body ?? ""
        return draft
    }
}

public struct SummarizeThreadIntent: AppIntent {
    public static let title = LocalizedStringResource("intent.summarize", bundle: #bundle)

    @Parameter(title: LocalizedStringResource("intent.conversation", bundle: #bundle))
    public var thread: MailThreadEntity

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<String> & ProvidesDialog {
        let summary = try MailIntents.summary(of: thread.id)
        return .result(value: summary, dialog: "\(summary)")
    }
}

public struct SearchMailIntent: AppIntent {
    public static let title = LocalizedStringResource("search.search_in_mail", bundle: #bundle)

    @Parameter(title: LocalizedStringResource("intent.query", bundle: #bundle))
    public var query: String

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<[MailThreadEntity]> {
        .result(value: try MailIntents.search(query))
    }
}

/// Opens the composer with a draft. It cannot send: the draft goes to
/// `ComposeRequests`, and only the composer's confirm step sends.
public struct ComposeMessageIntent: AppIntent {
    public static let title = LocalizedStringResource("app.compose", bundle: #bundle)
    public static let openAppWhenRun = true

    @Parameter(title: LocalizedStringResource("compose.to", bundle: #bundle))
    public var to: String?

    @Parameter(title: LocalizedStringResource("compose.subject", bundle: #bundle))
    public var subject: String?

    @Parameter(title: LocalizedStringResource("app.message", bundle: #bundle))
    public var body: String?

    /// The app registers it with `AppDependencyManager`; a test sets it.
    @Dependency var requests: ComposeRequests

    public init() {}

    @MainActor
    public func perform() async throws -> some IntentResult {
        requests.open(MailIntents.draft(to: to, subject: subject, body: body))
        return .result()
    }
}
