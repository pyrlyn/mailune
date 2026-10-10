import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

/// The phone pushes the same `ReaderView` the Mac shows, so these run on the
/// iOS simulator in scripts/test.sh as well as on macOS.
final class PhoneReaderTests: XCTestCase {
    @MainActor
    func testEveryFixtureThreadOpensWithoutAWebView() throws {
        for row in ThreadFixtures.all {
            guard let message = MessageFixtures.message(forThread: row.id) else { continue }
            let view = ReaderView(message: message, assist: AssistPolicy.fixture(for: message))
            let classes = Hosting.classNames(in: Hosting.render(view, width: 390, height: 844))
            XCTAssertFalse(classes.contains { $0.contains("WebView") }, "thread \(row.id)")
        }
    }

    func testOnDeviceLineNamesTheMacOnlyOnTheMac() {
        #if os(macOS)
            XCTAssertEqual(AssistCard.deviceKey, "ai.made_on_device")
        #else
            XCTAssertEqual(AssistCard.deviceKey, "ai.made_on_this_device")
        #endif
        let line = Copy.text("ai.made_on_this_device", language: "en")
        XCTAssertNotEqual(line, "ai.made_on_this_device")
        XCTAssertFalse(line.contains("Mac"))
        XCTAssertNotEqual(Copy.text("ai.made_on_this_device", language: "ja"), line)
    }

    func testRemoteContentStaysBlockedAtPhoneWidth() throws {
        let receipt = try XCTUnwrap(MessageFixtures.message(forThread: "t3"))
        let presentation = ReaderPresentation(message: receipt, quotesCollapsed: true)
        XCTAssertEqual(presentation.blockedRemote, RemoteSummary(blocked: 5, trackerPixels: 2))
    }
}
