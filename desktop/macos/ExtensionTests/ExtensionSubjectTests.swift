//! Reads one fixture subject from the share, notification, and widget sources.

import XCTest

final class ExtensionSubjectTests: XCTestCase {
    func testReadsOneFixtureSubjectFromEach() {
        XCTAssertEqual(ShareMail.subject, "Hello")
        XCTAssertEqual(NotificationPreview.subject(), "Hello")
        XCTAssertEqual(WidgetMail.subject, "Hello")
    }
}
