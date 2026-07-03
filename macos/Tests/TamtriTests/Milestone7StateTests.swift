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
        XCTAssertTrue(RootRowViewModel.showsRepickButton(bookmarkMissing: root.bookmarkMissing))
        XCTAssertEqual(
            RootRowViewModel.warningMessage(bookmarkMissing: true),
            RootBookmarkStatus.missingBookmarkWarning
        )
        XCTAssertEqual(
            RootBookmarkStatus.missingBookmarkError(rootName: root.name),
            "Missing access bookmark for root \"Reports\". Re-pick the folder in conversation settings."
        )
    }

    func testKnowledgeBaseRootNeverMarkedBookmarkMissing() {
        XCTAssertFalse(
            RootBookmarkStatus.isBookmarkMissing(
                kind: "knowledge_base",
                conversationId: "conv-1",
                rootId: "kb-1"
            )
        )
    }

    func testRootBookmarkDeletedSurfacesMissingFlag() throws {
        let conversationId = "conv-\(UUID().uuidString)"
        let rootId = "root-\(UUID().uuidString)"
        defer { try? RootBookmarkStore.deleteBookmark(conversationId: conversationId, rootId: rootId) }

        try RootBookmarkStore.saveBookmark(data: Data("mock-bookmark".utf8), conversationId: conversationId, rootId: rootId)
        XCTAssertFalse(
            RootBookmarkStatus.isBookmarkMissing(kind: "filesystem", conversationId: conversationId, rootId: rootId)
        )

        try RootBookmarkStore.deleteBookmark(conversationId: conversationId, rootId: rootId)
        XCTAssertTrue(
            RootBookmarkStatus.isBookmarkMissing(kind: "filesystem", conversationId: conversationId, rootId: rootId)
        )
    }

    func testFilesystemRootAttachFlowClearsMissingFlagAfterBookmarkSave() throws {
        let conversationId = "conv-\(UUID().uuidString)"
        let rootId = "root-\(UUID().uuidString)"
        defer { try? RootBookmarkStore.deleteBookmark(conversationId: conversationId, rootId: rootId) }

        XCTAssertTrue(
            RootBookmarkStatus.isBookmarkMissing(kind: "filesystem", conversationId: conversationId, rootId: rootId)
        )
        try RootBookmarkStore.saveBookmark(data: Data("attach-flow".utf8), conversationId: conversationId, rootId: rootId)
        XCTAssertFalse(
            RootBookmarkStatus.isBookmarkMissing(kind: "filesystem", conversationId: conversationId, rootId: rootId)
        )
    }

    @MainActor
    func testComposerAttachRootOpensRootsSheet() async throws {
        let store = AppStore(core: MockCoreClient())
        let summary = ConversationSummary(id: "sample", title: "Report from CSV", updatedAt: "now")
        store.selectConversation(summary)
        XCTAssertFalse(store.showConversationRoots)
        store.presentConversationRoots()
        XCTAssertTrue(store.showConversationRoots)
    }

    func testAppPanelViewModelOfflineAndLoadedSnapshots() {
        XCTAssertEqual(AppPanelViewModel.accessibilityValue(templateLoaded: false), "offline")
        XCTAssertEqual(AppPanelViewModel.accessibilityValue(templateLoaded: true), "loaded")
        XCTAssertEqual(AppPanelViewModel.accessibilityLabel(title: "ui://demo"), "MCP App ui://demo")
        XCTAssertTrue(AppPanelViewModel.offlineMessage.contains("offline"))
        XCTAssertTrue(AppPanelViewModel.offlineMessage.contains("gateway"))
    }

    func testAppBridgeConsentViewModelSnapshot() throws {
        let payload = #"{"request_id":"bridge-1","server_id":"m7-app","app_id":"ui://demo","template_ref":"ui://demo","summary":"Call echo","options":[{"id":"deny","label":"Deny"}]}"#
        let request = try JSONDecoder().decode(AppBridgeConsentPayload.self, from: Data(payload.utf8))
        XCTAssertEqual(AppBridgeConsentViewModel.headline, "App action needs consent")
        XCTAssertEqual(
            AppBridgeConsentViewModel.serverAttribution(serverId: request.serverId, appId: request.appId),
            "Server: m7-app · App: ui://demo"
        )
    }

    func testTaskLiveCardViewModelRunningCompletedFailedSnapshots() {
        let running = LiveTaskState(payloadJSON: #"{"state":{"task_id":"t-1","server_id":"tasks","status":"running","title":"Import CSV","progress":{"message":"Reading rows"}}}"#)
        XCTAssertEqual(TaskLiveCardViewModel.statusIcon(for: running.status), "clock")
        XCTAssertTrue(TaskLiveCardViewModel.showsCancelButton(for: running))
        XCTAssertEqual(TaskLiveCardViewModel.accessibilityStatus(for: running), "running, Reading rows")

        let completed = LiveTaskState(payloadJSON: #"{"state":{"task_id":"t-1","status":"completed","title":"Import CSV"}}"#)
        XCTAssertEqual(TaskLiveCardViewModel.statusIcon(for: completed.status), "checkmark.circle")
        XCTAssertFalse(TaskLiveCardViewModel.showsCancelButton(for: completed))

        let failed = LiveTaskState(payloadJSON: #"{"state":{"task_id":"t-2","status":"failed","title":"Import CSV"}}"#)
        XCTAssertEqual(TaskLiveCardViewModel.statusIcon(for: failed.status), "xmark.circle")
        XCTAssertFalse(TaskLiveCardViewModel.showsCancelButton(for: failed))
    }

    func testTaskRefCardViewModelCompletedSnapshot() {
        let block = TranscriptContentBlock.fixture(
            type: "task_ref",
            taskId: "task-42",
            taskStatus: "completed",
            taskTitle: "Build report"
        )
        XCTAssertEqual(TaskRefCardViewModel.title(block: block), "Build report")
        XCTAssertEqual(
            TaskRefCardViewModel.accessibilityValue(status: "completed", resultSummary: "Done"),
            "completed, Done"
        )
    }

    func testRootRowViewModelAttachedAndMissingSnapshots() {
        let attached = RootRecord(
            id: "root-1",
            name: "Data",
            uri: "file:///tmp/data",
            kind: "filesystem",
            scope: "conversation",
            bookmarkMissing: false
        )
        XCTAssertFalse(RootRowViewModel.showsRepickButton(bookmarkMissing: attached.bookmarkMissing))
        XCTAssertNil(RootRowViewModel.warningMessage(bookmarkMissing: attached.bookmarkMissing))

        let missing = RootRecord(
            id: "root-2",
            name: "Reports",
            uri: "file:///tmp/reports",
            kind: "filesystem",
            scope: "conversation",
            bookmarkMissing: true
        )
        XCTAssertTrue(RootRowViewModel.showsRepickButton(bookmarkMissing: missing.bookmarkMissing))
        XCTAssertEqual(RootRowViewModel.warningMessage(bookmarkMissing: true), RootBookmarkStatus.missingBookmarkWarning)
    }

    func testCapabilityBadgeSamplingDeclinedSnapshot() {
        XCTAssertEqual(CapabilityBadgeViewModel.effectiveStatus(title: "Sampling", status: "supported"), "declined")
        XCTAssertEqual(
            CapabilityBadgeViewModel.accessibilityText(title: "Sampling", status: "supported"),
            "Sampling declined by design. The model lives in the harness."
        )
        XCTAssertEqual(CapabilityBadgeViewModel.effectiveStatus(title: "Apps", status: "supported"), "supported")
        XCTAssertEqual(
            CapabilityBadgeViewModel.accessibilityText(title: "Apps", status: "server_only"),
            "Apps server only"
        )
    }

    func testAppOfflineAccessibilitySemantics() {
        XCTAssertEqual(AppPanelViewModel.accessibilityValue(templateLoaded: false), "offline")
        XCTAssertEqual(AppPanelViewModel.accessibilityValue(templateLoaded: true), "loaded")
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
        {"type":"task_ref","task_id":"task-42","status":"completed","title":"Build report","result_summary":"Done","origin_tool_call_id":"tool-42"}
        """
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        XCTAssertEqual(block.taskId, "task-42")
        XCTAssertEqual(block.taskStatus, "completed")
        XCTAssertEqual(block.taskTitle, "Build report")
        XCTAssertEqual(block.taskResultSummary, "Done")
        XCTAssertEqual(block.originToolCallId, "tool-42")
    }
}
