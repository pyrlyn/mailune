/// The macOS host the shell talks to. Integrations arrive behind protocols later.
public struct Host: Sendable, Equatable {
    /// Which operating system this build is for.
    public var system: String

    public init(system: String = "macOS") {
        self.system = system
    }
}
