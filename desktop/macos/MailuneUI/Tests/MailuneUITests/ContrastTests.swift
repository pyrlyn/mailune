import XCTest
@testable import MailuneUI

final class ContrastTests: XCTestCase {
    func testReaderPairMeetsTheBodyTextRatio() {
        XCTAssertGreaterThanOrEqual(MailuneContrast.readerPair, 4.5)
    }
}
