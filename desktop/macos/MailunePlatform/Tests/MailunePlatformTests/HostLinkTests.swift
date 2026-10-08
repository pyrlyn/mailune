import XCTest
@testable import MailunePlatform

final class HostLinkTests: XCTestCase {
    func testMailtoDockShareAndSpotlightUseFakes() throws {
        let mailto = FakeMailto()
        let url = try XCTUnwrap(mailto.compose(address: "ana@example.com", subject: "Hello"))
        XCTAssertEqual(url.scheme, "mailto")
        XCTAssertEqual(mailto.lastAddress(), "ana@example.com")

        let badge = FakeDockBadge()
        badge.setCount(3)
        XCTAssertEqual(badge.count(), 3)

        let share = FakeShareSheet()
        share.share(text: "See you tomorrow.")
        XCTAssertEqual(share.shared(), ["See you tomorrow."])

        let spotlight = FakeSpotlight()
        spotlight.index(id: "1", title: "Hello")
        XCTAssertEqual(spotlight.title(for: "1"), "Hello")
    }
}
