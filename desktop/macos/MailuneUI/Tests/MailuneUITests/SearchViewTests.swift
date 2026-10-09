import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

final class SearchViewTests: XCTestCase {
    func testSearchErrorsNameTheToken() {
        XCTAssertTrue(ThreadList.message(for: .bad("colour:red")).contains("colour:red"))
        XCTAssertTrue(ThreadList.message(for: .unsupported("to:ben")).contains("to:ben"))
        XCTAssertNotEqual(ThreadList.message(for: .bad("x")), "search.bad_query")
    }

    @MainActor
    func testSearchingListAndAskBuild() {
        XCTAssertGreaterThan(Hosting.render(ThreadList(selected: .constant([]), query: "is:unread")).bounds.width, 0)
        XCTAssertGreaterThan(Hosting.render(AskView { _ in }).bounds.width, 0)
    }
}
