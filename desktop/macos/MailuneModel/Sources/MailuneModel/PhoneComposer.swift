import Foundation

/// A phone draft. Send stays off until `confirmed` is set.
public struct PhoneDraft: Equatable, Sendable {
    public var recipient: String
    public var subject: String
    public var body: String
    public var confirmed: Bool

    public init(recipient: String, subject: String, body: String, confirmed: Bool) {
        self.recipient = recipient
        self.subject = subject
        self.body = body
        self.confirmed = confirmed
    }
}

public final class FakePhoneDrafts: @unchecked Sendable {
    private let lock = NSLock()
    private var drafts: [String: PhoneDraft] = [:]
    private var sent: Set<String> = []
    private var next = 1

    public init() {}

    public func save(_ draft: PhoneDraft) -> String {
        lock.lock()
        defer { lock.unlock() }
        let id = String(next)
        next += 1
        drafts[id] = draft
        return id
    }

    public func load(_ id: String) -> PhoneDraft? {
        lock.lock()
        defer { lock.unlock() }
        return drafts[id]
    }

    public func confirm(_ id: String) {
        lock.lock()
        drafts[id]?.confirmed = true
        lock.unlock()
    }

    public func send(_ id: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard drafts[id]?.confirmed == true else { return false }
        sent.insert(id)
        return true
    }
}
