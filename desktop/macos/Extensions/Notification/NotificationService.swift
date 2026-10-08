//! Notification service. It replaces the title with the local preview and does not forward mail.

import UserNotifications

final class NotificationService: UNNotificationServiceExtension {
    private var handler: ((UNNotificationContent) -> Void)?
    private var delivered = false

    override func didReceive(
        _ request: UNNotificationRequest,
        withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void
    ) {
        handler = contentHandler
        guard let content = request.content.mutableCopy() as? UNMutableNotificationContent else {
            finish(request.content)
            return
        }
        content.title = NotificationPreview.subject()
        finish(content)
    }

    override func serviceExtensionTimeWillExpire() {
        finish(UNNotificationContent())
    }

    private func finish(_ content: UNNotificationContent) {
        guard !delivered else {
            return
        }
        delivered = true
        handler?(content)
    }
}
