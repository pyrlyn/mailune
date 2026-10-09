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
        let list = NSHostingView(rootView: ThreadList(selected: .constant([]), query: "is:unread"))
        list.layoutSubtreeIfNeeded()
        XCTAssertGreaterThan(list.fittingSize.width, 0)
        let ask = NSHostingView(rootView: AskView { _ in })
        ask.layoutSubtreeIfNeeded()
        XCTAssertGreaterThan(ask.fittingSize.width, 0)
    }
}
