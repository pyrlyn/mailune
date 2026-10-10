import SwiftUI
import XCTest
@testable import MailuneUI

final class InboxViewTests: XCTestCase {
    @MainActor
    func testViewBuilds() {
        XCTAssertGreaterThan(Hosting.render(InboxView()).bounds.width, 0)
    }
}
