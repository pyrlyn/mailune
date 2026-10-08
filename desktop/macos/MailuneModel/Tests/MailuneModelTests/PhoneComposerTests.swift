import XCTest
@testable import MailuneModel

final class PhoneComposerTests: XCTestCase {
    func testRecipientSubjectAndBodyRoundTripAndSendWaitsForConfirm() {
        let store = FakePhoneDrafts()
        let draft = PhoneDraft(recipient: "ana@example.com", subject: "Hello", body: "See you", confirmed: false)
        let id = store.save(draft)
        XCTAssertEqual(store.load(id), draft)
        XCTAssertFalse(store.send(id))
        store.confirm(id)
        XCTAssertTrue(store.send(id))
        XCTAssertEqual(store.load(id)?.subject, "Hello")
    }
}
