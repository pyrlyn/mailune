import XCTest
@testable import MailunePlatform

final class IntegrationTests: XCTestCase {
    func testKeychainRoundTrip() {
        let store = FakeKeychain()
        store.set(account: "ana", secret: "token")
        XCTAssertEqual(store.secret(for: "ana"), "token")
        XCTAssertNil(store.secret(for: "missing"))
    }

    func testNotifierRecords() {
        let notifier = FakeNotifier()
        notifier.post(title: "Mail", body: "One new message")
        XCTAssertEqual(notifier.posted().count, 1)
        XCTAssertEqual(notifier.posted()[0].title, "Mail")
    }

    func testNetworkPath() {
        XCTAssertFalse(FakeNetworkPath(isOnline: false).isOnline)
        XCTAssertTrue(FakeNetworkPath(isOnline: true).isOnline)
    }

    func testWebAuthReturnsTheFakeToken() {
        let auth = FakeWebAuth(token: "abc")
        XCTAssertEqual(auth.authenticate(callback: "mailune://done"), "abc")
        XCTAssertNil(FakeWebAuth(token: "abc").authenticate(callback: ""))
    }

    func testOpenURLRecordsTheRequest() throws {
        let opener = FakeURLOpener()
        let url = try XCTUnwrap(URL(string: "mailto:ana@example.com"))
        XCTAssertTrue(opener.open(url))
        XCTAssertEqual(opener.opened(), [url])
    }
}
