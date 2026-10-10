import AppIntents
import MailuneModel
import MailunePlatform
import MailuneUI
import SwiftUI

@main
struct MailuneIOSApp: App {
    @State private var preferences = DefaultsPreferencesStore()
    @State private var requests: ComposeRequests

    init() {
        let requests = ComposeRequests()
        _requests = State(initialValue: requests)
        AppDependencyManager.shared.add(dependency: requests)
    }

    var body: some Scene {
        WindowGroup {
            // No iOS host integration exists yet, so badge, share and Spotlight
            // calls land in fakes rather than in macOS-only AppKit code.
            AdaptiveMailbox(preferences: preferences, host: .fake(), requests: requests)
        }
    }
}

/// The intents live in MailuneUI; listing its package here is what lets
/// Shortcuts find them in this app.
struct MailuneAppIntents: AppIntentsPackage {
    static var includedPackages: [any AppIntentsPackage.Type] {
        [MailuneIntentsPackage.self]
    }
}
