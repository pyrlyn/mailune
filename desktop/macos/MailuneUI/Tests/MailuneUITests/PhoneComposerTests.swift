import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

/// The phone opens the shared `ComposerView`; these run on the iOS simulator in
/// scripts/test.sh as well as on macOS.
final class PhoneComposerTests: XCTestCase {
    @MainActor
    func testAReplyDraftOpensAtPhoneWidthWithoutSending() throws {
        let message = try XCTUnwrap(MessageFixtures.message(forThread: "t1"))
        let outbox = FakeOutbox()
        let draft = Draft.reply(to: message, body: "Thanks, reading it now.")
        // Without the sheet's NavigationStack: hosting one in a window that has
        // no scene crashes the iOS test runner.
        let view = ComposerView(outbox: outbox, draft: draft)
        XCTAssertGreaterThan(Hosting.render(view, width: 390, height: 844).bounds.width, 0)
        XCTAssertTrue(outbox.held.isEmpty, "opening the composer sends nothing")
        XCTAssertTrue(outbox.sent.isEmpty)
    }

    @MainActor
    func testRecipientSubjectAndBodyRoundTripAndSendWaitsForConfirm() {
        let outbox = FakeOutbox()
        var draft = Draft()
        draft.subject = "Lunch"
        draft.body = "Thursday at noon?"
        let composer = Composer(outbox: outbox, undoWindow: 10, draft: draft)
        composer.addRecipients("ben@ito.example")
        let start = Date(timeIntervalSinceReferenceDate: 800_000_000)

        composer.confirmSend(now: start)
        XCTAssertTrue(outbox.held.isEmpty, "no confirm, no send")
        composer.requestSend()
        composer.confirmSend(now: start)
        outbox.release(now: start.addingTimeInterval(10))

        XCTAssertEqual(outbox.sent.first?.to, ["ben@ito.example"])
        XCTAssertEqual(outbox.sent.first?.subject, "Lunch")
        XCTAssertEqual(outbox.sent.first?.body, "Thursday at noon?")
    }
}
