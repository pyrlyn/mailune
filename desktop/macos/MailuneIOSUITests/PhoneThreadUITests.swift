import XCTest

/// The phone layout on an iPhone simulator: the list pushes the reader.
final class PhoneThreadUITests: XCTestCase {
    @MainActor
    func testTappingAFixtureThreadOpensItsReader() {
        let app = XCUIApplication.launchPhone()
        let row = app.staticTexts["thread-t1"]
        XCTAssertTrue(row.waitForExistence(timeout: 30), "the phone list shows no fixture row")
        XCTAssertEqual(row.label, "Quarterly plan")
        row.tap()

        XCTAssertTrue(app.staticTexts["From Ana Ruiz"].waitForExistence(timeout: 10), "the reader did not open")
        XCTAssertTrue(app.navigationBars.buttons["Inbox"].exists, "the reader was not pushed onto the list")
    }

    @MainActor
    func testPhoneReaderShowsTheSummaryAndReplyChips() {
        let app = XCUIApplication.launchPhone()
        let row = app.staticTexts["thread-t1"]
        XCTAssertTrue(row.waitForExistence(timeout: 30))
        row.tap()

        let summary = app.staticTexts["assist-summary"]
        XCTAssertTrue(summary.waitForExistence(timeout: 10), "the phone reader shows no summary")
        XCTAssertEqual(summary.label, "Ana shared the quarterly plan draft and wants comments by Friday.")
        XCTAssertTrue(app.staticTexts["Made on this device"].exists)

        let chips = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "Reply with")).allElementsBoundByIndex
        XCTAssertEqual(chips.count, 3)
        let width = app.windows.firstMatch.frame.width
        XCTAssertTrue(chips.allSatisfy { $0.frame.maxX <= width }, "a chip runs off the screen")
        XCTAssertEqual(Set(chips.map(\.frame.minY)).count, 3, "the chips are squeezed into one row")

        chips[0].tap()
        let body = app.textViews.firstMatch
        XCTAssertTrue(body.waitForExistence(timeout: 10), "a chip did not open the composer")
        XCTAssertEqual(body.value as? String, "I'll send comments by Friday.")
        app.buttons["Discard"].tap()
    }
}

extension XCUIApplication {
    @MainActor
    static func launchPhone() -> XCUIApplication {
        let app = XCUIApplication()
        // Fake saved settings, in English because the tests read labels.
        app.launchArguments = ["-mailune-ui-test", "-AppleLanguages", "(en)"]
        app.launch()
        return app
    }
}
