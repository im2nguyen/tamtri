import Foundation

struct LiveEventGroupModel: Identifiable {
    let id: Int
    let toolEvent: CoreEvent?
    let toolCompletion: CoreEvent?
    let nested: [CoreEvent]
    let standalone: CoreEvent?

    init(
        id: Int,
        toolEvent: CoreEvent? = nil,
        toolCompletion: CoreEvent? = nil,
        nested: [CoreEvent] = [],
        standalone: CoreEvent? = nil
    ) {
        self.id = id
        self.toolEvent = toolEvent
        self.toolCompletion = toolCompletion
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

    private static func toolCallId(from event: CoreEvent) -> String? {
        payloadString(from: event.payloadJSON, key: "id")
            ?? payloadString(from: event.payloadJSON, key: "toolCallId")
    }

    private static func isToolCompletionEvent(_ kind: String) -> Bool {
        kind == "tool_call_progress" || kind == "tool_call_completed"
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
               let toolId = toolCallId(from: item.event) {
                var nested: [CoreEvent] = []
                var completion: CoreEvent?
                var next = index + 1
                if next < events.count {
                    let candidate = events[next].event
                    if isToolCompletionEvent(candidate.kind),
                       toolCallId(from: candidate) == toolId {
                        completion = candidate
                        next += 1
                    }
                }
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
                groups.append(
                    LiveEventGroupModel(
                        id: item.id,
                        toolEvent: item.event,
                        toolCompletion: completion,
                        nested: nested
                    )
                )
                index = next
                continue
            }
            groups.append(LiveEventGroupModel(id: item.id, standalone: item.event))
            index += 1
        }
        return groups
    }
}

enum LiveTranscriptSegment: Identifiable {
    case activity(ActivityClusterModel)
    case toolGroup(LiveEventGroupModel)
    case event(CoreEvent)

    var id: String {
        switch self {
        case .activity(let cluster): "live-activity-\(cluster.id)"
        case .toolGroup(let group): "live-tool-\(group.id)"
        case .event(let event): "live-event-\(event.kind)-\(event.payloadJSON.hashValue)"
        }
    }
}

enum StreamingTextMerge {
    /// Combines incremental and cumulative harness text deltas into one render string.
    static func merged(existing: String, delta: String) -> String {
        if delta.isEmpty {
            return existing
        }
        if existing.isEmpty {
            return delta
        }
        if delta.hasPrefix(existing) {
            return delta
        }
        if existing.hasPrefix(delta) {
            return existing
        }
        return existing + delta
    }
}

enum LiveActivityGrouping {
    private static let mutedKinds: Set<String> = [
        "thought_delta",
        "tool_call_started",
        "tool_call_progress",
        "tool_call_completed",
        "file_changed",
    ]

    static func build(from events: [IdentifiedCoreEvent]) -> [LiveTranscriptSegment] {
        let toolGroups = LiveEventGrouping.build(from: events)
        var segments: [LiveTranscriptSegment] = []
        var activityBuffer: [ActivityItemModel] = []
        var activityClusterId = 0
        var activityItemId = 0
        var textBuffer = ""
        var textConversationId: String?

        func flushActivity(markLastThoughtStreaming: Bool = false) {
            guard !activityBuffer.isEmpty else { return }
            var items = activityBuffer
            let streamingThoughtIndex: Int? = {
                guard markLastThoughtStreaming,
                      let lastIndex = items.indices.last,
                      case .thinking = items[lastIndex].kind else { return nil }
                return lastIndex
            }()
            let endedAt = Date()
            for index in items.indices {
                guard case .thinking(let presentation) = items[index].kind else { continue }
                if index == streamingThoughtIndex {
                    items[index] = ActivityItemModel(
                        id: items[index].id,
                        kind: .thinking(
                            ThinkingDisclosurePresentation(
                                title: presentation.title,
                                text: presentation.text,
                                initiallyExpanded: presentation.initiallyExpanded,
                                startedAt: presentation.startedAt ?? Date(),
                                endedAt: nil,
                                isStreaming: true,
                                estimatedDurationSeconds: presentation.estimatedDurationSeconds
                            )
                        )
                    )
                } else if presentation.startedAt != nil, presentation.endedAt == nil {
                    items[index] = ActivityItemModel(
                        id: items[index].id,
                        kind: .thinking(
                            ThinkingDisclosurePresentation(
                                title: presentation.title,
                                text: presentation.text,
                                initiallyExpanded: presentation.initiallyExpanded,
                                startedAt: presentation.startedAt,
                                endedAt: endedAt,
                                isStreaming: false,
                                estimatedDurationSeconds: presentation.estimatedDurationSeconds
                            )
                        )
                    )
                }
            }
            segments.append(.activity(ActivityClusterModel(id: activityClusterId, items: items)))
            activityClusterId += 1
            activityBuffer = []
            activityItemId = 0
        }

        func flushText() {
            guard !textBuffer.isEmpty, let conversationId = textConversationId else { return }
            flushActivity()
            let payload = (try? JSONSerialization.data(
                withJSONObject: ["text": textBuffer],
                options: [.sortedKeys]
            ))
                .flatMap { String(data: $0, encoding: .utf8) } ?? "{\"text\":\"\"}"
            segments.append(.event(CoreEvent(
                conversationId: conversationId,
                kind: "text_delta",
                payloadJSON: payload
            )))
            textBuffer = ""
            textConversationId = nil
        }

        func appendOrMergeThinking(_ thinking: ThinkingDisclosurePresentation) {
            if let lastIndex = activityBuffer.indices.last,
               case .thinking(let existing) = activityBuffer[lastIndex].kind {
                activityBuffer[lastIndex] = ActivityItemModel(
                    id: activityBuffer[lastIndex].id,
                    kind: .thinking(
                        ThinkingDisclosurePresentation(
                            title: existing.title,
                            text: existing.text + thinking.text,
                            initiallyExpanded: existing.initiallyExpanded,
                            startedAt: existing.startedAt ?? thinking.startedAt ?? Date(),
                            endedAt: nil,
                            isStreaming: false,
                            estimatedDurationSeconds: existing.estimatedDurationSeconds
                        )
                    )
                )
            } else {
                activityBuffer.append(
                    ActivityItemModel(
                        id: activityItemId,
                        kind: .thinking(
                            ThinkingDisclosurePresentation(
                                title: thinking.title,
                                text: thinking.text,
                                initiallyExpanded: thinking.initiallyExpanded,
                                startedAt: thinking.startedAt ?? Date(),
                                endedAt: nil,
                                isStreaming: false,
                                estimatedDurationSeconds: thinking.estimatedDurationSeconds
                            )
                        )
                    )
                )
                activityItemId += 1
            }
        }

        for group in toolGroups {
            if group.toolEvent != nil, !group.nested.isEmpty {
                flushText()
                flushActivity()
                segments.append(.toolGroup(group))
                continue
            }

            if let completion = group.toolCompletion,
               let presentation = ToolCardPresentationBuilder.fromLiveEvent(
                   kind: completion.kind,
                   payloadJSON: completion.payloadJSON
               ) {
                flushText()
                activityBuffer.append(ActivityItemModel(id: activityItemId, kind: .tool(presentation)))
                activityItemId += 1
                continue
            }

            if let toolEvent = group.toolEvent, mutedKinds.contains(toolEvent.kind) {
                if let presentation = ToolCardPresentationBuilder.fromLiveEvent(
                    kind: toolEvent.kind,
                    payloadJSON: toolEvent.payloadJSON
                ) {
                    flushText()
                    activityBuffer.append(ActivityItemModel(id: activityItemId, kind: .tool(presentation)))
                    activityItemId += 1
                }
                continue
            }

            if let standalone = group.standalone {
                if standalone.kind == "thought_delta",
                   let thinking = ThinkingDisclosurePresentationBuilder.fromLivePayloadJSON(standalone.payloadJSON) {
                    flushText()
                    appendOrMergeThinking(thinking)
                } else if standalone.kind == "text_delta" {
                    let delta = LiveEventPayload.text(from: standalone.payloadJSON)
                    textConversationId = standalone.conversationId
                    textBuffer = StreamingTextMerge.merged(existing: textBuffer, delta: delta)
                } else if mutedKinds.contains(standalone.kind),
                          let presentation = ToolCardPresentationBuilder.fromLiveEvent(
                              kind: standalone.kind,
                              payloadJSON: standalone.payloadJSON
                          ) {
                    flushText()
                    activityBuffer.append(ActivityItemModel(id: activityItemId, kind: .tool(presentation)))
                    activityItemId += 1
                } else {
                    flushText()
                    flushActivity()
                    segments.append(.event(standalone))
                }
            }
        }

        flushText()
        flushActivity(markLastThoughtStreaming: true)
        return segments
    }
}

struct IdentifiedCoreEvent: Identifiable {
    let id: Int
    let event: CoreEvent
}
