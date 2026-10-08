import Foundation

/// Builds a mailto link. The fake records it and does not open an application.
public protocol MailtoHandler: Sendable {
    func compose(address: String, subject: String) -> URL?
    func lastAddress() -> String?
}

/// The number drawn on the Dock icon. The fake stores the count.
public protocol DockBadge: Sendable {
    func setCount(_ count: Int)
    func count() -> Int
}

/// A share sheet. The fake keeps the text that would have been shared.
public protocol ShareSheet: Sendable {
    func share(text: String)
    func shared() -> [String]
}

/// Spotlight items. The fake keeps titles by fixture id and does not write an index.
public protocol SpotlightIndex: Sendable {
    func index(id: String, title: String)
    func title(for id: String) -> String?
}

public final class FakeMailto: MailtoHandler, @unchecked Sendable {
    private let lock = NSLock()
    private var address: String?

    public init() {}

    public func compose(address: String, subject: String) -> URL? {
        let allowed = CharacterSet.urlQueryAllowed
        let query = subject.addingPercentEncoding(withAllowedCharacters: allowed) ?? subject
        guard let url = URL(string: "mailto:\(address)?subject=\(query)") else { return nil }
        lock.lock()
        self.address = address
        lock.unlock()
        return url
    }

    public func lastAddress() -> String? {
        lock.lock()
        defer { lock.unlock() }
        return address
    }
}

public final class FakeDockBadge: DockBadge, @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0

    public init() {}

    public func setCount(_ count: Int) {
        lock.lock()
        value = count
        lock.unlock()
    }

    public func count() -> Int {
        lock.lock()
        defer { lock.unlock() }
        return value
    }
}

public final class FakeShareSheet: ShareSheet, @unchecked Sendable {
    private let lock = NSLock()
    private var items: [String] = []

    public init() {}

    public func share(text: String) {
        lock.lock()
        items.append(text)
        lock.unlock()
    }

    public func shared() -> [String] {
        lock.lock()
        defer { lock.unlock() }
        return items
    }
}

public final class FakeSpotlight: SpotlightIndex, @unchecked Sendable {
    private let lock = NSLock()
    private var titles: [String: String] = [:]

    public init() {}

    public func index(id: String, title: String) {
        lock.lock()
        titles[id] = title
        lock.unlock()
    }

    public func title(for id: String) -> String? {
        lock.lock()
        defer { lock.unlock() }
        return titles[id]
    }
}
