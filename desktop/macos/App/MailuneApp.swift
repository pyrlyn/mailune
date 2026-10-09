import MailuneModel
import MailuneUI
import SwiftUI

@main
struct MailuneApp: App {
    @State private var preferences = DefaultsPreferencesStore()

    var body: some Scene {
        WindowGroup {
            ShellView(preferences: preferences)
        }
        Settings {
            SettingsView(store: preferences)
        }
    }
}
