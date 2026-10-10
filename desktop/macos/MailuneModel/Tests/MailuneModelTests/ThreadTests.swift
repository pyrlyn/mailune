import XCTest
@testable import MailuneModel

final class ThreadTests: XCTestCase {
    func testFixtureDecodesFromTheContractSnapshot() {
        XCTAssertEqual(ThreadFixtures.all.map(\.id), ["t1", "t2", "t3", "t4"])
        XCTAssertEqual(ThreadFixtures.all[0].from.display, "Ana Ruiz")
        XCTAssertTrue(ThreadFixtures.all[0].hasAttachment)
        XCTAssertEqual(ThreadFixtures.all[2].from.display, "receipts@shop.example")
    }


    func testUnknownCategoryIsRejected() {
        let json = #"{"snapshot":{"threads":[{"id":"x","from":{"email":"a@b"},"subject":"","snippet":"","stamp":"","unread":false,"has_attachment":false,"category":"spam","labels":[]}]}}"#
        XCTAssertThrowsError(try ThreadFixtures.decode(Data(json.utf8)))
    }
}
