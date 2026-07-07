import XCTest
@testable import Tamtri

final class LiveActivityGroupingTests: XCTestCase {
    func testMergesConsecutiveThoughtDeltasIntoOneItem() {
        let texts = ["0", "1", "2", "3", "4"]
        let events = texts.enumerated().map { index, text in
            IdentifiedCoreEvent(
                id: index,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "thought_delta",
                    payloadJSON: "{\"text\":\"\(text)\"}"
                )
            )
        }

        let segments = LiveActivityGrouping.build(from: events)
        XCTAssertEqual(segments.count, 1)
        guard case .activity(let cluster) = segments[0] else {
            return XCTFail("Expected one activity cluster")
        }
        XCTAssertEqual(cluster.items.count, 1)
        guard case .thinking(let presentation) = cluster.items[0].kind else {
            return XCTFail("Expected merged thinking item")
        }
        XCTAssertEqual(presentation.text, "01234")
        XCTAssertEqual(ActivitySummaryBuilder.summarize(items: cluster.items), "1 thought")
    }

    func testSplitsThoughtClustersAroundTextDelta() {
        let events = [
            IdentifiedCoreEvent(
                id: 0,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "thought_delta",
                    payloadJSON: #"{"text":"hmm"}"#
                )
            ),
            IdentifiedCoreEvent(
                id: 1,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "text_delta",
                    payloadJSON: #"{"text":"Hello"}"#
                )
            ),
            IdentifiedCoreEvent(
                id: 2,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "thought_delta",
                    payloadJSON: #"{"text":" wait"}"#
                )
            ),
        ]

        let segments = LiveActivityGrouping.build(from: events)
        XCTAssertEqual(segments.count, 3)
        guard case .activity(let firstCluster) = segments[0] else {
            return XCTFail("Expected first activity cluster")
        }
        XCTAssertEqual(firstCluster.items.count, 1)
        guard case .thinking(let firstThought) = firstCluster.items[0].kind else {
            return XCTFail("Expected first thought item")
        }
        XCTAssertEqual(firstThought.text, "hmm")
        guard case .event(let textEvent) = segments[1] else {
            return XCTFail("Expected merged text event segment")
        }
        XCTAssertEqual(textEvent.kind, "text_delta")
        XCTAssertEqual(LiveEventPayload.text(from: textEvent.payloadJSON), "Hello")
        guard case .activity(let secondCluster) = segments[2] else {
            return XCTFail("Expected second activity cluster")
        }
        XCTAssertEqual(secondCluster.items.count, 1)
        guard case .thinking(let secondThought) = secondCluster.items[0].kind else {
            return XCTFail("Expected second thought item")
        }
        XCTAssertEqual(secondThought.text, " wait")
    }

    func testMergesConsecutiveTextDeltasIntoOneSegment() {
        let deltas = ["Done", "Done.", "Done."]
        let events = deltas.enumerated().map { index, text in
            IdentifiedCoreEvent(
                id: index,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "text_delta",
                    payloadJSON: "{\"text\":\"\(text)\"}"
                )
            )
        }

        let segments = LiveActivityGrouping.build(from: events)
        XCTAssertEqual(segments.count, 1)
        guard case .event(let textEvent) = segments[0] else {
            return XCTFail("Expected one merged text segment")
        }
        XCTAssertEqual(LiveEventPayload.text(from: textEvent.payloadJSON), "Done.")
    }

    func testMergesIncrementalTextDeltasIntoOneSegment() {
        let deltas = ["Hello", " world"]
        let events = deltas.enumerated().map { index, text in
            IdentifiedCoreEvent(
                id: index,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "text_delta",
                    payloadJSON: "{\"text\":\"\(text)\"}"
                )
            )
        }

        let segments = LiveActivityGrouping.build(from: events)
        XCTAssertEqual(segments.count, 1)
        guard case .event(let textEvent) = segments[0] else {
            return XCTFail("Expected one merged text segment")
        }
        XCTAssertEqual(LiveEventPayload.text(from: textEvent.payloadJSON), "Hello world")
    }

    func testPairsToolCallStartedWithProgressCompletion() {
        let events = [
            IdentifiedCoreEvent(
                id: 0,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "tool_call_started",
                    payloadJSON: #"{"id":"tc-abc","name":"","input":{}}"#
                )
            ),
            IdentifiedCoreEvent(
                id: 1,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "tool_call_progress",
                    payloadJSON: """
                    {"type":"tool_call_progress","id":"tc-abc","status":"completed","content":[{"type":"json","value":{"kind":"execute","status":"completed","content":[{"type":"content","content":{"type":"text","text":"Execution complete\\n\\nOutput:\\nrows: 30"}}]}}]}
                    """
                )
            ),
        ]

        let segments = LiveActivityGrouping.build(from: events)
        XCTAssertEqual(segments.count, 1)
        guard case .activity(let cluster) = segments[0] else {
            return XCTFail("Expected activity cluster for paired tool completion")
        }
        XCTAssertEqual(cluster.items.count, 1)
        guard case .tool(let presentation) = cluster.items[0].kind else {
            return XCTFail("Expected tool card")
        }
        XCTAssertEqual(presentation.title, "Execute completed")
        XCTAssertTrue(presentation.detail.contains("rows: 30"))
    }
}
