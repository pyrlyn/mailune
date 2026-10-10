import MailuneModel

/// Stand-ins for tests and previews. They record what the shell asked for
/// and touch no system service.
@MainActor
public final class FakeDockBadge: DockBadge {
    public var label: String?

    public init() {}
}

@MainActor
public final class FakeSharing: Sharing {
    public private(set) var shared: [String] = []

    public init() {}

    public func share(_ text: String) {
        shared.append(text)
    }
}

@MainActor
public final class FakeSpotlight: SpotlightIndex {
    public private(set) var items: [SpotlightItem] = []

    public init() {}

    public func replace(with items: [SpotlightItem]) {
        self.items = items
    }
}

extension HostServices {
    public static func fake() -> HostServices {
        HostServices(dock: FakeDockBadge(), sharing: FakeSharing(), spotlight: FakeSpotlight())
    }
}
