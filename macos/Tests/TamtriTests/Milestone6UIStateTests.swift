import Foundation
@testable import Tamtri
import XCTest

final class Milestone6UIStateTests: XCTestCase {
    func testElicitationFormCardStateSnapshot() {
        let schema: JSONValue = .object([
            "type": .string("object"),
            "properties": .object([
                "animal": .object([
                    "type": .string("string"),
                    "title": .string("Animal"),
                    "enum": .array([.string("cat"), .string("dog")])
                ])
            ]),
            "required": .array([.string("animal")])
        ])

        XCTAssertEqual(ElicitationCardRouter.cardKind(mode: "form", schema: schema), .form)
        let fields = ElicitationSchemaParser.fields(from: schema)
        XCTAssertEqual(fields.count, 1)
        XCTAssertEqual(fields[0].id, "animal")
        XCTAssertEqual(fields[0].enumValues, ["cat", "dog"])

        let presentation = ElicitationFormPresentationBuilder.build(fields: fields)
        XCTAssertEqual(
            presentation.cardAccessibilityLabel,
            ElicitationFormPresentationBuilder.cardAccessibilityLabel
        )
        XCTAssertEqual(presentation.fieldAccessibilityLabels, ["Animal"])
        XCTAssertTrue(presentation.submitUsesDefaultKeyboardShortcut)
    }

    func testElicitationFormFieldAccessibilityLabelsCoverRequiredFields() {
        let schema: JSONValue = .object([
            "type": .string("object"),
            "properties": .object([
                "name": .object([
                    "type": .string("string"),
                    "title": .string("Display name")
                ]),
                "count": .object([
                    "type": .string("integer"),
                    "title": .string("Count")
                ])
            ]),
            "required": .array([.string("name"), .string("count")])
        ])
        let fields = ElicitationSchemaParser.fields(from: schema)
        let presentation = ElicitationFormPresentationBuilder.build(fields: fields)
        XCTAssertEqual(presentation.fieldAccessibilityLabels, ["Count", "Display name"])
        XCTAssertEqual(presentation.cardAccessibilityLabel, "Elicitation requested")
    }

    func testURLHandoffConsentCardStateSnapshot() {
        let request = URLConsentRequest(
            requestId: "req-1",
            serverId: "remote",
            originToolCallId: "tool-1",
            message: "Sign in to continue",
            url: "https://auth.example.com/oauth?client_id=abc&state=secret"
        )
        let presentation = URLConsentPresentationBuilder.build(request: request)

        XCTAssertFalse(presentation.showsUnsafeWarning)
        XCTAssertEqual(presentation.destinationOrigin, "https://auth.example.com")
        XCTAssertTrue(presentation.openButtonEnabled)
        XCTAssertEqual(presentation.accessibilityLabel, "Browser handoff requested")
        XCTAssertFalse(
            URLHandoffPolicy.destination(for: request.url ?? "")?.displayURL.contains("state=") ?? true
        )
    }

    func testURLHandoffConsentCardRejectsUnsafeURLSnapshot() {
        let request = URLConsentRequest(
            requestId: "req-2",
            serverId: "remote",
            originToolCallId: nil,
            message: "Unsafe handoff",
            url: "http://example.com/oauth"
        )
        let presentation = URLConsentPresentationBuilder.build(request: request)

        XCTAssertTrue(presentation.showsUnsafeWarning)
        XCTAssertNil(presentation.destinationOrigin)
        XCTAssertFalse(presentation.openButtonEnabled)
    }

    func testOAuthStatusNeedsConnectionSnapshot() {
        let missing = GatewayOAuthPresentation.forStatus("missing")
        XCTAssertEqual(missing.iconSystemName, "key.slash")
        XCTAssertTrue(missing.showsConnectButton)
        XCTAssertEqual(missing.statusLabel, "missing")

        let notConfigured = GatewayOAuthPresentation.forStatus("not_configured")
        XCTAssertTrue(notConfigured.showsConnectButton)
        XCTAssertEqual(notConfigured.iconSystemName, "key")
    }

    func testOAuthStatusConnectedSnapshot() {
        let connected = GatewayOAuthPresentation.forStatus("connected")
        XCTAssertEqual(connected.iconSystemName, "checkmark.seal.fill")
        XCTAssertEqual(connected.iconTone, .connected)
        XCTAssertFalse(connected.showsConnectButton)
        XCTAssertEqual(connected.statusLabel, "connected")
    }

    func testOAuthStatusReauthRequiredSnapshot() {
        let reauth = GatewayOAuthPresentation.forStatus("reauth_required")
        XCTAssertEqual(reauth.iconSystemName, "exclamationmark.triangle.fill")
        XCTAssertEqual(reauth.iconTone, .warning)
        XCTAssertTrue(reauth.showsConnectButton)
        XCTAssertEqual(reauth.statusLabel, "reauth required")

        let expired = GatewayOAuthPresentation.forStatus("expired")
        XCTAssertEqual(expired.iconTone, .warning)
        XCTAssertTrue(expired.showsConnectButton)
    }

    func testElicitationSecretBlockedCardStateSnapshot() {
        let schema: JSONValue = .object([
            "type": .string("object"),
            "properties": .object([
                "token": .object([
                    "type": .string("string"),
                    "title": .string("Access token")
                ])
            ])
        ])
        XCTAssertEqual(ElicitationCardRouter.cardKind(mode: "form", schema: schema), .secretBlocked)
    }

    func testElicitationUnsupportedSchemaCardStateSnapshot() {
        let schema: JSONValue = .object([
            "type": .string("object"),
            "properties": .object([
                "address": .object([
                    "type": .string("object"),
                    "properties": .object([
                        "street": .object(["type": .string("string")])
                    ])
                ])
            ])
        ])
        XCTAssertEqual(ElicitationCardRouter.cardKind(mode: "form", schema: schema), .unsupportedSchema)
    }

    func testLiveEventGroupingNestsElicitationUnderToolCall() {
        let events = [
            IdentifiedCoreEvent(
                id: 0,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "tool_call_started",
                    payloadJSON: #"{"id":"tool-9","name":"elicit"}"#
                )
            ),
            IdentifiedCoreEvent(
                id: 1,
                event: CoreEvent(
                    conversationId: "c1",
                    kind: "elicitation_requested",
                    payloadJSON: #"{"request_id":"req-1","server_id":"mock","origin_tool_call_id":"tool-9","mode":"form","message":"What is your name?"}"#
                )
            ),
        ]

        let groups = LiveEventGrouping.build(from: events)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].nested.count, 1)
        XCTAssertEqual(groups[0].nested[0].kind, "elicitation_requested")
    }

    func testTranscriptContentGroupingNestsElicitationUnderToolCall() {
        let blocks = [
            TranscriptContentBlock(
                type: "tool_call",
                text: nil,
                name: "elicit",
                input: nil,
                callId: "tool-9",
                output: nil,
                path: nil,
                mimeType: nil,
                size: nil,
                sha256: nil,
                inline: nil,
                integrityFailed: nil,
                requestId: nil,
                serverId: nil,
                originToolCallId: nil,
                mode: nil,
                message: nil,
                schema: nil,
                url: nil,
                action: nil,
                data: nil,
                uri: nil,
                templateRef: nil,
                state: nil,
                taskId: nil,
                taskStatus: nil,
                taskTitle: nil,
                taskResultSummary: nil
            ),
            TranscriptContentBlock(
                type: "elicitation_request",
                text: nil,
                name: nil,
                input: nil,
                callId: nil,
                output: nil,
                path: nil,
                mimeType: nil,
                size: nil,
                sha256: nil,
                inline: nil,
                integrityFailed: nil,
                requestId: "req-1",
                serverId: "mock",
                originToolCallId: "tool-9",
                mode: "form",
                message: "What is your name?",
                schema: nil,
                url: nil,
                action: nil,
                data: nil,
                uri: nil,
                templateRef: nil,
                state: nil,
                taskId: nil,
                taskStatus: nil,
                taskTitle: nil,
                taskResultSummary: nil
            ),
        ]

        let groups = TranscriptContentGrouping.build(from: blocks)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].nested.count, 1)
        XCTAssertEqual(groups[0].nested[0].type, "elicitation_request")
    }
}
