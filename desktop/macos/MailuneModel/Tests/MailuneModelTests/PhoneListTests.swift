import XCTest
@testable import MailuneModel

final class PhoneListTests: XCTestCase {
    func testSwipeArchiveAndRefresh() {
        var state = PhoneListState(selected: "1")
        XCTAssertEqual(state.rows().map(\.subject), ["Hello", "Notes"])
        state.archive("2")
        XCTAssertEqual(state.rows().map(\.id), ["1"])
        state.refresh()
        XCTAssertEqual(state.refreshCount, 1)
        XCTAssertEqual(state.selected, "1")
    }
}
