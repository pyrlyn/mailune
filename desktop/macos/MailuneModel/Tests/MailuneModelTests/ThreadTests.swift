import XCTest
@testable import MailuneModel

final class ThreadTests: XCTestCase {
    func testPrimaryTabHidesOtherCategoriesAndArchivedRows() {
        let rows = ThreadFixtures.visible(category: "Primary", archived: ["2"])
        XCTAssertEqual(rows.map(\.id), ["1"])
        XCTAssertTrue(rows[0].unread)
    }

    func testUpdatesTab() {
        XCTAssertEqual(ThreadFixtures.visible(category: "Updates", archived: []).map(\.subject), ["Receipt"])
    }
}
