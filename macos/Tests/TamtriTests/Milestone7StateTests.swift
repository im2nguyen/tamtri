import Foundation
@testable import Tamtri
import XCTest

final class Milestone7StateTests: XCTestCase {
    func testLiveTaskStateParsesRunningAndCompleted() {
        let running = LiveTaskState(payloadJSON: #"{"state":{"task_id":"t-1","server_id":"tasks","status":"running","title":"Import CSV","progress":{"message":"Reading rows"}}}"#)
        XCTAssertEqual(running.taskId, "t-1")
        XCTAssertEqual(running.status, "running")
        XCTAssertEqual(running.title, "Import CSV")
        XCTAssertEqual(running.progressMessage, "Reading rows")
        XCTAssertFalse(running.isTerminal)

        let completed = LiveTaskState(payloadJSON: #"{"state":{"task_id":"t-1","status":"completed","title":"Import CSV"}}"#)
        XCTAssertEqual(completed.status, "completed")
        XCTAssertTrue(completed.isTerminal)
    }

    func testRootBookmarkMissingState() {
        let root = RootRecord(
            id: "root-1",
            name: "Reports",
            uri: "file:///tmp/reports",
            kind: "filesystem",
            scope: "conversation",
            bookmarkMissing: true
        )
        XCTAssertTrue(root.bookmarkMissing)
    }

    func testLiveEventGroupingNestsAppUnderToolCall() {
        let events = [
            IdentifiedCoreEvent(
                id: 0,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "tool_call_started",
                    payloadJSON: #"{"id":"tool-9","name":"show_app"}"#
                )
            ),
            IdentifiedCoreEvent(
                id: 1,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "app_returned",
                    payloadJSON: #"{"origin_tool_call_id":"tool-9","server_id":"m7-app","template_ref":"ui://demo","uri":"ui://demo","state":{}}"#
                )
            ),
        ]

        let groups = LiveEventGrouping.build(from: events)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].nested.count, 1)
        XCTAssertEqual(groups[0].nested[0].kind, "app_returned")
    }

    func testTranscriptTaskRefParsesTitleAndResultSummary() throws {
        let json = """
        {"type":"task_ref","task_id":"task-42","status":"completed","title":"Build report","result_summary":"Done"}
        """
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        XCTAssertEqual(block.taskId, "task-42")
        XCTAssertEqual(block.taskStatus, "completed")
        XCTAssertEqual(block.taskTitle, "Build report")
        XCTAssertEqual(block.taskResultSummary, "Done")
    }
}
