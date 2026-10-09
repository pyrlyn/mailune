import XCTest
@testable import MailuneUI

final class CopyTests: XCTestCase {
    func testGeneratedCatalogsCarryEnglishAndASecondLanguage() {
        XCTAssertEqual(Copy.text("app.inbox", language: "en"), "Inbox")
        XCTAssertEqual(Copy.text("app.inbox", language: "de"), "Posteingang")
    }

    func testUnknownLanguageFallsBackToEnglish() {
        XCTAssertEqual(Copy.text("app.inbox", language: "xx"), "Inbox")
    }

    func testPlaceholderIsFilled() {
        XCTAssertEqual(Copy.format("data.archived", "Notes", language: "en"), "Archived “Notes”")
    }

    func testKeyMissingFromOneCatalogFallsBackToEnglish() throws {
        let bundle = try catalogs([
            "en": ["shared": "Shared", "only_en": "English only"],
            "de": ["shared": "Geteilt"],
        ])
        XCTAssertEqual(Copy.text("shared", language: "de", in: bundle), "Geteilt")
        XCTAssertEqual(Copy.text("only_en", language: "de", in: bundle), "English only")
        XCTAssertEqual(Copy.text("nowhere", language: "de", in: bundle), "nowhere")
    }

    /// Writes catalogs in the format scripts/i18n.py emits into a throwaway bundle.
    private func catalogs(_ tables: [String: [String: String]]) throws -> Bundle {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: root) }
        for (language, entries) in tables {
            let lproj = root.appendingPathComponent("\(language).lproj", isDirectory: true)
            try FileManager.default.createDirectory(at: lproj, withIntermediateDirectories: true)
            let body = entries.map { "\"\($0.key)\" = \"\($0.value)\";" }.joined(separator: "\n")
            try body.write(to: lproj.appendingPathComponent("Localizable.strings"), atomically: true, encoding: .utf8)
        }
        return try XCTUnwrap(Bundle(path: root.path))
    }
}
