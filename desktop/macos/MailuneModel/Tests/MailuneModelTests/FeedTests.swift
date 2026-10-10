import XCTest
@testable import MailuneModel

private struct ScriptedSource: ThreadSource {
    var result: Result<[ThreadItem], Error>

    func fetch() async throws -> [ThreadItem] {
        try result.get()
    }
}

private struct Offline: Error {}

@MainActor
final class FeedTests: XCTestCase {
    func testPrimaryTabHidesOtherCategoriesAndArchivedRows() {
        let feed = ThreadFeed()
        feed.archive(["t2"])
        let rows = feed.visible(category: .primary)
        XCTAssertEqual(rows.map(\.id), ["t1"])
        XCTAssertTrue(rows[0].unread)
        XCTAssertEqual(feed.visible(category: .updates).map(\.subject), ["Your receipt"])
        XCTAssertEqual(feed.visible(category: nil).map(\.id), ["t1", "t3", "t4"])
    }

    func testRefreshReplacesRowsAndArchiveSurvivesIt() async {
        let fresh = Array(ThreadFixtures.all.prefix(2))
        let feed = ThreadFeed(source: ScriptedSource(result: .success(fresh)), rows: [])
        feed.archive(["t1"])
        await feed.refresh()
        XCTAssertEqual(feed.rows.map(\.id), ["t1", "t2"])
        XCTAssertEqual(feed.visible(category: nil).map(\.id), ["t2"])
        XCTAssertFalse(feed.refreshFailed)
    }

    func testAFailedRefreshKeepsTheRowsItHad() async {
        let feed = ThreadFeed(source: ScriptedSource(result: .failure(Offline())))
        await feed.refresh()
        XCTAssertTrue(feed.refreshFailed)
        XCTAssertEqual(feed.rows, ThreadFixtures.all)
    }
}
