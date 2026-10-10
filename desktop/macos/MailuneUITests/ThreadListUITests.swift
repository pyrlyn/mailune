import XCTest

final class ThreadListUITests: XCTestCase {
    @MainActor
    func testThreadListShowsAFixtureSubject() {
        let app = XCUIApplication()
        // Fakes instead of the Dock, Spotlight and saved settings, so a test
        // run leaves nothing behind on the machine.
        app.launchArguments = ["-mailune-ui-test", "-ApplePersistenceIgnoreState", "YES"]
        app.launch()
        app.activate()
        // A launch that macOS keeps in the background opens no window; the
        // File menu still answers, so ask it for one.
        if !app.windows.firstMatch.waitForExistence(timeout: 10) {
            app.menuBarItems["File"].click()
            app.menuItems["New Window"].click()
        }
        XCTAssertTrue(app.windows.firstMatch.waitForExistence(timeout: 20), "the app did not open its window")

        let subject = app.staticTexts["thread-t1"]
        XCTAssertTrue(subject.waitForExistence(timeout: 20))
        XCTAssertTrue([subject.label, subject.value as? String].contains("Quarterly plan"))
    }
}
