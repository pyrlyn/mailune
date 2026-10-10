import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

final class ReaderViewTests: XCTestCase {
    @MainActor
    func testReaderBuildsNoWebView() throws {
        let message = try XCTUnwrap(MessageFixtures.message(forThread: "t1"))
        let classes = Hosting.classNames(in: Hosting.render(ReaderView(message: message), width: 600, height: 500))
        XCTAssertFalse(classes.isEmpty)
        XCTAssertFalse(classes.contains { $0.contains("WebView") }, "a web view can run scripts and load remote content")
    }

    func testEveryBadgeHasACatalogEntry() {
        for badge in [SecurityBadge.failed, .encrypted, .signed, .authenticated, .unverified] {
            XCTAssertNotEqual(Copy.text(badge.titleKey, language: "en"), badge.titleKey)
            XCTAssertNotEqual(Copy.text(badge.titleKey, language: "de"), Copy.text(badge.titleKey, language: "en"))
        }
    }
}
