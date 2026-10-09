import MailuneModel
import MailunePlatform
import MailuneUI
import SwiftUI

@main
struct MailuneApp: App {
    /// Set by the UI tests so a run never writes the Dock, Spotlight or the
    /// user's saved settings.
    private static let underUITest = CommandLine.arguments.contains("-mailune-ui-test")

    @State private var preferences: any PreferencesStore =
        underUITest ? FakePreferencesStore() : DefaultsPreferencesStore()

    var body: some Scene {
        WindowGroup {
            ShellView(preferences: preferences, host: Self.underUITest ? .fake() : .live())
        }
        Settings {
            SettingsView(store: preferences)
        }
    }
}
