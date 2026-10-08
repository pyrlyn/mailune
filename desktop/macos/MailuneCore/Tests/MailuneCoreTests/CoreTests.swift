import Foundation
import XCTest
@testable import MailuneCore

final class RecordingSink: ThreadSink, @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0

    func onThreads(threads: [ThreadRecord]) {
        lock.lock()
        count = threads.count
        lock.unlock()
    }

    var received: Int {
        lock.lock()
        defer { lock.unlock() }
        return count
    }
}

final class MailuneCoreTests: XCTestCase {
    func testPlatformIsMacOS() {
        XCTAssertEqual(MailuneCore.platform, "macOS")
    }

    func testCoreReady() async throws {
        try await coreReady()
    }

    func testCallbackReceivesTheRecord() {
        let sink = RecordingSink()
        publishThreads(
            threads: [ThreadRecord(id: "t1", subject: "Hello", unread: true)],
            sink: sink
        )
        XCTAssertEqual(sink.received, 1)
    }
}
