import SwiftUI

/// An sRGB colour as numbers, so a test computes contrast from the very values
/// the views draw.
public struct MailuneRGB: Equatable, Sendable {
    public var red: Double
    public var green: Double
    public var blue: Double

    public init(red: Double, green: Double, blue: Double) {
        self.red = red
        self.green = green
        self.blue = blue
    }

    public var color: Color { Color(red: red, green: green, blue: blue) }

    /// WCAG 2.2 relative luminance.
    public var luminance: Double {
        func linear(_ channel: Double) -> Double {
            channel <= 0.04045 ? channel / 12.92 : pow((channel + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
    }

    /// WCAG 2.2 contrast ratio, from 1 (none) to 21 (black on white).
    public static func contrast(_ first: Self, _ second: Self) -> Double {
        let (light, dark) = (max(first.luminance, second.luminance), min(first.luminance, second.luminance))
        return (light + 0.05) / (dark + 0.05)
    }
}

/// Canvas, ink, and accent. One palette so views do not invent colours.
public enum MailunePalette {
    public static let canvas = MailuneRGB(red: 0.96, green: 0.95, blue: 0.93)
    public static let ink = MailuneRGB(red: 0.12, green: 0.12, blue: 0.11)
    public static let accent = MailuneRGB(red: 0.20, green: 0.36, blue: 0.55)
}

public enum MailuneColor {
    public static let canvas = MailunePalette.canvas.color
    public static let ink = MailunePalette.ink.color
    public static let accent = MailunePalette.accent.color
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

/// Short, so a highlight does not linger.
public enum MailuneMotion {
    public static let quick = Animation.easeInOut(duration: 0.18)
}
