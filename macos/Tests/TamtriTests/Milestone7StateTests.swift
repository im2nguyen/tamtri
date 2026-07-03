import Foundation
@testable import Tamtri
import XCTest

extension TranscriptContentBlock {
    static func fixture(
        type: String,
        text: String? = nil,
        name: String? = nil,
        callId: String? = nil,
        originToolCallId: String? = nil,
        path: String? = nil,
        mimeType: String? = nil,
        size: UInt64? = nil,
        sha256: String? = nil,
        serverId: String? = nil,
        templateRef: String? = nil,
        uri: String? = nil,
        taskId: String? = nil,
        taskStatus: String? = nil,
        taskTitle: String? = nil
    ) -> TranscriptContentBlock {
        TranscriptContentBlock(
            type: type,
            text: text,
            name: name,
            input: nil,
            callId: callId,
            output: nil,
            path: path,
            mimeType: mimeType,
            size: size,
            sha256: sha256,
            inline: nil,
            requestId: nil,
            serverId: serverId,
            originToolCallId: originToolCallId,
            mode: nil,
            message: nil,
            schema: nil,
            url: nil,
            action: nil,
            data: nil,
            uri: uri,
            templateRef: templateRef,
            state: nil,
            taskId: taskId,
            taskStatus: taskStatus,
            taskTitle: taskTitle,
            taskResultSummary: nil
        )
    }
}

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

    func testLiveEventGroupingNestsTaskUnderToolCall() {
        let events = [
            IdentifiedCoreEvent(
                id: 0,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "tool_call_started",
                    payloadJSON: #"{"id":"tool-2","name":"run_task"}"#
                )
            ),
            IdentifiedCoreEvent(
                id: 1,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "task_started",
                    payloadJSON: #"{"type":"task_started","state":{"taskId":"task-1","serverId":"tasks","status":"running","originToolCallId":"tool-2"}}"#
                )
            ),
        ]

        let groups = LiveEventGrouping.build(from: events)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].nested.count, 1)
        XCTAssertEqual(groups[0].nested[0].kind, "task_started")
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

    func testTranscriptContentGroupingNestsAppUnderToolCall() {
        let blocks = [
            TranscriptContentBlock.fixture(type: "tool_call", name: "show_app", callId: "tool-9"),
            TranscriptContentBlock.fixture(
                type: "app_resource",
                originToolCallId: "tool-9",
                serverId: "m7-app",
                templateRef: "ui://demo",
                uri: "ui://demo"
            ),
        ]

        let groups = TranscriptContentGrouping.build(from: blocks)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].nested.count, 1)
        XCTAssertEqual(groups[0].nested[0].type, "app_resource")
    }

    func testTranscriptContentGroupingNestsTaskRefUnderToolCall() {
        let blocks = [
            TranscriptContentBlock.fixture(type: "tool_call", name: "run_task", callId: "tool-2"),
            TranscriptContentBlock.fixture(
                type: "task_ref",
                originToolCallId: "tool-2",
                taskId: "task-1",
                taskStatus: "completed",
                taskTitle: "Import CSV"
            ),
        ]

        let groups = TranscriptContentGrouping.build(from: blocks)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].nested.count, 1)
        XCTAssertEqual(groups[0].nested[0].taskId, "task-1")
    }

    func testTranscriptContentGroupingNestsArtifactUnderToolCall() {
        let blocks = [
            TranscriptContentBlock.fixture(type: "tool_call", name: "write", callId: "tool-1"),
            TranscriptContentBlock.fixture(
                type: "artifact",
                originToolCallId: "tool-1",
                path: "attachments/report.html"
            ),
            TranscriptContentBlock.fixture(type: "text", text: "Done"),
        ]

        let groups = TranscriptContentGrouping.build(from: blocks)
        XCTAssertEqual(groups.count, 2)
        XCTAssertEqual(groups[0].nested.count, 1)
        XCTAssertEqual(groups[0].nested[0].type, "artifact")
        XCTAssertEqual(groups[1].standalone?.type, "text")
    }

    func testTranscriptArtifactsExtractsUniqueFrozenAttachments() {
        let messages = [
            ParsedTranscriptMessage(
                id: "m1",
                role: "assistant",
                harnessId: nil,
                content: [
                    TranscriptContentBlock.fixture(
                        type: "artifact",
                        path: "attachments/a.html",
                        mimeType: "text/html",
                        size: 12,
                        sha256: "abc"
                    ),
                    TranscriptContentBlock.fixture(
                        type: "artifact",
                        path: "attachments/a.html",
                        mimeType: "text/html",
                        size: 12,
                        sha256: "abc"
                    ),
                ],
                rawJSON: nil
            ),
            ParsedTranscriptMessage(
                id: "m2",
                role: "assistant",
                harnessId: nil,
                content: [
                    TranscriptContentBlock.fixture(
                        type: "artifact",
                        path: "attachments/b.csv",
                        mimeType: "text/csv",
                        size: 4,
                        sha256: "def"
                    ),
                ],
                rawJSON: nil
            ),
        ]

        let artifacts = TranscriptArtifacts.extract(from: messages)
        XCTAssertEqual(artifacts.count, 2)
        XCTAssertEqual(artifacts.map { $0.path }.sorted(), ["attachments/a.html", "attachments/b.csv"])
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
