import XCTest
@testable import MailuneUI

final class CommandTests: XCTestCase {
    func testEmptyQueryShowsEveryCommand() {
        XCTAssertEqual(CommandList.filter("").map(\.id), ["inbox", "compose"])
    }

    func testQueryKeepsTheMatchingCommand() {
        XCTAssertEqual(CommandList.filter("comp").map(\.id), ["compose"])
    }
}
