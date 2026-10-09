#if os(iOS)
    import XCTest
    @testable import MailuneUI

    final class AdaptiveMailboxTests: XCTestCase {
        func testCompactWidthStacksAndEveryOtherWidthSplits() {
            XCTAssertEqual(AdaptiveMailbox.layout(for: .compact), .stack)
            XCTAssertEqual(AdaptiveMailbox.layout(for: .regular), .split)
            XCTAssertEqual(AdaptiveMailbox.layout(for: nil), .split)
        }
    }
#endif
