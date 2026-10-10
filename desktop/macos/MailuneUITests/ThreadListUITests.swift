import XCTest

final class ThreadListUITests: XCTestCase {
    @MainActor
    func testThreadListShowsAFixtureSubject() {
        let app = XCUIApplication.launchMailune()

        let subject = app.staticTexts["thread-t1"]
        XCTAssertTrue(subject.waitForExistence(timeout: 20))
        XCTAssertTrue([subject.label, subject.value as? String].contains("Quarterly plan"))
    }
}
