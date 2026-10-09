import Foundation
import Observation

/// What the writer is putting together.
public struct Draft: Equatable, Sendable {
    public var to: [String] = []
    public var subject = ""
    public var body = ""
    public var attachment: Attachment?
    /// Nil sends as soon as the undo window closes.
    public var sendAt: Date?

    public init() {}
}

/// Where a confirmed draft waits until it goes out. `FakeOutbox` stands in
/// until MailuneCore (B3) exposes the core's queue.
@MainActor
public protocol Outbox: AnyObject {
    func hold(_ draft: Draft, until release: Date) -> UUID
    /// The held draft, or nil once it has gone out.
    func cancel(_ id: UUID) -> Draft?
}

@MainActor
public final class FakeOutbox: Outbox {
    public private(set) var held: [UUID: (draft: Draft, release: Date)] = [:]
    public private(set) var sent: [Draft] = []

    public init() {}

    public func hold(_ draft: Draft, until release: Date) -> UUID {
        let id = UUID()
        held[id] = (draft, release)
        return id
    }

    public func cancel(_ id: UUID) -> Draft? {
        held.removeValue(forKey: id)?.draft
    }

    /// Sends what is due. The core's queue does this on its own clock.
    public func release(now: Date) {
        for (id, item) in held where item.release <= now {
            held[id] = nil
            sent.append(item.draft)
        }
    }
}

/// Chips, confirm, send later and undo send. Nothing reaches the outbox
/// except through `confirmSend`, and that only after `requestSend`.
@MainActor
@Observable
public final class Composer {
    public enum Stage: Equatable {
        case editing
        case confirming
        case held(UUID, until: Date)
    }

    public var draft = Draft()
    public private(set) var stage = Stage.editing
    public let undoWindow: TimeInterval
    private let outbox: Outbox

    public init(outbox: Outbox, undoWindow: TimeInterval = 10) {
        self.outbox = outbox
        self.undoWindow = undoWindow
    }

    /// Splits typed text into chips and returns the parts that are not addresses.
    @discardableResult
    public func addRecipients(_ text: String) -> [String] {
        var rejected: [String] = []
        for part in text.split(whereSeparator: { $0 == "," || $0 == ";" || $0.isWhitespace }) {
            let address = String(part)
            guard Self.looksLikeAddress(address) else {
                rejected.append(address)
                continue
            }
            if !draft.to.contains(where: { $0.caseInsensitiveCompare(address) == .orderedSame }) {
                draft.to.append(address)
            }
        }
        return rejected
    }

    public func removeRecipient(_ address: String) {
        draft.to.removeAll { $0 == address }
    }

    public var canSend: Bool {
        stage == .editing && !draft.to.isEmpty
    }

    public func requestSend() {
        guard canSend else { return }
        stage = .confirming
    }

    public func cancelSend() {
        guard stage == .confirming else { return }
        stage = .editing
    }

    /// The draft is held until the later of the undo window and `sendAt`, so
    /// a scheduled send can be taken back until it goes out.
    public func confirmSend(now: Date) {
        guard stage == .confirming else { return }
        let release = max(draft.sendAt ?? now, now.addingTimeInterval(undoWindow))
        stage = .held(outbox.hold(draft, until: release), until: release)
    }

    public func canUndo(now: Date) -> Bool {
        guard case let .held(_, until) = stage else { return false }
        return now < until
    }

    /// Takes the draft back for editing. False once it has gone out.
    @discardableResult
    public func undo(now: Date) -> Bool {
        guard canUndo(now: now), case let .held(id, _) = stage, let restored = outbox.cancel(id) else {
            return false
        }
        draft = restored
        stage = .editing
        return true
    }

    /// After the window, the composer starts over with an empty draft. True
    /// when that happened, so the view can close.
    @discardableResult
    public func settle(now: Date) -> Bool {
        guard case let .held(_, until) = stage, now >= until else { return false }
        draft = Draft()
        stage = .editing
        return true
    }

    static func looksLikeAddress(_ text: String) -> Bool {
        let parts = text.split(separator: "@", omittingEmptySubsequences: false)
        return parts.count == 2 && !parts[0].isEmpty && parts[1].contains(".")
            && !parts[1].hasPrefix(".") && !parts[1].hasSuffix(".")
    }
}
