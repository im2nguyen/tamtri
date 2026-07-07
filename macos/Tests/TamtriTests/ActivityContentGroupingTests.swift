import XCTest
@testable import Tamtri

final class ActivityContentGroupingTests: XCTestCase {
    func testSummarizeMixedActivity() {
        let items = [
            ActivityItemModel(id: 0, kind: .thinking(ThinkingDisclosurePresentation(title: "Thinking", text: "a", initiallyExpanded: false))),
            ActivityItemModel(id: 1, kind: .tool(ToolCardPresentation(title: "Read", subtitle: "file.txt", detail: "", diff: nil, accessibilityLabel: "Tool"))),
            ActivityItemModel(id: 2, kind: .tool(ToolCardPresentation(title: "Write", subtitle: "out.html", detail: "", diff: nil, accessibilityLabel: "Tool"))),
        ]
        XCTAssertEqual(ActivitySummaryBuilder.summarize(items: items), "1 thought, 1 read, 1 edit")
    }

    func testMessageSegmentsClusterConsecutiveMutedBlocks() throws {
        let json = """
        [
          {"type":"thinking","text":"plan"},
          {"type":"tool_call","id":"t1","name":"Read","input":{"path":"sales.csv"}},
          {"type":"text","text":"Done."}
        ]
        """
        let blocks = try JSONDecoder().decode([TranscriptContentBlock].self, from: Data(json.utf8))
        let segments = ActivityContentGrouping.messageSegments(from: blocks)
        XCTAssertEqual(segments.count, 2)
        guard case .activityCluster(let cluster) = segments[0] else {
            return XCTFail("Expected activity cluster first")
        }
        XCTAssertEqual(cluster.items.count, 2)
        guard case .text = segments[1] else {
            return XCTFail("Expected text segment second")
        }
    }

    func testMessageSegmentsPairsHermesExecuteResultWithEmptyToolCall() throws {
        let expected = "Execution complete\\n\\nOutput:\\nsales.csv rows: 30"
        let json = """
        [
          {"type":"tool_call","id":"tc-abc","name":"","input":{}},
          {"type":"tool_result","call_id":"tc-abc","output":{"status":"completed","content":[{"type":"json","value":{"kind":"execute","status":"completed","content":[{"type":"content","content":{"type":"text","text":"\(expected)"}}]}}]}},
          {"type":"text","text":"Done."}
        ]
        """
        let blocks = try JSONDecoder().decode([TranscriptContentBlock].self, from: Data(json.utf8))
        let segments = ActivityContentGrouping.messageSegments(from: blocks)
        XCTAssertEqual(segments.count, 2)
        guard case .activityCluster(let cluster) = segments[0] else {
            return XCTFail("Expected activity cluster for execute tool result")
        }
        XCTAssertEqual(cluster.items.count, 1)
        guard case .tool(let presentation) = cluster.items[0].kind else {
            return XCTFail("Expected tool card in activity cluster")
        }
        XCTAssertEqual(presentation.title, "Execute completed")
        XCTAssertTrue(presentation.detail.contains("sales.csv rows: 30"))
        guard case .text = segments[1] else {
            return XCTFail("Expected trailing assistant text")
        }
    }

    func testMessageSegmentsSurfacePermissionBlocksNatively() throws {
        let json = """
        [
          {"type":"tool_call","id":"t1","name":"Execute","input":{}},
          {
            "type":"tool_result",
            "call_id":"0",
            "output":{
              "permission":{
                "action":"edit",
                "status":"requested",
                "options":[{"id":"allow_once","label":"Allow edit"},{"id":"deny","label":"Deny"}]
              }
            }
          }
        ]
        """
        let blocks = try JSONDecoder().decode([TranscriptContentBlock].self, from: Data(json.utf8))
        let segments = ActivityContentGrouping.messageSegments(from: blocks)
        XCTAssertEqual(segments.count, 2)
        guard case .activityCluster = segments[0] else {
            return XCTFail("Expected empty execute call in activity cluster")
        }
        guard case .rich(let block) = segments[1] else {
            return XCTFail("Expected permission block as rich native segment")
        }
        XCTAssertNotNil(PermissionCardPresentationBuilder.fromCommittedPermissionBlock(block))
    }
}
