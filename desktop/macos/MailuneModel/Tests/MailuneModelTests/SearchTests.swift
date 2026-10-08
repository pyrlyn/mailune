import XCTest
@testable import MailuneModel

final class SearchTests: XCTestCase {
    func testTokensNarrowTheFixtureList() {
        let tokens = [
            SearchToken(id: "1", field: "subject", value: "Hello"),
            SearchToken(id: "2", field: "category", value: "Primary"),
        ]
        let rows = ThreadSearch.narrow(ThreadFixtures.all, tokens: tokens)
        XCTAssertEqual(rows.map(\.id), ["1"])
    }

    func testAskCitesTheFixtureId() {
        let citation = ThreadSearch.cite(question: "Where is the receipt?", in: ThreadFixtures.all)
        XCTAssertEqual(citation?.fixtureID, "3")
        XCTAssertEqual(citation?.excerpt, "Receipt")
    }
}
