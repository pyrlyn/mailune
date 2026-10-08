//! Fixture mail the extensions can read without the network or a cloud model.

enum ExtensionMail {
    static let subject = "Hello"

    /// "Hello" sealed with a local mask. Opening it stays on device.
    static let sealed: [UInt8] = [0x12, 0x3F, 0x36, 0x36, 0x35]

    static func open(_ sealed: [UInt8]) -> String {
        let mask: UInt8 = 0x5A
        let bytes = sealed.map { $0 ^ mask }
        guard let text = String(bytes: bytes, encoding: .utf8) else {
            return ""
        }
        return text
    }
}
