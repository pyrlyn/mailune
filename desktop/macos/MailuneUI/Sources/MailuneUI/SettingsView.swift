import MailuneModel
import SwiftUI

/// Accounts, appearance, notifications, reading, compose, and sync.
public struct SettingsView: View {
    @State private var settings: Preferences
    private let store: any PreferencesStore

    public init(store: any PreferencesStore) {
        self.store = store
        _settings = State(initialValue: store.load())
    }

    public var body: some View {
        Form {
            Section("Accounts") {
                TextField("Name", text: $settings.accountName)
            }
            Section("Appearance") {
                Picker("Appearance", selection: $settings.appearance) {
                    ForEach(Preferences.appearances, id: \.self) { Text($0).tag($0) }
                }
            }
            Section("Notifications") {
                Toggle("Notify", isOn: $settings.notifications)
            }
            Section("Reading") {
                Toggle("Reading pane", isOn: $settings.readingPane)
            }
            Section("Compose") {
                Stepper("Font size \(settings.composeFontSize)", value: $settings.composeFontSize, in: 12...24)
            }
            Section("Sync") {
                Stepper(
                    "Every \(settings.syncIntervalMinutes) minutes",
                    value: $settings.syncIntervalMinutes,
                    in: 1...60
                )
            }
            Section("Privacy") {
                Text(PrivacyCopy.summariesStayLocal)
            }
        }
        .font(MailuneType.body)
        .padding(MailuneSpace.m)
        .onChange(of: settings) { _, updated in
            store.save(updated)
        }
    }
}
