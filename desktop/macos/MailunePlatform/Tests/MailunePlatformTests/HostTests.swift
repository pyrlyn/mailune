import XCTest
@testable import MailunePlatform

final class HostTests: XCTestCase {
    func testPackageBuildsForAppleSilicon() {
        XCTAssertEqual(Host.architecture, "arm64")
    }
}
