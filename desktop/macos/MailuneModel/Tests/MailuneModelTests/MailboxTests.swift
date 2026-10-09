import XCTest
@testable import MailuneModel

final class MailboxTests: XCTestCase {
    func testSidebarStartsWithTheInbox() {
        XCTAssertEqual(Mailbox.fixtures.first, .inbox)
        XCTAssertEqual(Set(Mailbox.fixtures.map(\.id)).count, Mailbox.fixtures.count)
    }
}
