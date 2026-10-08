import MailunePlatform
import MailuneUI
import SwiftUI

@main
struct MailuneIOSApp: App {
    private let host = IOSHostServices.fakes()

    var body: some Scene {
        WindowGroup {
            AdaptiveMailbox()
                .accessibilityHint(host.refresh.isScheduled() ? "Scheduled" : "Idle")
        }
    }
}
