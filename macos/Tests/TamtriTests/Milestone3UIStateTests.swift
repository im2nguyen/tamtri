import Foundation
@testable import Tamtri
import XCTest

final class Milestone3UIStateTests: XCTestCase {
    func testPermissionCardFromLivePayloadSnapshot() {
        let payload = """
        {
          "request_id": "perm-1",
          "action": "edit",
          "harness_display_name": "Mock ACP",
          "detail": {
            "type": "file_edit",
            "diff": {
              "path": "report.html",
              "change": "modified",
              "old_text": "",
              "new_text": "<h1>ok</h1>"
            }
          },
          "options": [
            {"id": "allow_once", "label": "Allow once"},
            {"id": "deny", "label": "Deny"}
          ]
        }
        """
        let presentation = PermissionCardPresentationBuilder.build(payloadJSON: payload)

        XCTAssertEqual(presentation?.harnessDisplayName, "Mock ACP")
        XCTAssertEqual(presentation?.diffPath, "report.html")
        XCTAssertEqual(presentation?.diffNewText, "<h1>ok</h1>")
        XCTAssertEqual(presentation?.allowOptionLabels, ["Allow once"])
        XCTAssertEqual(presentation?.rejectOptionLabels, ["Deny"])
        XCTAssertEqual(presentation?.accessibilityLabel, "Permission requested")
    }

    func testPermissionCardFromCommittedPayloadSnapshot() throws {
        let json = """
        {
          "type": "tool_result",
          "call_id": "perm-1",
          "output": {
            "permission": {
              "action": "edit",
              "status": "requested",
              "options": [{"id": "allow_once", "label": "Allow once"}]
            }
          }
        }
        """
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        let presentation = PermissionCardPresentationBuilder.fromCommittedPermissionBlock(block)

        XCTAssertEqual(presentation?.summary, "Action: edit")
        XCTAssertEqual(presentation?.allowOptionLabels, ["Allow once"])
        XCTAssertTrue(presentation?.rejectOptionLabels.isEmpty ?? false)
    }

    func testThinkingDisclosureFromCommittedPayloadSnapshot() throws {
        let json = #"{"type":"thinking","text":"thinking"}"#
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        let presentation = ThinkingDisclosurePresentationBuilder.fromCommittedBlock(block)

        XCTAssertEqual(presentation?.title, "Thinking")
        XCTAssertEqual(presentation?.text, "thinking")
        XCTAssertEqual(presentation?.initiallyExpanded, false)
    }

    func testThinkingDisclosureFromLivePayloadSnapshot() {
        let presentation = ThinkingDisclosurePresentationBuilder.fromLivePayloadJSON(#"{"text":"thinking"}"#)

        XCTAssertEqual(presentation?.title, "Thinking")
        XCTAssertEqual(presentation?.text, "thinking")
        XCTAssertEqual(presentation?.initiallyExpanded, false)
    }

    func testToolCardFromCommittedCallPayloadSnapshot() throws {
        let json = """
        {
          "type": "tool_call",
          "id": "tool-1",
          "name": "Write",
          "input": {"path": "report.html"}
        }
        """
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        let presentation = ToolCardPresentationBuilder.fromCommittedCallBlock(block)

        XCTAssertEqual(presentation?.title, "Write")
        XCTAssertTrue(presentation?.detail.contains("report.html") ?? false)
        XCTAssertEqual(presentation?.accessibilityLabel, "Tool call")
    }

    func testToolCardFromCommittedResultPayloadSnapshot() throws {
        let json = """
        {
          "type": "tool_result",
          "call_id": "tool-1",
          "output": {
            "status": "completed",
            "content": [{
              "type": "diff",
              "diff": {
                "path": "report.html",
                "change": "modified",
                "new_text": "<h1>ok</h1>"
              }
            }]
          }
        }
        """
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        let presentation = ToolCardPresentationBuilder.fromCommittedResultBlock(block)

        XCTAssertEqual(presentation?.title, "Tool result")
        XCTAssertEqual(presentation?.diffPath, "report.html")
        XCTAssertEqual(presentation?.subtitle, "modified • report.html")
    }

    func testToolCardFromLivePayloadSnapshot() {
        let presentation = ToolCardPresentationBuilder.fromLiveEvent(
            kind: "tool_call_started",
            payloadJSON: """
            {
              "id": "tool-1",
              "name": "Write",
              "title": "Write report",
              "status": "in_progress",
              "path": "report.html"
            }
            """
        )

        XCTAssertEqual(presentation.title, "Write")
        XCTAssertEqual(presentation.subtitle, "in_progress • report.html")
        XCTAssertEqual(presentation.accessibilityLabel, "Tool event")
    }
}
