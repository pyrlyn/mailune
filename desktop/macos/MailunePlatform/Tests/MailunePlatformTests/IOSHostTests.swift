import XCTest
@testable import MailunePlatform

final class IOSHostTests: XCTestCase {
    func testKeychainRefreshAndNotificationsStayOnFakes() {
        let host = IOSHostServices.fakes()
        host.keychain.set(account: "ana", secret: "token")
        XCTAssertEqual(host.keychain.secret(for: "ana"), "token")

        host.refresh.schedule()
        XCTAssertTrue(host.refresh.isScheduled())

        host.notifier.post(title: "Mail", body: "Hello")
        XCTAssertEqual(host.notifier.posted().count, 1)
    }
}
