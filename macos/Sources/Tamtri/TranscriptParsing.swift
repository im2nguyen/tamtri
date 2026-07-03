import Foundation

struct ParsedTranscriptMessage: Identifiable, Equatable {
    let id: String
    let role: String
    let harnessId: String?
    let content: [TranscriptContentBlock]
    let rawJSON: String?

    init(
        id: String,
        role: String,
        harnessId: String?,
        content: [TranscriptContentBlock],
        rawJSON: String?
    ) {
        self.id = id
        self.role = role
        self.harnessId = harnessId
        self.content = content
        self.rawJSON = rawJSON
    }

    init(json: String, fallbackIndex: Int) {
        if let decoded = try? JSONDecoder().decode(DecodedMessage.self, from: Data(json.utf8)) {
            id = decoded.id ?? "message-\(fallbackIndex)"
            role = decoded.role
            harnessId = decoded.harnessId
            content = decoded.content
            rawJSON = nil
        } else {
            id = "message-\(fallbackIndex)"
            role = "unknown"
            harnessId = nil
            content = []
            rawJSON = json
        }
    }
}

struct TranscriptContentBlock: Decodable, Equatable {
    let type: String
    let text: String?
    let name: String?
    let input: JSONValue?
    let callId: String?
    let output: JSONValue?
    let path: String?
    let mimeType: String?
    let size: UInt64?
    let sha256: String?
    let inline: String?

    let requestId: String?
    let serverId: String?
    let originToolCallId: String?
    let mode: String?
    let message: String?
    let schema: JSONValue?
    let url: String?
    let action: String?
    let data: JSONValue?

    let uri: String?
    let templateRef: String?
    let state: JSONValue?

    let taskId: String?
    let taskStatus: String?
    let taskTitle: String?
    let taskResultSummary: String?

    enum CodingKeys: String, CodingKey {
        case type
        case text
        case name
        case input
        case callId = "call_id"
        case output
        case path
        case mimeType = "mime_type"
        case size
        case sha256
        case inline
        case requestId = "request_id"
        case serverId = "server_id"
        case originToolCallId = "origin_tool_call_id"
        case mode
        case message
        case schema
        case url
        case action
        case data
        case uri
        case templateRef = "template_ref"
        case state
        case taskId = "task_id"
        case taskStatus = "status"
        case taskTitle = "title"
        case taskResultSummary = "result_summary"
    }

    var inputSummary: String {
        input?.truncatedDescription ?? ""
    }

    var outputSummary: String {
        output?.truncatedDescription ?? ""
    }

    var toolDiffs: [DiffPayload] {
        ToolDiffParsing.diffs(from: output)
    }
}

enum ToolDiffParsing {
    static func diffs(from output: JSONValue?) -> [DiffPayload] {
        guard let output else { return [] }
        guard case .object(let object) = output,
              case .array(let items)? = object["content"]
        else {
            return []
        }
        return items.compactMap { item in
            guard case .object(let entry) = item,
                  case .string(let kind)? = entry["type"],
                  kind == "diff",
                  case .object(let diffObject) = entry["diff"]
            else {
                return nil
            }
            return DiffPayload(json: diffObject)
        }
    }

    static func diff(from eventPayload: JSONValue?) -> DiffPayload? {
        guard let eventPayload else { return nil }
        if case .object(let diffObject) = eventPayload.value(at: "diff") {
            return DiffPayload(json: diffObject)
        }
        return diffs(from: eventPayload).first
    }
}

struct DiffPayload: Equatable, Decodable {
    let path: String?
    let change: String?
    let oldText: String?
    let newText: String?

    init(path: String?, change: String?, oldText: String?, newText: String?) {
        self.path = path
        self.change = change
        self.oldText = oldText
        self.newText = newText
    }

    init(json: [String: JSONValue]) {
        path = json["path"]?.stringValue
        change = json["change"]?.stringValue
        oldText = json["old_text"]?.stringValue ?? json["oldText"]?.stringValue
        newText = json["new_text"]?.stringValue ?? json["newText"]?.stringValue
    }

    enum CodingKeys: String, CodingKey {
        case path
        case change
        case oldText = "old_text"
        case newText = "new_text"
    }

    var summary: String {
        var lines = [path, change].compactMap { $0 }
        if let newText, !newText.isEmpty {
            lines.append(newText)
        }
        return lines.joined(separator: "\n")
    }
}

private extension JSONValue {
    var stringValue: String? {
        if case .string(let value) = self {
            return value
        }
        return nil
    }
}

enum TranscriptParsing {
    static func parseTranscript(_ transcriptJSON: String) -> [ParsedTranscriptMessage] {
        guard let data = transcriptJSON.data(using: .utf8),
              let decoded = try? JSONDecoder().decode([DecodedMessage].self, from: data)
        else {
            return []
        }
        return decoded.enumerated().map { index, message in
            ParsedTranscriptMessage(
                id: message.id ?? "message-\(index)",
                role: message.role,
                harnessId: message.harnessId,
                content: message.content,
                rawJSON: nil
            )
        }
    }
}

private struct DecodedMessage: Decodable {
    let id: String?
    let role: String
    let harnessId: String?
    let content: [TranscriptContentBlock]

    enum CodingKeys: String, CodingKey {
        case id
        case role
        case harnessId = "harness_id"
        case content
    }
}

enum JSONValue: Decodable, Equatable, CustomStringConvertible {
    case string(String)
    case number(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([JSONValue].self) {
            self = .array(value)
        } else {
            self = .object(try container.decode([String: JSONValue].self))
        }
    }

    var description: String {
        switch self {
        case .string(let value):
            value
        case .number(let value):
            value.rounded() == value ? String(Int(value)) : String(value)
        case .bool(let value):
            String(value)
        case .null:
            "null"
        case .array(let values):
            values.map(\.description).joined(separator: "\n")
        case .object(let object):
            object
                .sorted { $0.key < $1.key }
                .map { "\($0.key): \($0.value.description)" }
                .joined(separator: "\n")
        }
    }

    var truncatedDescription: String {
        let full = description
        if full.count <= 600 {
            return full
        }
        return String(full.prefix(600)) + "…"
    }

    static func from(json: String) -> JSONValue? {
        try? JSONDecoder().decode(JSONValue.self, from: Data(json.utf8))
    }

    func value(at key: String) -> JSONValue? {
        if case .object(let object) = self {
            return object[key]
        }
        return nil
    }

    func string(at key: String) -> String? {
        guard case .string(let value) = value(at: key) else {
            return nil
        }
        return value
    }

    func toJSONString() -> String? {
        guard let data = try? JSONEncoder().encode(EncodableJSONValue(self)) else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }
}

private struct EncodableJSONValue: Encodable {
    let value: JSONValue

    init(_ value: JSONValue) {
        self.value = value
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch value {
        case .string(let v):
            try container.encode(v)
        case .number(let v):
            try container.encode(v)
        case .bool(let v):
            try container.encode(v)
        case .null:
            try container.encodeNil()
        case .array(let values):
            try container.encode(values.map(EncodableJSONValue.init))
        case .object(let object):
            try container.encode(object.mapValues(EncodableJSONValue.init))
        }
    }
}
