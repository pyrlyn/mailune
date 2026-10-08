import Foundation

/// Summary and reply chips shown beside a message. They come from fixtures, not a model.
public struct ReaderAssist: Equatable, Sendable {
    public var summary: String
    public var replies: [String]

    public init(summary: String, replies: [String]) {
        self.summary = summary
        self.replies = replies
    }
}

public enum AssistFixtures {
    public static func assist(forThread id: String) -> ReaderAssist? {
        switch id {
        case "1":
            ReaderAssist(summary: "Ana suggests meeting tomorrow.", replies: ["Sounds good", "Not then"])
        default:
            nil
        }
    }
}

/// The line settings shows about where summaries stay.
public enum PrivacyCopy {
    public static let summariesStayLocal = "Summaries stay on this Mac."
}
