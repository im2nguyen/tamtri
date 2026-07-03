import Foundation

struct PermissionCardPresentation: Equatable {
    let harnessDisplayName: String?
    let command: String?
    let diffPath: String?
    let diffNewText: String?
    let summary: String
    let allowOptionLabels: [String]
    let rejectOptionLabels: [String]
    let accessibilityLabel: String

    static let defaultAccessibilityLabel = "Permission requested"
}

enum PermissionCardPresentationBuilder {
    static func build(payloadJSON: String) -> PermissionCardPresentation? {
        guard let payload = try? JSONDecoder().decode(PermissionPayloadDTO.self, from: Data(payloadJSON.utf8))
        else {
            return nil
        }
        let allow = payload.options.filter { !$0.isReject }.map(\.label)
        let reject = payload.options.filter(\.isReject).map(\.label)
        return PermissionCardPresentation(
            harnessDisplayName: payload.harnessDisplayName,
            command: payload.detail?.command,
            diffPath: payload.detail?.diff?.path,
            diffNewText: payload.detail?.diff?.newText,
            summary: payload.summary,
            allowOptionLabels: allow,
            rejectOptionLabels: reject,
            accessibilityLabel: PermissionCardPresentation.defaultAccessibilityLabel
        )
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
        return PermissionCardPresentation(
            harnessDisplayName: nil,
            command: nil,
            diffPath: nil,
            diffNewText: nil,
            summary: payload.summary,
            allowOptionLabels: options.filter { !$0.isReject }.map(\.label),
            rejectOptionLabels: options.filter(\.isReject).map(\.label),
            accessibilityLabel: PermissionCardPresentation.defaultAccessibilityLabel
        )
    }
}

struct ThinkingDisclosurePresentation: Equatable {
    let title: String
    let text: String
    let initiallyExpanded: Bool
}

enum ThinkingDisclosurePresentationBuilder {
    static func fromCommittedBlock(_ block: TranscriptContentBlock) -> ThinkingDisclosurePresentation? {
        guard block.type == "thinking" else { return nil }
        return ThinkingDisclosurePresentation(
            title: "Thinking",
            text: block.text ?? "",
            initiallyExpanded: false
        )
    }

    static func fromLivePayloadJSON(_ payloadJSON: String) -> ThinkingDisclosurePresentation? {
        let text = EventPayload.text(from: payloadJSON)
        guard !text.isEmpty else { return nil }
        return ThinkingDisclosurePresentation(
            title: "Thinking",
            text: text,
            initiallyExpanded: false
        )
    }
}

struct ToolCardPresentation: Equatable {
    let title: String
    let subtitle: String
    let detail: String
    let diffPath: String?
    let accessibilityLabel: String
}

enum ToolCardPresentationBuilder {
    static func fromCommittedCallBlock(_ block: TranscriptContentBlock) -> ToolCardPresentation? {
        guard block.type == "tool_call" else { return nil }
        return ToolCardPresentation(
            title: block.name ?? "Tool call",
            subtitle: block.inputSummary,
            detail: block.inputSummary,
            diffPath: nil,
            accessibilityLabel: "Tool call"
        )
    }

    static func fromCommittedResultBlock(_ block: TranscriptContentBlock) -> ToolCardPresentation? {
        guard block.type == "tool_result" else { return nil }
        if block.output?.value(at: "permission") != nil {
            return nil
        }
        let diffs = block.toolDiffs
        if let diff = diffs.first {
            return ToolCardPresentation(
                title: "Tool result",
                subtitle: [diff.change, diff.path].compactMap { $0 }.joined(separator: " • "),
                detail: "",
                diffPath: diff.path,
                accessibilityLabel: "Tool result"
            )
        }
        return ToolCardPresentation(
            title: "Tool result",
            subtitle: "",
            detail: block.outputSummary,
            diffPath: nil,
            accessibilityLabel: "Tool result"
        )
    }

    static func fromLiveEvent(kind: String, payloadJSON: String) -> ToolCardPresentation {
        let json = JSONValue.from(json: payloadJSON)
        let name = json?.string(at: "name") ?? json?.string(at: "title")
        let path = json?.string(at: "path") ?? json?.value(at: "diff")?.string(at: "path")
        let status = json?.string(at: "status")
        let title = name ?? kind.replacingOccurrences(of: "_", with: " ").capitalized
        let subtitle = [status, path].compactMap { $0 }.joined(separator: " • ")
        let diffPath = ToolDiffParsing.diff(from: json)?.path
        let detail: String
        if diffPath != nil {
            detail = ""
        } else if let json {
            detail = json.description
        } else {
            detail = payloadJSON
        }
        return ToolCardPresentation(
            title: title,
            subtitle: subtitle,
            detail: detail,
            diffPath: diffPath,
            accessibilityLabel: "Tool event"
        )
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

private struct EventPayload: Decodable {
    let text: String?

    static func text(from json: String) -> String {
        (try? JSONDecoder().decode(EventPayload.self, from: Data(json.utf8)).text) ?? ""
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
