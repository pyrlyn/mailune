import XCTest
@testable import MailuneModel

@MainActor
final class ComposerTests: XCTestCase {
    private let start = Date(timeIntervalSinceReferenceDate: 800_000_000)
    private let attachment = Attachment(name: "agenda.txt", size: 512)

    private func filled(_ outbox: FakeOutbox) -> Composer {
        let composer = Composer(outbox: outbox, undoWindow: 10)
        composer.addRecipients("ana@acme.example")
        composer.draft.subject = "Agenda"
        composer.draft.body = "See the attached agenda."
        composer.draft.attachment = attachment
        return composer
    }

    func testChipsSplitTypedTextAndRejectNonAddresses() {
        let composer = Composer(outbox: FakeOutbox())
        let rejected = composer.addRecipients("ana@acme.example, ben@ito.example; nope  ANA@acme.example a@b")
        XCTAssertEqual(composer.draft.to, ["ana@acme.example", "ben@ito.example"])
        XCTAssertEqual(rejected, ["nope", "a@b"])
        composer.removeRecipient("ana@acme.example")
        XCTAssertEqual(composer.draft.to, ["ben@ito.example"])
    }

    func testSendStaysOffUntilConfirm() {
        let outbox = FakeOutbox()
        let composer = Composer(outbox: outbox)
        XCTAssertFalse(composer.canSend)
        composer.requestSend()
        XCTAssertEqual(composer.stage, .editing)

        composer.addRecipients("ana@acme.example")
        composer.confirmSend(now: start)
        XCTAssertTrue(outbox.held.isEmpty, "confirm without a request does nothing")

        composer.requestSend()
        XCTAssertEqual(composer.stage, .confirming)
        XCTAssertFalse(composer.canSend)
        XCTAssertTrue(outbox.held.isEmpty)

        composer.cancelSend()
        XCTAssertEqual(composer.stage, .editing)
        outbox.release(now: start.addingTimeInterval(3600))
        XCTAssertTrue(outbox.held.isEmpty)
        XCTAssertTrue(outbox.sent.isEmpty)
    }

    func testUndoInsideTheWindowRestoresTheWholeDraft() {
        let outbox = FakeOutbox()
        let composer = filled(outbox)
        let before = composer.draft
        composer.requestSend()
        composer.confirmSend(now: start)
        XCTAssertEqual(outbox.held.values.first?.release, start.addingTimeInterval(10))

        XCTAssertTrue(composer.undo(now: start.addingTimeInterval(9)))
        XCTAssertEqual(composer.draft, before)
        XCTAssertEqual(composer.stage, .editing)
        outbox.release(now: start.addingTimeInterval(60))
        XCTAssertTrue(outbox.sent.isEmpty)
    }

    func testAfterTheWindowTheDraftGoesOutAndUndoFails() {
        let outbox = FakeOutbox()
        let composer = filled(outbox)
        composer.requestSend()
        composer.confirmSend(now: start)
        let due = start.addingTimeInterval(10)
        outbox.release(now: due)
        XCTAssertFalse(composer.undo(now: due))

        XCTAssertEqual(outbox.sent.count, 1)
        XCTAssertEqual(outbox.sent.first?.to, ["ana@acme.example"])
        XCTAssertEqual(outbox.sent.first?.body, "See the attached agenda.")
        XCTAssertEqual(outbox.sent.first?.attachment, attachment)

        composer.settle(now: due)
        XCTAssertEqual(composer.draft, Draft())
        XCTAssertEqual(composer.stage, .editing)
    }

    func testSendLaterHoldsUntilTheChosenTimeAndCanBeUndone() {
        let outbox = FakeOutbox()
        let composer = filled(outbox)
        let later = start.addingTimeInterval(3600)
        composer.draft.sendAt = later
        composer.requestSend()
        composer.confirmSend(now: start)

        outbox.release(now: start.addingTimeInterval(11))
        XCTAssertTrue(outbox.sent.isEmpty)
        XCTAssertTrue(composer.undo(now: start.addingTimeInterval(600)))
        XCTAssertEqual(composer.draft.sendAt, later)

        composer.requestSend()
        composer.confirmSend(now: start.addingTimeInterval(601))
        outbox.release(now: later)
        XCTAssertEqual(outbox.sent.map(\.sendAt), [later])
    }

    func testPastSendLaterStillGetsTheUndoWindow() {
        let outbox = FakeOutbox()
        let composer = filled(outbox)
        composer.draft.sendAt = start.addingTimeInterval(-60)
        composer.requestSend()
        composer.confirmSend(now: start)
        XCTAssertEqual(outbox.held.values.first?.release, start.addingTimeInterval(10))
    }
}
