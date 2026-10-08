import SwiftUI

/// Canvas, ink, and accent. One palette so views do not invent colours.
public enum MailuneColor {
    public static let canvas = Color(red: 0.96, green: 0.95, blue: 0.93)
    public static let ink = Color(red: 0.12, green: 0.12, blue: 0.11)
    public static let accent = Color(red: 0.20, green: 0.36, blue: 0.55)
}

/// The two sizes the shell uses for titles and body text.
public enum MailuneType {
    public static let body = Font.system(size: 13, weight: .regular)
    public static let title = Font.system(size: 20, weight: .semibold)
}

/// Spacing steps. Views pad with these, not raw numbers.
public enum MailuneSpace {
    public static let s: CGFloat = 8
    public static let m: CGFloat = 16
}

/// Corner radii for cards and chips.
public enum MailuneRadius {
    public static let card: CGFloat = 10
}

/// Motion the shell shares. Short, so a highlight does not linger.
public enum MailuneMotion {
    public static let quick = Animation.easeInOut(duration: 0.18)
}
