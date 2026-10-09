import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

final class AssistViewTests: XCTestCase {
    @MainActor
    func testReaderWithAssistBuildsAndStillHasNoWebView() throws {
        let message = try XCTUnwrap(MessageFixtures.message(forThread: "t1"))
        let assist = AssistPolicy.visible(AssistFixtures.assist(forThread: "t1"), for: message)
        XCTAssertNotNil(assist)
        let host = NSHostingView(rootView: ReaderView(message: message, assist: assist).frame(width: 640, height: 600))
        host.layoutSubtreeIfNeeded()
        XCTAssertGreaterThan(host.fittingSize.height, 0)
        XCTAssertFalse(ReaderViewTests.classNames(in: host).contains { $0.contains("WebView") })
    }

    func testAIStringsAreInTheCatalogs() {
        let keys = ["ai.summary", "ai.action_items", "ai.made_on_device", "ai.made_in_cloud", "ai.reply_with", "settings.ai_privacy"]
        for key in keys {
            XCTAssertNotEqual(Copy.text(key, language: "en"), key)
            XCTAssertNotEqual(Copy.text(key, language: "fr"), Copy.text(key, language: "en"))
        }
        XCTAssertTrue(Copy.text("settings.ai_privacy", language: "en").contains("never sent to a cloud model"))
    }
}
