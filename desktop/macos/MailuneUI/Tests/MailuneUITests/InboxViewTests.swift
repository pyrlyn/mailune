import SwiftUI
import XCTest
@testable import MailuneUI

final class InboxViewTests: XCTestCase {
    @MainActor
    func testViewBuilds() {
        XCTAssertNotNil(NSHostingView(rootView: InboxView()).fittingSize)
    }
}
