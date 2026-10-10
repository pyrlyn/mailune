import MailuneModel
import MailunePlatform
import MailuneUI
import SwiftUI

@main
struct MailuneIOSApp: App {
    @State private var preferences = DefaultsPreferencesStore()

    var body: some Scene {
        WindowGroup {
            // No iOS host integration exists yet, so badge, share and Spotlight
            // calls land in fakes rather than in macOS-only AppKit code.
            AdaptiveMailbox(preferences: preferences, host: .fake())
        }
    }
}
