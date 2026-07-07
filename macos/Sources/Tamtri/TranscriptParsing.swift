import Foundation

struct ParsedTranscriptMessage: Identifiable, Equatable {
    let id: String
    let role: String
    let harnessId: String?
    let content: [TranscriptContentBlock]
    let createdAt: String?
    let rawJSON: String?

    init(
        id: String,
        role: String,
        harnessId: String?,
        content: [TranscriptContentBlock],
        createdAt: String? = nil,
        rawJSON: String?
    ) {
        self.id = id
        self.role = role
        self.harnessId = harnessId
        self.content = content
        self.createdAt = createdAt
        self.rawJSON = rawJSON
    }

    init(json: String, fallbackIndex: Int) {
        if let decoded = try? JSONDecoder().decode(DecodedMessage.self, from: Data(json.utf8)) {
            id = decoded.id ?? "message-\(fallbackIndex)"
            role = decoded.role
            harnessId = decoded.harnessId
            content = decoded.content
            createdAt = decoded.createdAt
            rawJSON = nil
        } else {
            id = "message-\(fallbackIndex)"
            role = "unknown"
            harnessId = nil
            content = []
            createdAt = nil
            rawJSON = json
        }
    }

    static func pendingUserMessage(id: String, text: String) -> ParsedTranscriptMessage {
        let createdAt = ISO8601DateFormatter().string(from: Date())
        return ParsedTranscriptMessage(
            id: id,
            role: "user",
            harnessId: nil,
            content: [TranscriptContentBlock(type: "text", text: text)],
            createdAt: createdAt,
            rawJSON: nil
        )
    }
}

enum TranscriptMessageText {
    static func plainText(from message: ParsedTranscriptMessage) -> String {
        message.content
            .compactMap { block in
                guard block.type == "text", let text = block.text else { return nil }
                let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
                return trimmed.isEmpty ? nil : text
            }
            .joined(separator: "\n\n")
    }

    static func precedingUserMessage(
        before message: ParsedTranscriptMessage,
        in messages: [ParsedTranscriptMessage]
    ) -> ParsedTranscriptMessage? {
        guard let index = messages.firstIndex(where: { $0.id == message.id }), index > 0 else {
            return nil
        }
        for candidate in messages[..<index].reversed() where candidate.role == "user" {
            return candidate
        }
        return nil
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
    let integrityFailed: Bool?

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
        case id
        case output
        case path
        case mimeType = "mime_type"
        case size
        case sha256
        case inline
        case integrityFailed = "integrity_failed"
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

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        type = try container.decode(String.self, forKey: .type)
        text = try container.decodeIfPresent(String.self, forKey: .text)
        name = try container.decodeIfPresent(String.self, forKey: .name)
        input = try container.decodeIfPresent(JSONValue.self, forKey: .input)
        callId = try container.decodeIfPresent(String.self, forKey: .callId)
            ?? container.decodeIfPresent(String.self, forKey: .id)
        output = try container.decodeIfPresent(JSONValue.self, forKey: .output)
        path = try container.decodeIfPresent(String.self, forKey: .path)
        mimeType = try container.decodeIfPresent(String.self, forKey: .mimeType)
        size = try container.decodeIfPresent(UInt64.self, forKey: .size)
        sha256 = try container.decodeIfPresent(String.self, forKey: .sha256)
        inline = try container.decodeIfPresent(String.self, forKey: .inline)
        integrityFailed = try container.decodeIfPresent(Bool.self, forKey: .integrityFailed)
        requestId = try container.decodeIfPresent(String.self, forKey: .requestId)
        serverId = try container.decodeIfPresent(String.self, forKey: .serverId)
        originToolCallId = try container.decodeIfPresent(String.self, forKey: .originToolCallId)
        mode = try container.decodeIfPresent(String.self, forKey: .mode)
        message = try container.decodeIfPresent(String.self, forKey: .message)
        schema = try container.decodeIfPresent(JSONValue.self, forKey: .schema)
        url = try container.decodeIfPresent(String.self, forKey: .url)
        action = try container.decodeIfPresent(String.self, forKey: .action)
        data = try container.decodeIfPresent(JSONValue.self, forKey: .data)
        uri = try container.decodeIfPresent(String.self, forKey: .uri)
        templateRef = try container.decodeIfPresent(String.self, forKey: .templateRef)
        state = try container.decodeIfPresent(JSONValue.self, forKey: .state)
        taskId = try container.decodeIfPresent(String.self, forKey: .taskId)
        taskStatus = try container.decodeIfPresent(String.self, forKey: .taskStatus)
        taskTitle = try container.decodeIfPresent(String.self, forKey: .taskTitle)
        taskResultSummary = try container.decodeIfPresent(String.self, forKey: .taskResultSummary)
    }

    init(
        type: String,
        text: String? = nil,
        name: String? = nil,
        input: JSONValue? = nil,
        callId: String? = nil,
        output: JSONValue? = nil,
        path: String? = nil,
        mimeType: String? = nil,
        size: UInt64? = nil,
        sha256: String? = nil,
        inline: String? = nil,
        integrityFailed: Bool? = nil,
        requestId: String? = nil,
        serverId: String? = nil,
        originToolCallId: String? = nil,
        mode: String? = nil,
        message: String? = nil,
        schema: JSONValue? = nil,
        url: String? = nil,
        action: String? = nil,
        data: JSONValue? = nil,
        uri: String? = nil,
        templateRef: String? = nil,
        state: JSONValue? = nil,
        taskId: String? = nil,
        taskStatus: String? = nil,
        taskTitle: String? = nil,
        taskResultSummary: String? = nil
    ) {
        self.type = type
        self.text = text
        self.name = name
        self.input = input
        self.callId = callId
        self.output = output
        self.path = path
        self.mimeType = mimeType
        self.size = size
        self.sha256 = sha256
        self.inline = inline
        self.integrityFailed = integrityFailed
        self.requestId = requestId
        self.serverId = serverId
        self.originToolCallId = originToolCallId
        self.mode = mode
        self.message = message
        self.schema = schema
        self.url = url
        self.action = action
        self.data = data
        self.uri = uri
        self.templateRef = templateRef
        self.state = state
        self.taskId = taskId
        self.taskStatus = taskStatus
        self.taskTitle = taskTitle
        self.taskResultSummary = taskResultSummary
    }

    var hasToolCallBody: Bool {
        guard type == "tool_call" else { return true }
        let trimmedName = name?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !trimmedName.isEmpty {
            return true
        }
        guard let input else { return false }
        switch input {
        case .null:
            return false
        case .object(let object):
            return !object.isEmpty
        case .array(let values):
            return !values.isEmpty
        case .string(let value):
            return !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        case .number, .bool:
            return true
        }
    }

    var toolResultDisplayTitle: String? {
        guard type == "tool_result", let output else { return nil }
        if let kind = output.toolPayloadKind {
            return kind.capitalized
        }
        if let firstDiff = toolDiffs.first, let path = firstDiff.path {
            return (firstDiff.change ?? "Modified") + " · " + (path as NSString).lastPathComponent
        }
        return nil
    }

    var toolResultPreviewText: String? {
        guard type == "tool_result", let output else { return nil }
        if let text = output.toolPayloadText {
            return text
        }
        return nil
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
                createdAt: message.createdAt,
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
    let createdAt: String?

    enum CodingKeys: String, CodingKey {
        case id
        case role
        case harnessId = "harness_id"
        case content
        case createdAt = "created_at"
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

    /// ACP tool_result payloads often nest kind/title under content[].value.
    var toolPayloadKind: String? {
        if let kind = string(at: "kind") {
            return kind
        }
        guard case .object(let object) = self,
              case .array(let items)? = object["content"]
        else {
            return nil
        }
        for item in items {
            guard case .object(let entry) = item else { continue }
            if case .string(let kind) = entry["kind"] {
                return kind
            }
            if case .object(let value) = entry["value"], case .string(let kind) = value["kind"] {
                return kind
            }
        }
        return nil
    }

    var toolPayloadText: String? {
        if case .string(let value) = self {
            return value
        }
        guard case .object(let object) = self else { return nil }
        if let text = joinedExtractedText(from: object["content"]) {
            return text
        }
        return nil
    }

    fileprivate var extractedText: String? {
        switch self {
        case .string(let value):
            return value.isEmpty ? nil : value
        case .object(let object):
            if case .string(let text) = object["text"], !text.isEmpty {
                return text
            }
            if case .string(let text) = object["content"], !text.isEmpty {
                return text
            }
            if case .object(let content) = object["content"],
               case .string(let text) = content["text"],
               !text.isEmpty
            {
                return text
            }
            if case .object(let valueObject) = object["value"] {
                return JSONValue.object(valueObject).extractedText
            }
            return joinedExtractedText(from: object["content"])
        case .array(let items):
            return joinedExtractedText(from: .array(items))
        default:
            return nil
        }
    }

    fileprivate func joinedExtractedText(from value: JSONValue?) -> String? {
        guard let value else { return nil }
        let joined: String
        switch value {
        case .array(let items):
            joined = items.compactMap(\.extractedText).joined(separator: "\n")
        default:
            joined = value.extractedText ?? ""
        }
        let trimmed = joined.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : joined
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
