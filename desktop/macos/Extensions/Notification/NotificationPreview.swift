//! Local notification preview. The sealed fixture is opened on device and is never sent to a cloud model.

enum NotificationPreview {
    static func subject() -> String {
        ExtensionMail.open(ExtensionMail.sealed)
    }
}
