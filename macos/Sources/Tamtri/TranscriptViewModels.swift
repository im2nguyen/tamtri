import Foundation

struct PermissionOptionPresentation: Equatable, Identifiable {
    let id: String
    let label: String
    let isReject: Bool
}

struct PermissionCardPresentation: Equatable {
    let requestId: String
    let harnessDisplayName: String?
    let command: String?
    let diff: DiffPayload?
    let summary: String
    let options: [PermissionOptionPresentation]
    let accessibilityLabel: String

    static let defaultAccessibilityLabel = "Permission requested"

    var allowOptions: [PermissionOptionPresentation] {
        options.filter { !$0.isReject }
    }

    var rejectOptions: [PermissionOptionPresentation] {
        options.filter(\.isReject)
    }
}

struct PermissionResolvedReceipt: Equatable {
    let summary: String
    let accessibilityLabel: String
}

enum PermissionResolvedReceiptBuilder {
    static func fromCommittedBlock(_ block: TranscriptContentBlock) -> PermissionResolvedReceipt? {
        guard block.type == "tool_result",
              let permission = block.output?.value(at: "permission"),
              case .object(let object) = permission,
              case .string(let status) = object["status"]
        else {
            return nil
        }
        if status == "resolved" {
            let optionID = object["selected_option"]?.stringValue ?? "unknown"
            let label = PermissionResolvedReceiptBuilder.friendlyLabel(for: optionID)
            return PermissionResolvedReceipt(
                summary: "Permission · \(label)",
                accessibilityLabel: "Permission \(label)"
            )
        }
        if status == "requested" {
            return PermissionResolvedReceipt(
                summary: "Permission · not resolved",
                accessibilityLabel: "Permission request expired when the turn ended"
            )
        }
        return nil
    }

    static func friendlyLabel(for optionID: String) -> String {
        switch optionID.lowercased() {
        case "deny", "reject":
            return "denied"
        case "allow_once":
            return "allowed once"
        case "allow_for_conversation":
            return "allowed for conversation"
        default:
            return optionID.replacingOccurrences(of: "_", with: " ")
        }
    }
}

enum PermissionCardPresentationBuilder {
    static func build(payloadJSON: String) -> PermissionCardPresentation? {
        guard let payload = try? JSONDecoder().decode(PermissionPayloadDTO.self, from: Data(payloadJSON.utf8))
        else {
            return nil
        }
        return presentation(from: payload)
    }

    static func fromCommittedPermissionBlock(_ block: TranscriptContentBlock) -> PermissionCardPresentation? {
        guard block.type == "tool_result",
              let permission = block.output?.value(at: "permission"),
              case .object(let object) = permission,
              case .string(let status) = object["status"],
              status == "requested"
        else {
            return nil
        }
        let action = object["action"]?.stringValue
        let options: [PermissionOptionDTO] = {
            guard case .array(let items)? = object["options"] else { return [] }
            return items.compactMap { item in
                guard case .object(let option) = item,
                      case .string(let id) = option["id"],
                      case .string(let label) = option["label"]
                else {
                    return nil
                }
                return PermissionOptionDTO(id: id, label: label)
            }
        }()
        let payload = PermissionPayloadDTO(
            requestId: block.callId ?? "",
            action: action ?? "",
            options: options,
            detail: nil,
            harnessDisplayName: nil
        )
        return presentation(from: payload)
    }

    private static func presentation(from payload: PermissionPayloadDTO) -> PermissionCardPresentation {
        PermissionCardPresentation(
            requestId: payload.requestId,
            harnessDisplayName: payload.harnessDisplayName,
            command: payload.detail?.command,
            diff: payload.detail?.diff,
            summary: payload.summary,
            options: payload.options.map {
                PermissionOptionPresentation(id: $0.id, label: $0.label, isReject: $0.isReject)
            },
            accessibilityLabel: PermissionCardPresentation.defaultAccessibilityLabel
        )
    }
}

struct ThinkingDisclosurePresentation: Equatable {
    let title: String
    let text: String
    let initiallyExpanded: Bool
    let startedAt: Date?
    let endedAt: Date?
    let isStreaming: Bool
    let estimatedDurationSeconds: Double?

    init(
        title: String,
        text: String,
        initiallyExpanded: Bool,
        startedAt: Date? = nil,
        endedAt: Date? = nil,
        isStreaming: Bool = false,
        estimatedDurationSeconds: Double? = nil
    ) {
        self.title = title
        self.text = text
        self.initiallyExpanded = initiallyExpanded
        self.startedAt = startedAt
        self.endedAt = endedAt
        self.isStreaming = isStreaming
        self.estimatedDurationSeconds = estimatedDurationSeconds
    }
}

enum ThinkingDisclosurePresentationBuilder {
    static func fromCommittedBlock(_ block: TranscriptContentBlock) -> ThinkingDisclosurePresentation? {
        guard block.type == "thinking" else { return nil }
        let text = block.text ?? ""
        return ThinkingDisclosurePresentation(
            title: "Thinking",
            text: text,
            initiallyExpanded: false,
            estimatedDurationSeconds: ThoughtDurationFormatting.estimatedFromText(text)
        )
    }

    static func fromLivePayloadJSON(_ payloadJSON: String) -> ThinkingDisclosurePresentation? {
        let text = LiveEventPayload.text(from: payloadJSON)
        guard !text.isEmpty else { return nil }
        return ThinkingDisclosurePresentation(
            title: "Thinking",
            text: text,
            initiallyExpanded: false,
            startedAt: Date(),
            isStreaming: true
        )
    }
}

struct ToolCardPresentation: Equatable {
    let title: String
    let subtitle: String
    let detail: String
    let diff: DiffPayload?
    let accessibilityLabel: String
}

enum ToolCardPresentationBuilder {
    static func fromCommittedToolGroup(
        call: TranscriptContentBlock,
        result: TranscriptContentBlock
    ) -> ToolCardPresentation? {
        guard call.type == "tool_call", result.type == "tool_result" else { return nil }
        return fromCommittedResultBlock(result, fallbackCall: call)
    }

    static func fromCommittedCallBlock(_ block: TranscriptContentBlock) -> ToolCardPresentation? {
        guard block.type == "tool_call", block.hasToolCallBody else { return nil }
        return ToolCardPresentation(
            title: block.name?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
                ? (block.name ?? "Tool call")
                : "Tool call",
            subtitle: block.inputSummary,
            detail: block.inputSummary,
            diff: nil,
            accessibilityLabel: "Tool call"
        )
    }

    static func fromCommittedResultBlock(
        _ block: TranscriptContentBlock,
        fallbackCall: TranscriptContentBlock? = nil
    ) -> ToolCardPresentation? {
        guard block.type == "tool_result" else { return nil }
        if block.output?.value(at: "permission") != nil {
            return nil
        }
        let diffs = block.toolDiffs
        if let diff = diffs.first {
            return ToolCardPresentation(
                title: toolResultTitle(for: block, fallbackCall: fallbackCall),
                subtitle: [diff.change, diff.path].compactMap { $0 }.joined(separator: " • "),
                detail: "",
                diff: diff,
                accessibilityLabel: "Tool result"
            )
        }
        let preview = block.toolResultPreviewText ?? block.outputSummary
        let status = block.output?.string(at: "status") ?? ""
        return ToolCardPresentation(
            title: toolResultTitle(for: block, fallbackCall: fallbackCall),
            subtitle: status,
            detail: preview,
            diff: nil,
            accessibilityLabel: "Tool result"
        )
    }

    private static func toolResultTitle(
        for block: TranscriptContentBlock,
        fallbackCall: TranscriptContentBlock?
    ) -> String {
        if block.toolDiffs.first != nil {
            return block.toolResultDisplayTitle ?? "Tool result"
        }
        let kind = block.toolResultDisplayTitle
            ?? fallbackCall?.name?.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedKind = kind?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let status = block.output?.string(at: "status")?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !trimmedKind.isEmpty, !status.isEmpty {
            return "\(trimmedKind) \(humanizedToolStatus(status))"
        }
        if !trimmedKind.isEmpty {
            return trimmedKind
        }
        return "Tool result"
    }

    private static func humanizedToolStatus(_ status: String) -> String {
        status.replacingOccurrences(of: "_", with: " ")
    }

    static func fromLiveEvent(kind: String, payloadJSON: String) -> ToolCardPresentation? {
        let json = JSONValue.from(json: payloadJSON)
        let name = json?.string(at: "name") ?? json?.string(at: "title")
        let path = json?.string(at: "path") ?? json?.value(at: "diff")?.string(at: "path")
        let status = json?.string(at: "status")
        let diff = ToolDiffParsing.diff(from: json)
        let detail: String
        if diff != nil {
            detail = ""
        } else if let json {
            detail = json.toolPayloadText ?? ""
        } else {
            detail = ""
        }
        let kindLabel = json?.toolPayloadKind?.capitalized
        let baseTitle = name?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            ? name!
            : (kindLabel ?? "")
        let title: String
        if !baseTitle.isEmpty, let status, !status.isEmpty,
           kind == "tool_call_progress" || kind == "tool_call_completed" {
            title = "\(baseTitle) \(humanizedToolStatus(status))"
        } else if !baseTitle.isEmpty {
            title = baseTitle
        } else {
            title = kind.replacingOccurrences(of: "_", with: " ").capitalized
        }
        let subtitle = [status, path]
            .compactMap { $0?.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
            .joined(separator: " • ")
        if title == "Tool call started" && subtitle.isEmpty && detail.isEmpty && diff == nil {
            return nil
        }
        return ToolCardPresentation(
            title: title,
            subtitle: subtitle,
            detail: detail,
            diff: diff,
            accessibilityLabel: "Tool event"
        )
    }
}

enum LiveEventPayload {
    static func text(from json: String) -> String {
        struct Payload: Decodable {
            let text: String?
        }
        return (try? JSONDecoder().decode(Payload.self, from: Data(json.utf8)).text) ?? ""
    }
}

private struct PermissionPayloadDTO: Decodable {
    let requestId: String
    let action: String
    let options: [PermissionOptionDTO]
    let detail: PermissionDetailDTO?
    let harnessDisplayName: String?

    enum CodingKeys: String, CodingKey {
        case requestId = "request_id"
        case action
        case options
        case detail
        case harnessDisplayName = "harness_display_name"
    }

    var summary: String {
        if let detailSummary = detail?.summary, !detailSummary.isEmpty {
            return detailSummary
        }
        return action.isEmpty ? "The agent is asking for permission." : "Action: \(action)"
    }
}

private struct PermissionDetailDTO: Decodable {
    let type: String?
    let command: String?
    let diff: DiffPayload?

    var summary: String {
        if let command {
            return command
        }
        if let diff {
            return diff.summary
        }
        return type ?? ""
    }
}

private struct PermissionOptionDTO: Decodable {
    let id: String
    let label: String

    var isReject: Bool {
        id.localizedCaseInsensitiveContains("deny") || id.localizedCaseInsensitiveContains("reject")
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
