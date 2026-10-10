import XCTest
@testable import MailuneModel

final class AssistTests: XCTestCase {
    private func message(_ thread: String) throws -> MailMessage {
        try XCTUnwrap(MessageFixtures.message(forThread: thread))
    }

    func testSummaryAndThreeRepliesComeFromFixtures() throws {
        let assist = try XCTUnwrap(AssistFixtures.assist(forThread: "t1"))
        XCTAssertTrue(assist.summary.contains("Friday"))
        XCTAssertEqual(assist.actionItems.count, 2)
        XCTAssertEqual(assist.replies.count, 3)
        XCTAssertEqual(assist.source, .device)
        XCTAssertTrue(AssistFixtures.all.allSatisfy { $0.replies.count == 3 })
    }

    func testPolicyShowsTheThreadsOwnAssist() throws {
        let plan = try message("t1")
        XCTAssertEqual(AssistPolicy.visible(AssistFixtures.assist(forThread: "t1"), for: plan)?.thread, "t1")
        XCTAssertNil(AssistPolicy.visible(AssistFixtures.assist(forThread: "t2"), for: plan))
        XCTAssertNil(AssistPolicy.visible(nil, for: plan))
    }

    func testCloudTextAboutEncryptedMailIsHidden() throws {
        var receipt = try message("t3")
        let cloud = try XCTUnwrap(AssistFixtures.assist(forThread: "t3"))
        XCTAssertEqual(cloud.source, .cloud)
        XCTAssertNotNil(AssistPolicy.visible(cloud, for: receipt))

        receipt.security.encrypted = true
        XCTAssertNil(AssistPolicy.visible(cloud, for: receipt))
        var local = cloud
        local.source = .device
        XCTAssertNotNil(AssistPolicy.visible(local, for: receipt), "a summary made on this Mac may still show")
    }

    func testReplyChipBecomesADraftToTheSender() throws {
        let plan = try message("t1")
        let draft = Draft.reply(to: plan, body: "Thanks, reading it now.")
        XCTAssertEqual(draft.to, ["ana@acme.example"])
        XCTAssertEqual(draft.subject, "Re: Quarterly plan")
        XCTAssertEqual(draft.body, "Thanks, reading it now.")

        var already = plan
        already.subject = "RE: Quarterly plan"
        XCTAssertEqual(Draft.reply(to: already, body: "").subject, "RE: Quarterly plan")
    }
}
