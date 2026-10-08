import Foundation
import FoundationModels

/// A local model. Tests use a scripted stand-in so they never call the system model.
public protocol OnDeviceModel: Sendable {
    func complete(prompt: String) -> String
}

/// Returns a fixed JSON document for any non-empty prompt.
public struct ScriptedOnDeviceModel: OnDeviceModel, Sendable {
    public var document: String

    public init(document: String) {
        self.document = document
    }

    public func complete(prompt: String) -> String {
        guard !prompt.isEmpty else { return #"{"summary":""}"# }
        return document
    }
}

/// JSON the scripted model is expected to return.
public struct ModelReply: Codable, Equatable, Sendable {
    public var summary: String

    public init(summary: String) {
        self.summary = summary
    }
}

public enum OnDeviceSession {
    /// Whether this Mac can run the system model. Callers still use a scripted model in tests.
    public static func systemModelIsAvailable() -> Bool {
        if case .available = SystemLanguageModel.default.availability {
            return true
        }
        return false
    }

    public static func reply(prompt: String, model: any OnDeviceModel) throws -> ModelReply {
        let text = model.complete(prompt: prompt)
        return try JSONDecoder().decode(ModelReply.self, from: Data(text.utf8))
    }
}
