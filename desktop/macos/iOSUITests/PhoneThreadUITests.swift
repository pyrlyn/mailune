import XCTest

final class PhoneThreadUITests: XCTestCase {
    func testSelectsAFixtureThread() {
        let app = XCUIApplication()
        app.launch()
        let row = app.staticTexts["Hello"]
        XCTAssertTrue(row.waitForExistence(timeout: 20))
        row.tap()
        let body = app.staticTexts["See you tomorrow."]
        XCTAssertTrue(body.waitForExistence(timeout: 10))
    }
}
