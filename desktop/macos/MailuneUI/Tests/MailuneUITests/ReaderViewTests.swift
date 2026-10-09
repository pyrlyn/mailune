import AppKit
import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

final class ReaderViewTests: XCTestCase {
    @MainActor
    func testReaderBuildsNoWebView() throws {
        let message = try XCTUnwrap(MessageFixtures.message(forThread: "t1"))
        let host = NSHostingView(rootView: ReaderView(message: message).frame(width: 600, height: 500))
        let window = NSWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()

        let classes = Self.classNames(in: host)
        XCTAssertFalse(classes.isEmpty)
        XCTAssertFalse(classes.contains { $0.contains("WebView") }, "a web view can run scripts and load remote content")
    }

    func testEveryBadgeHasACatalogEntry() {
        for badge in [SecurityBadge.failed, .encrypted, .signed, .authenticated, .unverified] {
            XCTAssertNotEqual(Copy.text(badge.titleKey, language: "en"), badge.titleKey)
            XCTAssertNotEqual(Copy.text(badge.titleKey, language: "de"), Copy.text(badge.titleKey, language: "en"))
        }
    }

    @MainActor
    private static func classNames(in view: NSView) -> [String] {
        [String(describing: type(of: view))] + view.subviews.flatMap { classNames(in: $0) }
    }
}
