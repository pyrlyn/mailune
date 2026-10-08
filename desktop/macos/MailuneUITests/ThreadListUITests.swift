import XCTest

final class ThreadListUITests: XCTestCase {
    func testThreadListShowsAFixtureSubject() {
        let app = XCUIApplication()
        app.launch()
        let subject = app.staticTexts["thread-1"]
        XCTAssertTrue(subject.waitForExistence(timeout: 15))
        XCTAssertEqual(subject.label, "Hello")
    }
}
