import XCTest
@testable import MailuneModel

final class SearchTests: XCTestCase {
    private func ids(_ query: String) throws -> [String] {
        try SearchQuery.parse(query).narrow(ThreadFixtures.all).map(\.id)
    }

    func testParsesTheCoreGrammar() throws {
        let query = try SearchQuery.parse(#"from:ana is:unread has:attachment label:work "quarterly plan" draft"#)
        XCTAssertEqual(query.terms, [
            .from("ana"), .unread, .hasAttachment, .label("work"), .text("quarterly plan"), .text("draft"),
        ])
        XCTAssertEqual(try SearchQuery.parse(#"from:"Ana Ruiz""#).terms, [.from("Ana Ruiz")])
        XCTAssertEqual(try SearchQuery.parse("   ").terms, [])
    }

    func testTokensNarrowTheFixtureList() throws {
        XCTAssertEqual(try ids(""), ["t1", "t2", "t3", "t4"])
        XCTAssertEqual(try ids("is:unread"), ["t1", "t3"])
        XCTAssertEqual(try ids("is:unread has:attachment"), ["t1"])
        XCTAssertEqual(try ids("from:shop.example"), ["t3"])
        XCTAssertEqual(try ids("label:WORK"), ["t1"])
        XCTAssertEqual(try ids("rope"), ["t4"])
        XCTAssertEqual(try ids("from:ben rope"), [])
    }

    func testRejectsWhatTheCoreRejects() {
        XCTAssertThrowsError(try SearchQuery.parse("from:")) { XCTAssertEqual($0 as? SearchError, .bad("from")) }
        XCTAssertThrowsError(try SearchQuery.parse("is:starred")) { XCTAssertEqual($0 as? SearchError, .bad("is:starred")) }
        XCTAssertThrowsError(try SearchQuery.parse("colour:red")) { XCTAssertEqual($0 as? SearchError, .bad("colour:red")) }
        XCTAssertThrowsError(try SearchQuery.parse(#""open quote"#))
    }

    func testFieldsARowCannotAnswerAreReported() {
        XCTAssertThrowsError(try SearchQuery.parse("to:ben")) { XCTAssertEqual($0 as? SearchError, .unsupported("to:ben")) }
        XCTAssertThrowsError(try SearchQuery.parse("before:2026-10-01")) {
            XCTAssertEqual($0 as? SearchError, .unsupported("before:2026-10-01"))
        }
    }

    func testAskCitesTheFixtureMessage() throws {
        let answer = try XCTUnwrap(FixtureAsk.answer("When are the comments due?", from: MessageFixtures.all))
        XCTAssertEqual(answer.citations, [Citation(message: "m1", thread: "t1")])
        XCTAssertEqual(answer.text, "Could you send comments by Friday")
    }

    func testAskWithoutASourceGivesNoAnswer() {
        XCTAssertNil(FixtureAsk.answer("What is the weather?", from: MessageFixtures.all))
        XCTAssertNil(FixtureAsk.answer("is it ok", from: MessageFixtures.all))
    }
}
