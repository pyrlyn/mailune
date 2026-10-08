import XCTest
@testable import MailuneUI

final class CopyTests: XCTestCase {
    func testEnglishString() {
        XCTAssertEqual(Copy.text("inbox.title", language: "en"), "Inbox")
    }

    func testMissingKeyFallsBackToEnglish() {
        XCTAssertEqual(Copy.text("inbox.empty", language: "en-GB"), "No mail yet")
    }
}
