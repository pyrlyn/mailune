import MailuneModel
import XCTest
@testable import MailunePlatform

@MainActor
final class IntegrationTests: XCTestCase {
    func testMailtoBecomesADraft() throws {
        let url = try XCTUnwrap(URL(string: "mailto:ana@acme.example,ben@ito.example?subject=Quarterly%20plan&body=Hi%20Ana&cc=x@y.example"))
        let draft = try XCTUnwrap(MailtoLink.draft(from: url))
        XCTAssertEqual(draft.to, ["ana@acme.example", "ben@ito.example"])
        XCTAssertEqual(draft.subject, "Quarterly plan")
        XCTAssertEqual(draft.body, "Hi Ana")
        XCTAssertNil(draft.attachment)
        XCTAssertNil(draft.sendAt)
    }

    func testMailtoReadsToFromTheQueryAndDropsJunk() throws {
        let url = try XCTUnwrap(URL(string: "MAILTO:?to=ana@acme.example,nope&attach=/etc/passwd&To=ana@acme.example"))
        let draft = try XCTUnwrap(MailtoLink.draft(from: url))
        XCTAssertEqual(draft.to, ["ana@acme.example"])
        XCTAssertNil(draft.attachment, "a link never attaches a file")
    }

    func testOtherSchemesAreNotMailto() throws {
        XCTAssertNil(MailtoLink.draft(from: try XCTUnwrap(URL(string: "file:///tmp/x"))))
    }

    func testDockBadgeCountsUnreadRows() {
        let dock = FakeDockBadge()
        UnreadBadge.update(dock, rows: ThreadFixtures.all)
        XCTAssertEqual(dock.label, "2")
        UnreadBadge.update(dock, rows: ThreadFixtures.all.filter { !$0.unread })
        XCTAssertNil(dock.label)
    }

    func testShareSendsSubjectAndBody() throws {
        let sharing = FakeSharing()
        let message = try XCTUnwrap(MessageFixtures.message(forThread: "t2"))
        sharing.share(ShareContent.text(for: message))
        XCTAssertEqual(sharing.shared, ["Lunch on Thursday\n\nSame place as last time works for me."])
    }

    func testSpotlightGetsSubjectAndSenderButNoText() {
        let spotlight = FakeSpotlight()
        spotlight.replace(with: SpotlightItems.items(for: ThreadFixtures.all))
        XCTAssertEqual(spotlight.items.first, SpotlightItem(id: "t1", title: "Quarterly plan", sender: "Ana Ruiz"))
        XCTAssertEqual(spotlight.items.count, ThreadFixtures.all.count)
        let indexed = spotlight.items.map { "\($0.title) \($0.sender)" }.joined()
        for row in ThreadFixtures.all {
            XCTAssertFalse(indexed.contains(row.snippet))
        }
    }
}
