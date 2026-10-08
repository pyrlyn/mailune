import MailuneModel
import MailunePlatform
import MailuneUI
import SwiftUI

@main
struct MailuneApp: App {
    private let host = MailunePlatform.Host()

    var body: some Scene {
        WindowGroup {
            ShellView()
                .navigationTitle(Mailbox.inbox.title)
                .accessibilityHint(host.system)
            .toolbar {
                ToolbarItem {
                    Image("ToolbarCompose")
                }
            }
        }
    }
}
