import Foundation
@testable import Tamtri
import XCTest

final class Milestone3RelaunchTests: XCTestCase {
    private let conversationId = "018e1234-5678-7890-abcd-ef0123456789"
    private let folderName = "2024-03-15-m3-relaunch--018e123456787890abcdef0123456789"

    func testRelaunchRedrawsParsedMessagesWithoutLiveEvents() async throws {
        let vaultURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("tamtri-m3-relaunch-\(UUID().uuidString)", isDirectory: true)
        try installFixture(at: vaultURL)
        defer { try? FileManager.default.removeItem(at: vaultURL) }

        let client = try TamtriBindingClient(vaultPath: vaultURL.path)
        let record = try await client.loadConversation(id: conversationId)

        XCTAssertEqual(record.parsedMessages.count, 2)
        XCTAssertEqual(record.parsedMessages[0].role, "user")
        XCTAssertEqual(record.parsedMessages[1].role, "assistant")
        XCTAssertEqual(record.parsedMessages[1].harnessId, "mock-acp")

        let blocks = record.parsedMessages[1].content
        XCTAssertEqual(blocks.map(\.type), ["thinking", "text", "tool_call", "tool_result"])

        let thinking = try XCTUnwrap(
            ThinkingDisclosurePresentationBuilder.fromCommittedBlock(blocks[0])
        )
        XCTAssertEqual(thinking.text, "Let me write the report.")

        let toolCall = try XCTUnwrap(
            ToolCardPresentationBuilder.fromCommittedCallBlock(blocks[2])
        )
        XCTAssertEqual(toolCall.title, "Write")

        let toolResult = try XCTUnwrap(
            ToolCardPresentationBuilder.fromCommittedResultBlock(blocks[3])
        )
        XCTAssertEqual(toolResult.diff?.path, "report.html")
    }

    private func installFixture(at vaultURL: URL) throws {
        let conversationsURL = vaultURL.appendingPathComponent("conversations", isDirectory: true)
        let conversationURL = conversationsURL.appendingPathComponent(folderName, isDirectory: true)
        try FileManager.default.createDirectory(at: conversationURL, withIntermediateDirectories: true)

        let metaJSON = """
        {
          "schema_version": 1,
          "id": "\(conversationId)",
          "title": "M3 Relaunch",
          "created_at": "2024-03-15T12:00:00Z",
          "updated_at": "2024-03-15T12:05:00Z",
          "active_harness_id": "mock-acp",
          "model_id": "mock",
          "working_dir": {"mode": "vault_local"},
          "mcp_servers": [],
          "roots": []
        }
        """
        try metaJSON.write(
            to: conversationURL.appendingPathComponent("meta.json"),
            atomically: true,
            encoding: .utf8
        )

        let userMessage = """
        {"id":"018e1234-5678-7890-abcd-ef012345678a","role":"user","content":[{"type":"text","text":"hello"}],"created_at":"2024-03-15T12:00:01Z"}
        """
        let assistantMessage = """
        {"id":"018e1234-5678-7890-abcd-ef012345678b","role":"assistant","harness_id":"mock-acp","content":[{"type":"thinking","text":"Let me write the report."},{"type":"text","text":"Done."},{"type":"tool_call","id":"tool-1","name":"Write","input":{"path":"report.html"}},{"type":"tool_result","call_id":"tool-1","output":{"status":"completed","content":[{"type":"diff","diff":{"path":"report.html","change":"modified","new_text":"<h1>ok</h1>"}}]}}],"created_at":"2024-03-15T12:05:00Z"}
        """
        let messages = [userMessage, assistantMessage].joined(separator: "\n") + "\n"
        try messages.write(
            to: conversationURL.appendingPathComponent("messages.jsonl"),
            atomically: true,
            encoding: .utf8
        )
        FileManager.default.createFile(
            atPath: conversationURL.appendingPathComponent("events.jsonl").path,
            contents: Data()
        )
    }
}
