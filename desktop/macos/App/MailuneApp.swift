import MailuneModel
import MailunePlatform
import MailuneUI
import SwiftUI

@main
struct MailuneApp: App {
    private let host = MailunePlatform.Host()

    var body: some Scene {
        WindowGroup {
            VStack {
                InboxView()
                Text(host.system)
            }
            .navigationTitle(Mailbox.inbox.title)
            .toolbar {
                ToolbarItem {
                    Image("ToolbarCompose")
                }
            }
        }
    }
}
