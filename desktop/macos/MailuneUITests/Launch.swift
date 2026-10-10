import XCTest

extension XCUIApplication {
    /// The app on fixtures, in English, with a window open.
    @MainActor
    static func launchMailune() -> XCUIApplication {
        let app = XCUIApplication()
        // Fakes instead of the Dock, Spotlight and saved settings, so a test
        // run leaves nothing behind on the machine. English, because the
        // tests read labels.
        app.launchArguments = [
            "-mailune-ui-test", "-ApplePersistenceIgnoreState", "YES", "-AppleLanguages", "(en)",
        ]
        app.launch()
        app.activate()
        // A launch that macOS keeps in the background opens no window; the
        // File menu still answers, so ask it for one.
        if !app.windows.firstMatch.waitForExistence(timeout: 10) {
            app.menuBarItems["File"].click()
            app.menuItems["New Window"].click()
        }
        XCTAssertTrue(app.windows.firstMatch.waitForExistence(timeout: 20), "the app did not open its window")
        return app
    }
}
