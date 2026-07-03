import Foundation
@testable import Tamtri
import XCTest

final class Milestone8UIStateTests: XCTestCase {
    func testSearchScopeMessageNamesTitlesTextAndThinking() async {
        let client = MockCoreClient()
        let message = await client.searchScopeMessage()
        XCTAssertTrue(message.localizedCaseInsensitiveContains("title"))
        XCTAssertTrue(message.localizedCaseInsensitiveContains("Text"))
        XCTAssertTrue(message.localizedCaseInsensitiveContains("Thinking"))
    }

    func testHarnessHealthMockReportsReadyAgent() async throws {
        let client = MockCoreClient()
        let entries = try await client.listHarnessHealth()
        XCTAssertFalse(entries.isEmpty)
        XCTAssertTrue(entries.contains { $0.status == "ready" })
    }

    func testHarnessHealthChecklistIsCopyableText() async throws {
        let client = MockCoreClient()
        let checklist = try await client.harnessHealthChecklist()
        XCTAssertTrue(checklist.contains("tamtri harness setup checklist"))
    }
}
