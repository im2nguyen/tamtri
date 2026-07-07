import XCTest
@testable import Tamtri

final class VisualPolishTests: XCTestCase {
    func testRelativeTimestampFromISO8601() {
        let recent = ISO8601DateFormatter().string(from: Date().addingTimeInterval(-120))
        let formatted = TamtriFormatting.relativeTimestamp(from: recent)
        XCTAssertFalse(formatted.contains("T"))
        XCTAssertFalse(formatted.contains(":"))
    }

    func testCompactRelativeTimestampUsesShortUnits() {
        let twoHoursAgo = ISO8601DateFormatter().string(from: Date().addingTimeInterval(-7_200))
        XCTAssertEqual(TamtriFormatting.compactRelativeTimestamp(from: twoHoursAgo), "2h")
        let threeDaysAgo = ISO8601DateFormatter().string(from: Date().addingTimeInterval(-259_200))
        XCTAssertEqual(TamtriFormatting.compactRelativeTimestamp(from: threeDaysAgo), "3d")
    }

    func testMessageBubbleTimestampUsesRelativeForRecentMessages() {
        let recent = ISO8601DateFormatter().string(from: Date().addingTimeInterval(-120))
        let formatted = TamtriFormatting.messageBubbleTimestamp(from: recent)
        XCTAssertNotNil(formatted)
        XCTAssertFalse(formatted?.contains("T") ?? true)
    }

    func testMessageBubbleTimestampUsesShortDateForOlderMessages() {
        let old = ISO8601DateFormatter().string(from: Date().addingTimeInterval(-172_800))
        let formatted = TamtriFormatting.messageBubbleTimestamp(from: old)
        XCTAssertNotNil(formatted)
        XCTAssertTrue(formatted?.contains(" ") ?? false)
    }

    func testMessageActionBarTimestampUsesWeekdayAndTime() {
        var components = DateComponents()
        components.year = 2026
        components.month = 7
        components.day = 3
        components.hour = 22
        components.minute = 50
        let date = Calendar.current.date(from: components)!
        let iso = ISO8601DateFormatter().string(from: date)
        let formatted = TamtriFormatting.messageActionBarTimestamp(from: iso)
        XCTAssertNotNil(formatted)
        XCTAssertTrue(formatted?.contains("10:50") ?? false)
        XCTAssertTrue(formatted?.lowercased().contains("pm") ?? false)
    }

    func testTranscriptTurnSpacingIsTighterThanBefore() {
        XCTAssertEqual(TamtriLayout.transcriptTurnSpacing, TamtriSpacing.sm)
        XCTAssertEqual(TamtriLayout.transcriptUserToAssistantGap, TamtriSpacing.sm)
    }

    func testTranscriptMessagePlainTextJoinsTextBlocks() {
        let message = ParsedTranscriptMessage(
            id: "m1",
            role: "user",
            harnessId: nil,
            content: [
                TranscriptContentBlock(type: "text", text: "Hello"),
                TranscriptContentBlock(type: "text", text: "World"),
            ],
            createdAt: nil,
            rawJSON: nil
        )
        XCTAssertEqual(TranscriptMessageText.plainText(from: message), "Hello\n\nWorld")
    }

    func testHarnessDisplayNameResolvesKnownAgent() {
        let agents = [HarnessAgentRecord(id: "hermes-acp", displayName: "Hermes")]
        XCTAssertEqual(HarnessDisplayNames.harness("hermes-acp", agents: agents), "Hermes")
    }

    func testSidebarSectionsWhenManyConversations() {
        let formatter = ISO8601DateFormatter()
        let now = formatter.string(from: Date())
        let yesterday = formatter.string(from: Date().addingTimeInterval(-86_400))
        let conversations = (0..<10).map { index in
            ConversationSummary(
                id: "id-\(index)",
                title: "Conversation \(index)",
                updatedAt: index.isMultiple(of: 2) ? now : yesterday,
                activeHarnessId: "hermes-acp"
            )
        }
        XCTAssertEqual(conversations.count, 10)
        XCTAssertNotNil(TamtriFormatting.parseDate(now))
    }

    func testToolCallBlockDecodesIdField() throws {
        let json = #"{"type":"tool_call","id":"tc-abc","name":"","input":{}}"#
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        XCTAssertEqual(block.callId, "tc-abc")
        XCTAssertFalse(block.hasToolCallBody)
    }

    func testToolResultExtractsKindAndPreview() throws {
        let json = """
        {"type":"tool_result","call_id":"tc-abc","output":{"status":"completed","content":[{"type":"json","value":{"kind":"execute","status":"completed","content":[{"type":"content","content":{"type":"text","text":"Execution complete"}}]}}]}}
        """
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        XCTAssertEqual(block.toolResultDisplayTitle, "Execute")
        XCTAssertEqual(block.toolResultPreviewText, "Execution complete")
    }

    func testToolResultExtractsMultilineHermesExecuteOutput() throws {
        let expected = "Execution complete\n\nOutput:\nsales.csv rows: 30\nexternal_refs_found: False"
        let encodedText = expected
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: "\\n")
        let json = """
        {"type":"tool_result","call_id":"tc-abc","output":{"status":"completed","content":[{"type":"json","value":{"kind":"execute","status":"completed","content":[{"type":"content","content":{"type":"text","text":"\(encodedText)"}}]}}]}}
        """
        let block = try JSONDecoder().decode(TranscriptContentBlock.self, from: Data(json.utf8))
        XCTAssertEqual(block.toolResultPreviewText, expected)
    }

    func testLiveToolProgressPayloadExtractsMultilineOutput() throws {
        let expected = "Execution complete\n\nOutput:\nsales.csv rows: 30"
        let encodedText = expected
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: "\\n")
        let json = """
        {"type":"tool_call_progress","id":"tc-abc","status":"completed","content":[{"type":"json","value":{"kind":"execute","status":"completed","content":[{"type":"content","content":{"type":"text","text":"\(encodedText)"}}]}}]}
        """
        let jsonValue = try XCTUnwrap(JSONValue.from(json: json))
        XCTAssertEqual(jsonValue.toolPayloadText, expected)
    }

    private var repoRoot: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
    }

    func testDesignTokenSyncProducesRendererCSS() throws {
        let tokens = repoRoot.appendingPathComponent("macos/Sources/Tamtri/Design/design-tokens.json")
        let css = repoRoot.appendingPathComponent("renderer/src/tokens.css")
        XCTAssertTrue(FileManager.default.fileExists(atPath: tokens.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: css.path))
        let cssText = try String(contentsOf: css, encoding: .utf8)
        XCTAssertTrue(cssText.contains("--tamtri-space-lg: 16px"))
        XCTAssertTrue(cssText.contains("--tamtri-max-line: 72ch"))
        XCTAssertTrue(cssText.contains("--tamtri-surface-composer"))
    }

    func testTamtriLayoutConstantsMatchDesignTokens() throws {
        let tokensURL = repoRoot.appendingPathComponent("macos/Sources/Tamtri/Design/design-tokens.json")
        let data = try Data(contentsOf: tokensURL)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        let layout = json?["layout"] as? [String: Any]
        XCTAssertEqual(layout?["contentMaxCh"] as? Int, 72)
        XCTAssertEqual(TamtriLayout.railMinWidth, 280)
        XCTAssertEqual(TamtriLayout.filesBrowseRailIdealWidth, 360)
        XCTAssertEqual(TamtriLayout.filesPreviewRailIdealWidth, 520)
        XCTAssertEqual(TamtriLayout.filesPreviewRailMinWidth, 400)
        XCTAssertEqual(TamtriLayout.filesPreviewRailMaxWidth, 720)
        XCTAssertEqual(TamtriSpacing.lg, 16)
    }

    func testPlutoTranscriptEncodesTextMessagesForRenderer() {
        let transcript = """
        [{"id":"019f2631-ad95-7a40-9a4d-a1916e011ce2","role":"user","content":[{"type":"text","text":"is pluto still a planet?"}],"created_at":"2026-07-03T04:16:57.749284Z"},{"id":"019f2631-c355-7413-8fb7-764319b0132b","role":"assistant","harness_id":"hermes-acp","content":[{"type":"text","text":"No — Pluto is **not officially a planet** anymore."}],"created_at":"2026-07-03T04:17:03.317792Z"}]
        """
        let messages = TranscriptParsing.parseTranscript(transcript)
        XCTAssertEqual(messages.count, 2)
        XCTAssertEqual(messages[0].content.first?.text, "is pluto still a planet?")

        let encoded = TranscriptRendererPayload.encode(messages: messages)
        XCTAssertTrue(encoded.contains("is pluto still a planet?"))
        XCTAssertTrue(encoded.contains("not officially a planet"))
        XCTAssertNotEqual(encoded, "[]")
    }
}
