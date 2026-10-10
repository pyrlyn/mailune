import XCTest
@testable import MailuneModel

final class ThreadTests: XCTestCase {
    func testFixtureDecodesFromTheContractSnapshot() {
        XCTAssertEqual(ThreadFixtures.all.map(\.id), ["t1", "t2", "t3", "t4"])
        XCTAssertEqual(ThreadFixtures.all[0].from.display, "Ana Ruiz")
        XCTAssertTrue(ThreadFixtures.all[0].hasAttachment)
        XCTAssertEqual(ThreadFixtures.all[2].from.display, "receipts@shop.example")
    }

    func testPrimaryTabHidesOtherCategoriesAndArchivedRows() {
        let rows = ThreadFixtures.visible(category: .primary, archived: ["t2"])
        XCTAssertEqual(rows.map(\.id), ["t1"])
        XCTAssertTrue(rows[0].unread)
    }

    func testUpdatesTab() {
        XCTAssertEqual(ThreadFixtures.visible(category: .updates, archived: []).map(\.subject), ["Your receipt"])
    }

    func testUnknownCategoryIsRejected() {
        let json = #"{"snapshot":{"threads":[{"id":"x","from":{"email":"a@b"},"subject":"","snippet":"","stamp":"","unread":false,"has_attachment":false,"category":"spam"}]}}"#
        XCTAssertThrowsError(try ThreadFixtures.decode(Data(json.utf8)))
    }
}
