import XCTest
@testable import MailuneModel

final class ComposerTests: XCTestCase {
    func testSendStaysOffUntilConfirmAndUndoRestoresTheDraft() {
        let outbox = FakeOutbox()
        let later = Date(timeIntervalSince1970: 1_700_000_000)
        let draft = ComposerDraft(
            recipients: ["ana@example.com"],
            body: "Hi",
            attachmentName: "agenda.txt",
            sendLater: later,
            confirmed: false
        )
        let id = outbox.stage(draft)
        XCTAssertFalse(outbox.send(id))
        outbox.confirm(id)
        XCTAssertTrue(outbox.send(id))
        let restored = outbox.undo(id)
        XCTAssertEqual(restored?.recipients, ["ana@example.com"])
        XCTAssertEqual(restored?.body, "Hi")
        XCTAssertEqual(restored?.attachmentName, "agenda.txt")
        XCTAssertEqual(restored?.sendLater, later)
        XCTAssertFalse(outbox.isSent(id))
    }
}
