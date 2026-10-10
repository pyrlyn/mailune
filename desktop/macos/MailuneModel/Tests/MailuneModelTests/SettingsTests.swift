import XCTest
@testable import MailuneModel

@MainActor
final class SettingsTests: XCTestCase {
    private func everySectionChanged() -> Preferences {
        var changed = Preferences.standard
        changed.accounts[0].displayName = "Ana R."
        changed.theme = .dark
        changed.density = .compact
        changed.notifications = false
        changed.onlyVIPs = true
        changed.conversationView = false
        changed.autoAdvance = true
        changed.undoSendSeconds = 30
        changed.signature = "— Ana"
        changed.syncDays = 365
        changed.wifiOnly = true
        return changed
    }

    func testAChangeInEverySectionRoundTripsThroughTheFakeStore() throws {
        let store = FakePreferencesStore()
        XCTAssertEqual(store.current(), .standard)
        let changed = everySectionChanged()
        XCTAssertNotEqual(changed, .standard)
        try store.save(changed)

        let reopened = FakePreferencesStore(data: store.data)
        XCTAssertEqual(try reopened.load(), changed)
    }

    func testUnreadableDataFallsBackToDefaults() {
        let store = FakePreferencesStore(data: Data("{".utf8))
        XCTAssertThrowsError(try store.load())
        XCTAssertEqual(store.current(), .standard)
    }

    func testDefaultsStoreWritesTheSameJSON() throws {
        let suite = "app.mailune.tests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        addTeardownBlock { defaults.removePersistentDomain(forName: suite) }
        let store = DefaultsPreferencesStore(defaults: defaults)
        try store.save(everySectionChanged())
        XCTAssertEqual(try FakePreferencesStore(data: defaults.data(forKey: "preferences")).load(), everySectionChanged())
    }

    func testTheUndoWindowFeedsTheComposer() {
        var preferences = Preferences.standard
        preferences.undoSendSeconds = 20
        let composer = Composer(outbox: FakeOutbox(), undoWindow: TimeInterval(preferences.undoSendSeconds))
        XCTAssertEqual(composer.undoWindow, 20)
    }
}
