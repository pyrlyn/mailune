import MailuneUI
import SwiftUI

@main
struct MailuneApp: App {
    var body: some Scene {
        WindowGroup {
            ShellView()
                .toolbar {
                    ToolbarItem {
                        Image("ToolbarCompose")
                            .accessibilityLabel(Copy.text("app.compose"))
                    }
                }
        }
    }
}
