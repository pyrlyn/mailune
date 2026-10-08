import XCTest
@testable import MailuneModel

final class SettingsTests: XCTestCase {
    func testAChangeRoundTripsThroughTheFakeStore() {
        let store = FakePreferencesStore()
        var updated = store.load()
        updated.accountName = "Bea"
        updated.appearance = "Dark"
        updated.notifications = false
        updated.readingPane = false
        updated.composeFontSize = 18
        updated.syncIntervalMinutes = 15
        store.save(updated)
        XCTAssertEqual(store.load(), updated)
    }
}
