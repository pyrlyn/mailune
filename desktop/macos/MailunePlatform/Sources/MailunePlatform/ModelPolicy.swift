import Foundation

/// Work the phone may run locally. Embeddings are on unless a caller turns them off.
public enum OnDeviceJob: String, Equatable, Sendable {
    case embeddings
    case larger
}

/// Embeddings run by default. A larger job stays off until `allowLargerJobs` is set.
public struct ModelPolicy: Equatable, Sendable {
    public var allowLargerJobs: Bool

    public init(allowLargerJobs: Bool = false) {
        self.allowLargerJobs = allowLargerJobs
    }

    public func allows(_ job: OnDeviceJob) -> Bool {
        switch job {
        case .embeddings:
            true
        case .larger:
            allowLargerJobs
        }
    }
}

public enum OnDeviceJobs {
    /// Runs `job` through the scripted model when the policy allows it.
    public static func run(_ job: OnDeviceJob, policy: ModelPolicy, model: any OnDeviceModel, prompt: String) -> String? {
        guard policy.allows(job) else { return nil }
        return model.complete(prompt: prompt)
    }
}
