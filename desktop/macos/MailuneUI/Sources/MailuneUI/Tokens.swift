import Foundation
import SwiftUI

/// sRGB channels the reader paints with. The contrast test uses the same numbers.
public enum MailunePalette {
    public static let canvas = (red: 0.96, green: 0.95, blue: 0.93)
    public static let ink = (red: 0.12, green: 0.12, blue: 0.11)
    public static let accent = (red: 0.20, green: 0.36, blue: 0.55)
}

/// WCAG contrast. A pair is acceptable at 4.5 for body text.
public enum MailuneContrast {
    public static func ratio(
        foreground: (red: Double, green: Double, blue: Double),
        background: (red: Double, green: Double, blue: Double)
    ) -> Double {
        let light = luminance(foreground)
        let dark = luminance(background)
        let lighter = max(light, dark)
        let darker = min(light, dark)
        return (lighter + 0.05) / (darker + 0.05)
    }

    public static var readerPair: Double {
        ratio(foreground: MailunePalette.ink, background: MailunePalette.canvas)
    }

    private static func luminance(_ color: (red: Double, green: Double, blue: Double)) -> Double {
        0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
    }

    private static func linear(_ channel: Double) -> Double {
        if channel <= 0.03928 {
            return channel / 12.92
        }
        return pow((channel + 0.055) / 1.055, 2.4)
    }
}

/// Canvas, ink, and accent. One palette so views do not invent colours.
public enum MailuneColor {
    public static let canvas = Color(
        red: MailunePalette.canvas.red,
        green: MailunePalette.canvas.green,
        blue: MailunePalette.canvas.blue
    )
    public static let ink = Color(
        red: MailunePalette.ink.red,
        green: MailunePalette.ink.green,
        blue: MailunePalette.ink.blue
    )
    public static let accent = Color(
        red: MailunePalette.accent.red,
        green: MailunePalette.accent.green,
        blue: MailunePalette.accent.blue
    )
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
