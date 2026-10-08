import XCTest
@testable import MailuneModel

final class OnboardingTests: XCTestCase {
    func testAutoconfigAndOAuthCreateAnAccount() {
        let store = FakeAccountStore()
        let account = store.create(email: "ana@example.com")
        XCTAssertEqual(account?.host, "imap.example.com")
        XCTAssertEqual(account?.token, "stub-ana@example.com")
        XCTAssertEqual(store.accounts().map(\.email), ["ana@example.com"])
        XCTAssertNil(store.create(email: "not-an-email"))
    }
}
