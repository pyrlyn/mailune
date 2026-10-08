import XCTest
@testable import MailuneUI

final class IntentTests: XCTestCase {
    func testSummariseSearchAndCompose() async throws {
        _ = try await SummariseIntent(threadID: "1").perform()
        XCTAssertEqual(MailIntents.summarise(threadID: "1"), "See you tomorrow.")

        _ = try await SearchIntent(query: "Receipt").perform()
        XCTAssertEqual(MailIntents.search(query: "Receipt"), ["3"])

        _ = try await ComposeIntent(recipient: "ana@example.com", body: "Hi").perform()
        let draft = MailIntents.compose(recipient: "ana@example.com", body: "Hi")
        XCTAssertEqual(draft.body, "Hi")
        XCTAssertFalse(draft.confirmed)
    }
}
