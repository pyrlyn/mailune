import MailuneModel
import XCTest
@testable import MailuneUI

/// Each intent run the way Shortcuts runs it, on the fixture threads.
final class IntentTests: XCTestCase {
    func testSummarizeReturnsTheFixtureSummary() async throws {
        let intent = SummarizeThreadIntent()
        let row = try XCTUnwrap(ThreadFixtures.all.first { $0.id == "t1" })
        intent.thread = MailThreadEntity(row)
        _ = try await intent.perform()
        XCTAssertEqual(try MailIntents.summary(of: "t1"), AssistFixtures.assist(forThread: "t1")?.summary)
    }

    func testSummarizeGoesThroughTheEncryptedMailRule() throws {
        let message = try XCTUnwrap(MessageFixtures.message(forThread: "t3"))
        XCTAssertEqual(AssistFixtures.assist(forThread: "t3")?.source, .cloud)
        XCTAssertNotNil(AssistPolicy.fixture(for: message))
        var encrypted = message
        encrypted.security.encrypted = true
        XCTAssertNil(AssistPolicy.visible(AssistFixtures.assist(forThread: "t3"), for: encrypted))
    }

    func testSummarizeWithoutASummaryFails() async throws {
        var row = try XCTUnwrap(ThreadFixtures.all.first)
        row.id = "missing"
        let intent = SummarizeThreadIntent()
        intent.thread = MailThreadEntity(row)
        do {
            _ = try await intent.perform()
            XCTFail("a thread without a summary must not produce one")
        } catch {
            XCTAssertEqual(error as? MailIntentError, .noSummary)
        }
        XCTAssertNotEqual(String(localized: MailIntentError.noSummary.localizedStringResource), "intent.no_summary")
    }

    func testSearchUsesTheThreadListGrammar() async throws {
        let intent = SearchMailIntent()
        intent.query = "from:ana"
        _ = try await intent.perform()
        let expected = try SearchQuery.parse("from:ana").narrow(ThreadFixtures.all).map(\.id)
        XCTAssertFalse(expected.isEmpty)
        XCTAssertEqual(try MailIntents.search("from:ana").map(\.id), expected)
        let matched = try await MailThreadQuery().entities(matching: "from:ana")
        XCTAssertEqual(matched.map(\.id), expected)
    }

    func testSearchReportsABadQuery() async {
        let intent = SearchMailIntent()
        intent.query = "to:ana"
        do {
            _ = try await intent.perform()
            XCTFail("to: is not answerable from a thread row")
        } catch {
            XCTAssertEqual(error as? MailIntentError, .search(.unsupported("to:ana")))
        }
    }

    func testEntitiesCarryNoMessageText() async throws {
        let entities = try await MailThreadQuery().entities(for: ["t1", "nope"])
        XCTAssertEqual(entities.map(\.id), ["t1"])
        let body = try XCTUnwrap(MessageFixtures.message(forThread: "t1")).body
        let shown = Mirror(reflecting: entities[0]).children.compactMap { $0.value as? String }
        XCTAssertFalse(shown.contains(body))
    }

    @MainActor
    func testComposeOpensADraftAndSendsNothing() async throws {
        let requests = ComposeRequests()
        let intent = ComposeMessageIntent()
        intent.requests = requests
        intent.to = "ana@example.com, not-an-address; ANA@example.com ben@example.org"
        intent.subject = "Plan"
        intent.body = "See you Thursday."
        _ = try await intent.perform()

        let draft = try XCTUnwrap(requests.take())
        XCTAssertEqual(draft.to, ["ana@example.com", "ben@example.org"])
        XCTAssertEqual(draft.subject, "Plan")
        XCTAssertEqual(draft.body, "See you Thursday.")
        XCTAssertNil(requests.take())

        // The draft only reaches an outbox through the composer's confirm step.
        let outbox = FakeOutbox()
        let composer = Composer(outbox: outbox, draft: draft)
        composer.confirmSend(now: Date())
        XCTAssertTrue(outbox.held.isEmpty)
    }

    func testIntentTitlesAreInTheCatalogs() {
        for key in ["intent.summarize", "intent.conversation", "intent.query", "intent.no_summary"] {
            XCTAssertNotEqual(Copy.text(key, language: "en"), key)
            XCTAssertNotEqual(Copy.text(key, language: "de"), Copy.text(key, language: "en"))
        }
    }
}
