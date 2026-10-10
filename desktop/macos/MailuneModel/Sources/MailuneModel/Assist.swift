import Foundation

/// Where an AI text was made, so the reader can say so.
public enum AssistSource: String, Decodable, Sendable {
    case device, cloud
}

/// A thread's short summary, action items and three reply suggestions: the
/// shapes of `mailune-ai`'s summary cache and smart replies. Fixture text
/// until the core is wired; nothing here calls a model.
public struct ThreadAssist: Decodable, Equatable, Sendable {
    public var thread: String
    public var summary: String
    public var actionItems: [String]
    public var replies: [String]
    public var source: AssistSource
}

public enum AssistFixtures {
    public static let all: [ThreadAssist] = load()

    public static func assist(forThread id: String) -> ThreadAssist? {
        all.first { $0.thread == id }
    }

    static func decode(_ data: Data) throws -> [ThreadAssist] {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode([ThreadAssist].self, from: data)
    }

    private static func load() -> [ThreadAssist] {
        guard let url = Bundle.module.url(forResource: "assist", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let assists = try? decode(data)
        else {
            return []
        }
        return assists
    }
}

public enum AssistPolicy {
    /// Encrypted mail never goes to a cloud model, so a cloud-made text about
    /// an encrypted message is a sign something broke that rule: hide it.
    public static func visible(_ assist: ThreadAssist?, for message: MailMessage) -> ThreadAssist? {
        guard let assist, assist.thread == message.thread else { return nil }
        if message.security.encrypted && assist.source == .cloud {
            return nil
        }
        return assist
    }

    /// The fixture assist for `message`, already through the rule above.
    public static func fixture(for message: MailMessage) -> ThreadAssist? {
        visible(AssistFixtures.assist(forThread: message.thread), for: message)
    }
}

extension Draft {
    /// A reply to the sender with `body`. The prefix stays "Re:" in every
    /// language because a translated prefix breaks threading in other clients.
    public static func reply(to message: MailMessage, body: String) -> Draft {
        var draft = Draft()
        draft.to = [message.from.email]
        let subject = message.subject
        draft.subject = subject.lowercased().hasPrefix("re:") ? subject : "Re: " + subject
        draft.body = body
        return draft
    }
}
