import XCTest

/// XCUITest reads the same accessibility tree VoiceOver does, which a hosted
/// view in a unit test never builds.
final class AccessibilityUITests: XCTestCase {
    @MainActor
    func testReaderAndComposerAreLabelled() {
        let app = XCUIApplication.launchMailune()
        openFirstThread(app)

        XCTAssertTrue(app.staticTexts["From Ana Ruiz"].exists, "the sender has no label")
        XCTAssertTrue(app.groups["Summary"].exists, "the assist card is not one named group")

        app.buttons["Reply"].click()
        let sheet = app.sheets.firstMatch
        XCTAssertTrue(sheet.textViews["Message body"].waitForExistence(timeout: 10))
        XCTAssertTrue(sheet.textFields["To"].exists)
        XCTAssertTrue(sheet.descendants(matching: .any)["ana@acme.example"].exists, "the reply has no labelled chip")
        XCTAssertTrue(sheet.buttons["Send"].isEnabled)
    }

    @MainActor
    func testReplyAndSendFromTheKeyboard() throws {
        let app = XCUIApplication.launchMailune()
        openFirstThread(app)
        // Keys go to the frontmost app. When macOS keeps Mailune behind the
        // app that started the run, they would land there instead.
        guard app.wait(for: .runningForeground, timeout: 5) else {
            throw XCTSkip("macOS kept Mailune in the background, so key presses would reach another app")
        }

        app.typeKey("r", modifierFlags: .command)
        let sheet = app.sheets.firstMatch
        XCTAssertTrue(sheet.textViews["Message body"].waitForExistence(timeout: 10), "⌘R did not open the composer")
        app.typeKey(.return, modifierFlags: .command)
        XCTAssertTrue(app.buttons["Send"].firstMatch.waitForExistence(timeout: 10), "⌘↩ did not ask to confirm")
        XCTAssertFalse(app.buttons["Undo"].exists, "nothing is held before the confirmation")
        app.typeKey(.return, modifierFlags: [])
        let undo = app.buttons["Undo"]
        XCTAssertTrue(undo.waitForExistence(timeout: 10), "Return did not confirm")
        undo.click()
    }

    @MainActor
    private func openFirstThread(_ app: XCUIApplication) {
        let row = app.staticTexts["thread-t1"]
        XCTAssertTrue(row.waitForExistence(timeout: 20))
        row.click()
        XCTAssertTrue(app.buttons["Reply"].waitForExistence(timeout: 10))
    }
}
