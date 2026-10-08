import Foundation

/// A message the composer is holding. Send stays off until `confirmed` is set.
public struct ComposerDraft: Equatable, Sendable {
    public var recipients: [String]
    public var body: String
    public var attachmentName: String?
    public var sendLater: Date?
    public var confirmed: Bool

    public init(
        recipients: [String],
        body: String,
        attachmentName: String?,
        sendLater: Date?,
        confirmed: Bool
    ) {
        self.recipients = recipients
        self.body = body
        self.attachmentName = attachmentName
        self.sendLater = sendLater
        self.confirmed = confirmed
    }
}

/// Queues a draft, sends it only after confirm, and can hand it back on undo.
public final class FakeOutbox: @unchecked Sendable {
    private let lock = NSLock()
    private var drafts: [String: ComposerDraft] = [:]
    private var sent: Set<String> = []
    private var next = 1

    public init() {}

    public func stage(_ draft: ComposerDraft) -> String {
        lock.lock()
        defer { lock.unlock() }
        let id = String(next)
        next += 1
        drafts[id] = draft
        return id
    }

    public func confirm(_ id: String) {
        lock.lock()
        drafts[id]?.confirmed = true
        lock.unlock()
    }

    /// Returns false while the draft is unconfirmed, so send cannot run early.
    public func send(_ id: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard drafts[id]?.confirmed == true else { return false }
        sent.insert(id)
        return true
    }

    public func isSent(_ id: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return sent.contains(id)
    }

    /// Pulls a sent draft back out of the outbox.
    public func undo(_ id: String) -> ComposerDraft? {
        lock.lock()
        defer { lock.unlock() }
        guard sent.contains(id) else { return nil }
        sent.remove(id)
        return drafts.removeValue(forKey: id)
    }
}
