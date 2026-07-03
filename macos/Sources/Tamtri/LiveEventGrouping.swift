import Foundation

struct LiveEventGroupModel: Identifiable {
    let id: Int
    let toolEvent: CoreEvent?
    let nested: [CoreEvent]
    let standalone: CoreEvent?

    init(id: Int, toolEvent: CoreEvent? = nil, nested: [CoreEvent] = [], standalone: CoreEvent? = nil) {
        self.id = id
        self.toolEvent = toolEvent
        self.nested = nested
        self.standalone = standalone
    }
}

enum LiveEventGrouping {
    private static let nestableKinds: Set<String> = [
        "elicitation_requested",
        "app_returned",
        "app_bridge_consent_requested",
        "file_changed",
        "task_started",
        "task_updated",
        "task_completed",
    ]

    private static func payloadString(from json: String, key: String) -> String? {
        guard let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return nil
        }
        return object[key] as? String
    }

    private static func originToolCallId(from event: CoreEvent) -> String? {
        if let id = payloadString(from: event.payloadJSON, key: "origin_tool_call_id") {
            return id
        }
        if let id = payloadString(from: event.payloadJSON, key: "originToolCallId") {
            return id
        }
        guard let data = event.payloadJSON.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let state = object["state"] as? [String: Any]
        else {
            return nil
        }
        if let id = state["origin_tool_call_id"] as? String {
            return id
        }
        return state["originToolCallId"] as? String
    }

    static func build(from events: [IdentifiedCoreEvent]) -> [LiveEventGroupModel] {
        var groups: [LiveEventGroupModel] = []
        var index = 0
        while index < events.count {
            let item = events[index]
            if item.event.kind == "tool_call_started",
               let toolId = payloadString(from: item.event.payloadJSON, key: "id") {
                var nested: [CoreEvent] = []
                var next = index + 1
                while next < events.count {
                    let candidate = events[next].event
                    guard nestableKinds.contains(candidate.kind),
                          originToolCallId(from: candidate) == toolId
                    else {
                        break
                    }
                    nested.append(candidate)
                    next += 1
                }
                groups.append(LiveEventGroupModel(id: item.id, toolEvent: item.event, nested: nested))
                index = next
                continue
            }
            groups.append(LiveEventGroupModel(id: item.id, standalone: item.event))
            index += 1
        }
        return groups
    }
}

struct IdentifiedCoreEvent: Identifiable {
    let id: Int
    let event: CoreEvent
}
