import XCTest
@testable import MailunePlatform

final class ModelPolicyTests: XCTestCase {
    func testEmbeddingsAreTheDefaultAndALargerJobStaysOff() throws {
        let model = ScriptedOnDeviceModel(document: #"{"summary":"embedded"}"#)
        let policy = ModelPolicy()
        let embedded = try XCTUnwrap(OnDeviceJobs.run(.embeddings, policy: policy, model: model, prompt: "Embed"))
        XCTAssertEqual(embedded, #"{"summary":"embedded"}"#)
        XCTAssertNil(OnDeviceJobs.run(.larger, policy: policy, model: model, prompt: "Summarise"))

        let allowed = ModelPolicy(allowLargerJobs: true)
        let larger = try XCTUnwrap(OnDeviceJobs.run(.larger, policy: allowed, model: model, prompt: "Summarise"))
        XCTAssertEqual(larger, embedded)
    }
}
