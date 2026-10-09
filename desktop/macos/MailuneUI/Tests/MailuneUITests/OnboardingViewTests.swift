import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

final class OnboardingViewTests: XCTestCase {
    func testEveryErrorHasItsOwnSentence() {
        let errors: [OnboardingError] = [.notAnAddress, .alreadyAdded, .noConfiguration, .insecureOnly, .stateMismatch]
        let sentences = errors.map { OnboardingView.message(for: $0, address: "bea@acme.example") }
        XCTAssertEqual(Set(sentences).count, sentences.count)
        XCTAssertTrue(sentences[2].contains("bea@acme.example"))
        XCTAssertFalse(sentences.contains { $0.hasPrefix("onboarding.") })
        XCTAssertEqual(
            OnboardingView.message(for: .emptyCode, address: ""),
            OnboardingView.message(for: .stateMismatch, address: ""),
            "both sign-in failures read the same, without detail an attacker could use"
        )
    }

    @MainActor
    func testOnboardingBuildsWithoutWriting() {
        let store = FakePreferencesStore()
        XCTAssertGreaterThan(Hosting.render(OnboardingView(store: store, vault: MemorySecretVault())).bounds.width, 0)
        XCTAssertNil(store.data)
    }
}
