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
}
