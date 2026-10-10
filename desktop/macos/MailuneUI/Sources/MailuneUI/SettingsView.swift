import MailuneModel
import SwiftUI

/// Accounts, appearance, notifications, reading, compose and sync. Every
/// change is written through the store at once, as macOS settings windows do.
public struct SettingsView: View {
    private let store: any PreferencesStore
    @State private var settings: Preferences

    public init(store: any PreferencesStore) {
        self.store = store
        _settings = State(initialValue: store.current())
    }

    public var body: some View {
        Form {
            Section(Copy.text("settings.accounts")) {
                ForEach($settings.accounts) { $account in
                    LabeledContent {
                        TextField(Copy.text("settings.display_name"), text: $account.displayName)
                    } label: {
                        Text(verbatim: account.address)
                    }
                }
            }
            Section(Copy.text("settings.appearance")) {
                Picker(Copy.text("settings.theme"), selection: $settings.theme) {
                    ForEach(Theme.allCases, id: \.self) { Text(Copy.text($0.titleKey)).tag($0) }
                }
                Picker(Copy.text("settings.density"), selection: $settings.density) {
                    ForEach(Density.allCases, id: \.self) { Text(Copy.text($0.titleKey)).tag($0) }
                }
            }
            Section(Copy.text("settings.notifications")) {
                Toggle(Copy.text("settings.mail_notifications"), isOn: $settings.notifications)
                Toggle(Copy.text("settings.only_from_vips"), isOn: $settings.onlyVIPs)
                    .disabled(!settings.notifications)
            }
            Section(Copy.text("settings.reading")) {
                Toggle(Copy.text("settings.conversation_view"), isOn: $settings.conversationView)
                Toggle(Copy.text("settings.open_the_next_conversation"), isOn: $settings.autoAdvance)
            }
            Section(Copy.text("settings.compose")) {
                Picker(Copy.text("settings.undo_send"), selection: $settings.undoSendSeconds) {
                    ForEach(Preferences.undoSendChoices, id: \.self) { seconds in
                        Text(Copy.format("settings.seconds", String(seconds))).tag(seconds)
                    }
                }
                TextField(Copy.text("settings.signature"), text: $settings.signature, axis: .vertical)
            }
            Section(Copy.text("settings.sync")) {
                Picker(Copy.text("settings.days_of_mail_to"), selection: $settings.syncDays) {
                    ForEach(Preferences.syncDayChoices, id: \.self) { Text(verbatim: String($0)).tag($0) }
                }
                Toggle(Copy.text("settings.sync_on_wi_fi"), isOn: $settings.wifiOnly)
            }
            Section(Copy.text("settings.privacy_rules")) {
                Label(Copy.text("settings.ai_privacy"), systemImage: "lock.shield")
                    .accessibilityIdentifier("settings-ai-privacy")
            }
        }
        .formStyle(.grouped)
        .font(MailuneType.body)
        .frame(minWidth: 460, minHeight: 520)
        .onChange(of: settings) { _, updated in
            // Plain strings, numbers and enums always encode; a failure here
            // would be the store's medium, which has nowhere better to report.
            try? store.save(updated)
        }
    }
}
