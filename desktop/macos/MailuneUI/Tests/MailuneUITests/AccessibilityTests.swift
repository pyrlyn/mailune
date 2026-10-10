import SwiftUI
import XCTest
@testable import MailuneUI

final class AccessibilityTests: XCTestCase {
    func testContrastFollowsTheWCAGFormula() {
        let black = MailuneRGB(red: 0, green: 0, blue: 0)
        let white = MailuneRGB(red: 1, green: 1, blue: 1)
        XCTAssertEqual(MailuneRGB.contrast(black, white), 21, accuracy: 0.001)
        XCTAssertEqual(MailuneRGB.contrast(white, black), 21, accuracy: 0.001)
        XCTAssertEqual(MailuneRGB.contrast(MailunePalette.ink, MailunePalette.ink), 1, accuracy: 0.001)
        // #777777 on white is the textbook pair just under 4.5:1.
        let grey = MailuneRGB(red: 0x77 / 255, green: 0x77 / 255, blue: 0x77 / 255)
        XCTAssertEqual(MailuneRGB.contrast(grey, white), 4.48, accuracy: 0.01)
    }

    /// Body text is ink on canvas everywhere, so it gets the stricter AAA bar.
    func testInkOnCanvasMeetsAAA() {
        XCTAssertGreaterThanOrEqual(MailuneRGB.contrast(MailunePalette.ink, MailunePalette.canvas), 7)
    }

    func testShortcutsAreDistinctAndLeaveTheSystemOnesAlone() {
        func spelled(_ key: KeyEquivalent, _ modifiers: EventModifiers) -> String {
            "\(modifiers.rawValue)-\(key.character)"
        }
        let bound = MailuneShortcut.allCases.map { spelled($0.key, $0.modifiers) }
        XCTAssertEqual(Set(bound).count, bound.count)
        let reserved: [(KeyEquivalent, EventModifiers)] = [
            ("q", .command), ("w", .command), ("h", .command), ("m", .command), (",", .command),
            ("c", .command), ("v", .command), ("x", .command), ("a", .command), ("z", .command),
            ("z", [.command, .shift]), ("f", .command), (.tab, .command), (.space, .command),
        ]
        XCTAssertTrue(Set(bound).isDisjoint(with: reserved.map(spelled)))
        XCTAssertTrue(MailuneShortcut.replyPath.allSatisfy { MailuneShortcut.allCases.contains($0) })
    }

    func testLabelsAreInTheCatalogs() {
        for key in ["thread.reply", "compose.from", "thread.message_body", "compose.to", "ai.summary"] {
            XCTAssertNotEqual(Copy.text(key, language: "en"), key)
            XCTAssertNotEqual(Copy.text(key, language: "de"), Copy.text(key, language: "en"))
        }
        XCTAssertEqual(Copy.format("compose.from", "Ana", language: "en"), "From Ana")
    }
}
