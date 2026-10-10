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
