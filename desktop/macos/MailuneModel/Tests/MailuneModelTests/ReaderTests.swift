import XCTest
@testable import MailuneModel

final class ReaderTests: XCTestCase {
    private func message(_ thread: String) throws -> MailMessage {
        try XCTUnwrap(MessageFixtures.message(forThread: thread))
    }

    func testQuotesStartCollapsedAndExpandOnRequest() throws {
        let plan = try message("t1")
        let collapsed = ReaderPresentation(message: plan, quotesCollapsed: true)
        XCTAssertTrue(collapsed.hasQuote)
        XCTAssertFalse(collapsed.text.contains("Can you share the plan"))

        let expanded = ReaderPresentation(message: plan, quotesCollapsed: false)
        XCTAssertTrue(expanded.text.hasPrefix(plan.body))
        XCTAssertTrue(expanded.text.contains("Can you share the plan"))
    }

    func testMessageWithoutQuoteHasNoToggle() throws {
        XCTAssertFalse(ReaderPresentation(message: try message("t2"), quotesCollapsed: false).hasQuote)
    }

    func testOneAttachment() throws {
        let presentation = ReaderPresentation(message: try message("t1"), quotesCollapsed: true)
        XCTAssertEqual(presentation.attachments, [Attachment(name: "quarterly-plan.pdf", size: 182_344)])
    }

    func testBadgeComesFromTheFixtureSecurityState() throws {
        XCTAssertEqual(ReaderPresentation(message: try message("t1"), quotesCollapsed: true).badge, .signed)
        XCTAssertEqual(ReaderPresentation(message: try message("t2"), quotesCollapsed: true).badge, .authenticated)
        XCTAssertEqual(ReaderPresentation(message: try message("t3"), quotesCollapsed: true).badge, .failed)
    }

    func testAFailedCheckOutranksEveryOtherSignal() {
        let forged = SecurityState(dkim: .pass, signature: .invalid, encrypted: true)
        XCTAssertEqual(SecurityBadge(forged), .failed)
        XCTAssertEqual(SecurityBadge(SecurityState(dkim: .fail, signature: .valid, encrypted: true)), .failed)
    }

    func testBadgeOrder() {
        XCTAssertEqual(SecurityBadge(SecurityState(dkim: .none, signature: .valid, encrypted: true)), .encrypted)
        XCTAssertEqual(SecurityBadge(SecurityState(dkim: .none, signature: .valid, encrypted: false)), .signed)
        XCTAssertEqual(SecurityBadge(SecurityState(dkim: .pass, signature: .none, encrypted: false)), .authenticated)
        for unknown in [DkimVerdict.none, .tempError, .permError] {
            XCTAssertEqual(SecurityBadge(SecurityState(dkim: unknown, signature: .none, encrypted: false)), .unverified)
        }
    }

    func testBlockedRemoteContentIsReported() throws {
        let receipt = ReaderPresentation(message: try message("t3"), quotesCollapsed: true)
        XCTAssertEqual(receipt.blockedRemote, RemoteSummary(blocked: 5, trackerPixels: 2))
        XCTAssertNil(ReaderPresentation(message: try message("t2"), quotesCollapsed: true).blockedRemote)
    }

    func testUnknownVerdictIsRejected() {
        let json = #"[{"id":"x","thread":"t","from":{"email":"a@b"},"subject":"","body":"","attachments":[],"security":{"dkim":"maybe","signature":"none","encrypted":false},"remote":{"blocked":0,"tracker_pixels":0}}]"#
        XCTAssertThrowsError(try MessageFixtures.decode(Data(json.utf8)))
    }
}
