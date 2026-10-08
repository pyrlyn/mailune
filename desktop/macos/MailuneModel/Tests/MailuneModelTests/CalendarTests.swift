import XCTest
@testable import MailuneModel

final class CalendarTests: XCTestCase {
    func testFixtureBecomesTwoSuggestions() {
        let suggestions = CalendarSuggestions.load()
        XCTAssertEqual(suggestions.map(\.id), ["draft-review", "call-ana"])
        XCTAssertEqual(suggestions.map(\.summary), ["Review the draft", "Call Ana"])
        XCTAssertEqual(suggestions.map(\.start), ["2026-10-09 15:00 UTC", "2026-10-10 18:00 UTC"])
        XCTAssertEqual(suggestions.first?.location, "Local")
        XCTAssertEqual(suggestions.last?.location, "")
    }

    func testFoldedSummaryAndADatelessEventAreHandled() {
        // The fold marker (newline plus one space) is removed, so the word
        // space has to sit on the first line or the summary joins without it.
        let text = [
            "BEGIN:VEVENT",
            "UID:folded",
            "DTSTART:20261011T090000Z",
            "SUMMARY:Review ",
            " the notes",
            "END:VEVENT",
            "BEGIN:VEVENT",
            "UID:skipped",
            "SUMMARY:No time",
            "END:VEVENT",
        ].joined(separator: "\n")
        let suggestions = CalendarSuggestions.parse(text)
        XCTAssertEqual(suggestions.map(\.summary), ["Review the notes"])
        XCTAssertEqual(suggestions.map(\.start), ["2026-10-11 09:00 UTC"])
    }

    func testEmptyTextYieldsNothing() {
        XCTAssertTrue(CalendarSuggestions.parse("not a calendar").isEmpty)
    }
}
