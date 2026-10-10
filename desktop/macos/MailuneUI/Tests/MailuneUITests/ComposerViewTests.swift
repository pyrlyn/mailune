import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

final class ComposerViewTests: XCTestCase {
    @MainActor
    func testComposerBuildsWithoutSendingAnything() {
        let outbox = FakeOutbox()
        XCTAssertGreaterThan(Hosting.render(ComposerView(outbox: outbox)).bounds.height, 0)
        XCTAssertTrue(outbox.held.isEmpty)
        XCTAssertTrue(outbox.sent.isEmpty)
    }

    func testComposerStringsAreInTheCatalogs() {
        for key in ["compose.attach", "compose.confirm_send", "compose.remove", "compose.invalid_recipient"] {
            XCTAssertNotEqual(Copy.text(key, language: "en"), key)
            XCTAssertNotEqual(Copy.text(key, language: "ja"), Copy.text(key, language: "en"))
        }
    }
}
