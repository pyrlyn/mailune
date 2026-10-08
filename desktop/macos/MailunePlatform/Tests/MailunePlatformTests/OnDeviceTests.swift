import XCTest
@testable import MailunePlatform

final class OnDeviceTests: XCTestCase {
    func testAPromptReturnsJSONFromTheScriptedModel() throws {
        let model = ScriptedOnDeviceModel(document: #"{"summary":"Hello tomorrow"}"#)
        let reply = try OnDeviceSession.reply(prompt: "Summarise the thread", model: model)
        XCTAssertEqual(reply.summary, "Hello tomorrow")
    }

    func testAnEmptyPromptIsEmptyJSON() throws {
        let model = ScriptedOnDeviceModel(document: #"{"summary":"unused"}"#)
        let reply = try OnDeviceSession.reply(prompt: "", model: model)
        XCTAssertEqual(reply.summary, "")
    }
}
