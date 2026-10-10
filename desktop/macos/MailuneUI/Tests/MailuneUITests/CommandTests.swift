import MailuneModel
import SwiftUI
import XCTest
@testable import MailuneUI

final class CommandTests: XCTestCase {
    func testEmptyQueryShowsEveryMailbox() {
        XCTAssertEqual(CommandList.filter("", language: "en").map(\.title), ["Go to Inbox", "Go to Sent"])
    }

    func testQueryKeepsTheMatchingCommand() {
        let found = CommandList.filter("sent", language: "en")
        XCTAssertEqual(found.map(\.action), [.open(.sent)])
    }

    func testQueryMatchesTheTranslatedTitle() {
        XCTAssertEqual(CommandList.filter("posteingang", language: "de").map(\.action), [.open(.inbox)])
        XCTAssertTrue(CommandList.filter("inbox", language: "de").isEmpty)
    }

    @MainActor
    func testShellBuilds() {
        let view = NSHostingView(rootView: ShellView().frame(width: 900, height: 600))
        view.layout()
        XCTAssertGreaterThan(view.fittingSize.width, 0)
    }
}
