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
    /// In memory until the core's keychain store (C1) is reachable over B3;
    /// the shell itself never writes a token to disk.
    @State private var vault = MemorySecretVault()

    var body: some Scene {
        WindowGroup {
            ShellView(preferences: preferences, host: Self.underUITest ? .fake() : .live())
        }
        Settings {
            SettingsView(store: preferences, vault: vault)
        }
    }
}
