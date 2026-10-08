import XCTest
@testable import MailuneModel

final class AssistTests: XCTestCase {
    func testSummaryAndRepliesComeFromFixtures() throws {
        let assist = try XCTUnwrap(AssistFixtures.assist(forThread: "1"))
        XCTAssertEqual(assist.summary, "Ana suggests meeting tomorrow.")
        XCTAssertEqual(assist.replies, ["Sounds good", "Not then"])
        XCTAssertEqual(PrivacyCopy.summariesStayLocal, "Summaries stay on this Mac.")
    }
}
