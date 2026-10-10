import MailunePlatform
import XCTest

final class ArchitectureTests: XCTestCase {
    func testAppTestBundleRunsOnAppleSilicon() {
        XCTAssertEqual(Host.architecture, "arm64")
    }
}
