import Foundation
@testable import Tamtri
import XCTest

final class TranscriptPresentationTests: XCTestCase {
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

        XCTAssertEqual(presentation?.requestId, "perm-1")
        XCTAssertEqual(presentation?.harnessDisplayName, "Mock ACP")
        XCTAssertEqual(presentation?.diff?.path, "report.html")
        XCTAssertEqual(presentation?.diff?.newText, "<h1>ok</h1>")
        XCTAssertEqual(presentation?.allowOptions.map(\.label), ["Allow once"])
        XCTAssertEqual(presentation?.rejectOptions.map(\.label), ["Deny"])
        XCTAssertEqual(presentation?.accessibilityLabel, "Permission requested")
    }

    func testPermissionCardIncludesConversationScopeOption() {
        let payload = """
        {
          "request_id": "perm-2",
          "action": "execute",
          "detail": {"type": "command", "command": "npm test"},
          "options": [
            {"id": "allow_once", "label": "Allow once"},
            {"id": "allow_for_conversation", "label": "Allow for this conversation"},
            {"id": "deny", "label": "Deny"}
          ]
        }
        """
        let presentation = PermissionCardPresentationBuilder.build(payloadJSON: payload)

        XCTAssertEqual(presentation?.allowOptions.map(\.id), ["allow_once", "allow_for_conversation"])
        XCTAssertEqual(presentation?.allowOptions.map(\.label), ["Allow once", "Allow for this conversation"])
        XCTAssertEqual(presentation?.rejectOptions.map(\.id), ["deny"])
    }

    func testPermissionCardFromHarnessEventLivePayload() {
        let payload = """
        {
          "type": "permission_requested",
          "request_id": "0",
          "action": "edit",
          "harness_display_name": "Hermes ACP",
          "detail": {
            "type": "file_edit",
            "diff": {
              "path": "report.html",
              "change": "modified",
              "new_text": "<html></html>"
            }
          },
          "options": [
            {"id": "allow_once", "label": "Allow edit"},
            {"id": "deny", "label": "Deny"}
          ]
        }
        """
        let presentation = PermissionCardPresentationBuilder.build(payloadJSON: payload)

        XCTAssertEqual(presentation?.requestId, "0")
        XCTAssertEqual(presentation?.harnessDisplayName, "Hermes ACP")
        XCTAssertEqual(presentation?.diff?.path, "report.html")
        XCTAssertEqual(presentation?.allowOptions.map(\.id), ["allow_once"])
        XCTAssertEqual(presentation?.rejectOptions.map(\.id), ["deny"])
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

        XCTAssertEqual(presentation?.requestId, "perm-1")
        XCTAssertEqual(presentation?.summary, "Action: edit")
        XCTAssertEqual(presentation?.allowOptions.map(\.label), ["Allow once"])
        XCTAssertTrue(presentation?.rejectOptions.isEmpty ?? false)
    }

    func testStaleCommittedPermissionShowsReceipt() throws {
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
        let receipt = PermissionResolvedReceiptBuilder.fromCommittedBlock(block)

        XCTAssertEqual(receipt?.summary, "Permission · not resolved")
        XCTAssertEqual(receipt?.accessibilityLabel, "Permission request expired when the turn ended")
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

        XCTAssertEqual(presentation?.title, "modified · report.html")
        XCTAssertEqual(presentation?.diff?.path, "report.html")
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

        XCTAssertNotNil(presentation)
        XCTAssertEqual(presentation?.title, "Write")
        XCTAssertEqual(presentation?.subtitle, "in_progress • report.html")
        XCTAssertEqual(presentation?.accessibilityLabel, "Tool event")
    }
}
