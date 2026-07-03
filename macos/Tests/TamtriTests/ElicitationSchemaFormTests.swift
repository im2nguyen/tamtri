import XCTest
@testable import Tamtri

final class ElicitationSchemaFormTests: XCTestCase {
    func testBuildPayloadRejectsStringShorterThanMinLength() {
        let field = ElicitationSchemaField(
            id: "name",
            title: "Name",
            description: nil,
            type: "string",
            itemType: nil,
            required: true,
            enumValues: [],
            minLength: 3,
            maxLength: nil,
            minimum: nil,
            maximum: nil
        )
        let result = ElicitationSchemaFormBuilder.buildPayload(
            fields: [field],
            values: ["name": "ab"],
            booleans: [:]
        )
        XCTAssertEqual(result?.error, "Name must be at least 3 characters.")
        XCTAssertTrue(result?.payload.isEmpty == true)
    }

    func testBuildPayloadRejectsStringLongerThanMaxLength() {
        let field = ElicitationSchemaField(
            id: "name",
            title: "Name",
            description: nil,
            type: "string",
            itemType: nil,
            required: true,
            enumValues: [],
            minLength: nil,
            maxLength: 5,
            minimum: nil,
            maximum: nil
        )
        let result = ElicitationSchemaFormBuilder.buildPayload(
            fields: [field],
            values: ["name": "toolong"],
            booleans: [:]
        )
        XCTAssertEqual(result?.error, "Name must be at most 5 characters.")
        XCTAssertTrue(result?.payload.isEmpty == true)
    }

    func testBuildPayloadAcceptsStringWithinLengthBounds() {
        let field = ElicitationSchemaField(
            id: "name",
            title: "Name",
            description: nil,
            type: "string",
            itemType: nil,
            required: true,
            enumValues: [],
            minLength: 2,
            maxLength: 5,
            minimum: nil,
            maximum: nil
        )
        let result = ElicitationSchemaFormBuilder.buildPayload(
            fields: [field],
            values: ["name": "tam"],
            booleans: [:]
        )
        XCTAssertNil(result?.error)
        XCTAssertEqual(result?.payload["name"] as? String, "tam")
    }

    func testBuildPayloadRejectsIntegerBelowMinimum() {
        let field = ElicitationSchemaField(
            id: "count",
            title: "Count",
            description: nil,
            type: "integer",
            itemType: nil,
            required: true,
            enumValues: [],
            minLength: nil,
            maxLength: nil,
            minimum: 3,
            maximum: 10
        )
        let result = ElicitationSchemaFormBuilder.buildPayload(
            fields: [field],
            values: ["count": "2"],
            booleans: [:]
        )
        XCTAssertEqual(result?.error, "Count must be at least 3.")
    }

    func testBuildPayloadRejectsNumberAboveMaximum() {
        let field = ElicitationSchemaField(
            id: "score",
            title: "Score",
            description: nil,
            type: "number",
            itemType: nil,
            required: true,
            enumValues: [],
            minLength: nil,
            maxLength: nil,
            minimum: 0,
            maximum: 1.5
        )
        let result = ElicitationSchemaFormBuilder.buildPayload(
            fields: [field],
            values: ["score": "2.0"],
            booleans: [:]
        )
        XCTAssertEqual(result?.error, "Score must be at most 1.5.")
    }

    func testBuildPayloadAcceptsEnumSelection() {
        let field = ElicitationSchemaField(
            id: "color",
            title: "Color",
            description: nil,
            type: "string",
            itemType: nil,
            required: true,
            enumValues: ["red", "green", "blue"],
            minLength: nil,
            maxLength: nil,
            minimum: nil,
            maximum: nil
        )
        let result = ElicitationSchemaFormBuilder.buildPayload(
            fields: [field],
            values: ["color": "green"],
            booleans: [:]
        )
        XCTAssertNil(result?.error)
        XCTAssertEqual(result?.payload["color"] as? String, "green")
    }

    func testBuildPayloadRejectsMissingRequiredField() {
        let field = ElicitationSchemaField(
            id: "name",
            title: "Name",
            description: nil,
            type: "string",
            itemType: nil,
            required: true,
            enumValues: [],
            minLength: nil,
            maxLength: nil,
            minimum: nil,
            maximum: nil
        )
        let result = ElicitationSchemaFormBuilder.buildPayload(
            fields: [field],
            values: [:],
            booleans: [:]
        )
        XCTAssertNil(result)
    }

    func testURLHandoffPolicyRejectsPlainHTTPNonLoopback() {
        XCTAssertNil(URLHandoffPolicy.destination(for: "http://example.com/oauth"))
    }

    func testURLHandoffPolicyAcceptsHTTPS() {
        let destination = URLHandoffPolicy.destination(for: "https://example.com/oauth?state=abc")
        XCTAssertNotNil(destination)
        XCTAssertEqual(destination?.origin, "https://example.com")
        XCTAssertFalse(destination?.displayURL.contains("state=") ?? true)
    }

    func testURLHandoffPolicyRejectsUserinfo() {
        XCTAssertNil(URLHandoffPolicy.destination(for: "https://user:pass@example.com/oauth"))
    }

    func testSchemaLooksSecretBlocksApiKeyField() {
        let schema: JSONValue = .object([
            "type": .string("object"),
            "properties": .object([
                "api_key": .object([
                    "type": .string("string"),
                    "title": .string("API key")
                ])
            ])
        ])
        XCTAssertTrue(ElicitationSchemaPolicy.schemaLooksSecret(schema))
    }

    func testSchemaLooksSecretAllowsBenignForm() {
        let schema: JSONValue = .object([
            "type": .string("object"),
            "properties": .object([
                "name": .object([
                    "type": .string("string"),
                    "title": .string("Name")
                ])
            ])
        ])
        XCTAssertFalse(ElicitationSchemaPolicy.schemaLooksSecret(schema))
    }
}
