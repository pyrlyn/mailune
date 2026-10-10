import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

@MainActor
final class SettingsViewTests: XCTestCase {
    func testSettingsBuildFromTheStoreWithoutWritingIt() {
        let store = FakePreferencesStore()
        XCTAssertGreaterThan(Hosting.render(SettingsView(store: store)).bounds.height, 0)
        XCTAssertNil(store.data, "opening settings does not write")
    }

    func testEveryOptionHasACatalogEntry() {
        let keys = Theme.allCases.map(\.titleKey) + Density.allCases.map(\.titleKey)
            + ["settings.theme", "settings.display_name", "settings.seconds"]
        for key in keys {
            XCTAssertNotEqual(Copy.text(key, language: "en"), key)
        }
    }
}
