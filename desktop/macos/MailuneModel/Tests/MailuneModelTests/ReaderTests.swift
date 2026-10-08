import XCTest
@testable import MailuneModel

final class ReaderTests: XCTestCase {
    func testQuotesStartCollapsedAndSecurityStaysOff() throws {
        let message = try XCTUnwrap(MessageFixtures.message(forThread: "1"))
        let collapsed = ReaderPresentation(message: message, quotesCollapsed: true)
        XCTAssertFalse(collapsed.text.contains("earlier note"))
        XCTAssertEqual(collapsed.attachmentName, "notes.txt")
        XCTAssertEqual(collapsed.badge, "Authenticated")
        XCTAssertFalse(collapsed.remoteContentEnabled)
        XCTAssertFalse(collapsed.javaScriptEnabled)

        let expanded = ReaderPresentation(message: message, quotesCollapsed: false)
        XCTAssertTrue(expanded.text.contains("earlier note"))
        XCTAssertFalse(expanded.remoteContentEnabled)
        XCTAssertFalse(expanded.javaScriptEnabled)
    }
}
