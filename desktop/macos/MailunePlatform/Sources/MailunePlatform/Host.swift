/// The macOS host the shell runs on. Integrations arrive behind protocols later.
public enum Host {
    /// The CPU slice of this build. Intel Macs are not supported, so anything
    /// else means a build setting let an x86_64 slice through.
    public static var architecture: String {
        #if arch(arm64)
            "arm64"
        #elseif arch(x86_64)
            "x86_64"
        #else
            "unknown"
        #endif
    }
}
