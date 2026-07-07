import Foundation

struct ActivityItemModel: Identifiable, Equatable {
    let id: Int
    let kind: ActivityItemKind
}

enum ActivityItemKind: Equatable {
    case thinking(ThinkingDisclosurePresentation)
    case tool(ToolCardPresentation)
}

struct ActivityClusterModel: Identifiable, Equatable {
    let id: Int
    let items: [ActivityItemModel]
}

enum MessageDisplaySegment: Identifiable {
    case text(TranscriptContentBlock)
    case rich(TranscriptContentBlock)
    case toolGroup(TranscriptBlockGroup)
    case activityCluster(ActivityClusterModel)

    var id: String {
        switch self {
        case .text(let block), .rich(let block):
            "block-\(block.type)-\(block.callId ?? block.text?.prefix(8).description ?? "")"
        case .toolGroup(let group):
            "tool-group-\(group.id)"
        case .activityCluster(let cluster):
            "activity-\(cluster.id)"
        }
    }
}

enum ActivityContentGrouping {
    static func messageSegments(from blocks: [TranscriptContentBlock]) -> [MessageDisplaySegment] {
        let groups = TranscriptContentGrouping.build(from: blocks)
        var segments: [MessageDisplaySegment] = []
        var activityBuffer: [ActivityItemModel] = []
        var activityClusterId = 0
        var activityItemId = 0

        func flushActivity() {
            guard !activityBuffer.isEmpty else { return }
            segments.append(.activityCluster(ActivityClusterModel(id: activityClusterId, items: activityBuffer)))
            activityClusterId += 1
            activityBuffer = []
            activityItemId = 0
        }

        func appendActivity(_ item: ActivityItemModel) {
            activityBuffer.append(item)
            activityItemId += 1
        }

        for group in groups {
            if let toolBlock = group.toolBlock, !group.nested.isEmpty {
                let toolResults = group.nested.filter { $0.type == "tool_result" }
                let richNested = group.nested.filter { $0.type != "tool_result" }

                for result in toolResults {
                    if PermissionCardPresentationBuilder.fromCommittedPermissionBlock(result) != nil
                        || PermissionResolvedReceiptBuilder.fromCommittedBlock(result) != nil {
                        flushActivity()
                        segments.append(.rich(result))
                    } else if let presentation = ToolCardPresentationBuilder.fromCommittedToolGroup(
                        call: toolBlock,
                        result: result
                    ) {
                        appendActivity(ActivityItemModel(id: activityItemId, kind: .tool(presentation)))
                    }
                }

                if !richNested.isEmpty {
                    flushActivity()
                    segments.append(
                        .toolGroup(
                            TranscriptBlockGroup(
                                id: group.id,
                                toolBlock: toolBlock,
                                nested: richNested
                            )
                        )
                    )
                } else if toolResults.isEmpty,
                          let presentation = ToolCardPresentationBuilder.fromCommittedCallBlock(toolBlock) {
                    appendActivity(ActivityItemModel(id: activityItemId, kind: .tool(presentation)))
                }
                continue
            }

            if let toolBlock = group.toolBlock,
               let presentation = ToolCardPresentationBuilder.fromCommittedCallBlock(toolBlock) {
                appendActivity(ActivityItemModel(id: activityItemId, kind: .tool(presentation)))
                continue
            }

            guard let standalone = group.standalone else { continue }

            if let thinking = ThinkingDisclosurePresentationBuilder.fromCommittedBlock(standalone) {
                appendActivity(ActivityItemModel(id: activityItemId, kind: .thinking(thinking)))
            } else if standalone.type == "tool_result",
                      PermissionCardPresentationBuilder.fromCommittedPermissionBlock(standalone) == nil,
                      PermissionResolvedReceiptBuilder.fromCommittedBlock(standalone) == nil,
                      let presentation = ToolCardPresentationBuilder.fromCommittedResultBlock(standalone) {
                appendActivity(ActivityItemModel(id: activityItemId, kind: .tool(presentation)))
            } else if standalone.type == "text" {
                flushActivity()
                segments.append(.text(standalone))
            } else {
                flushActivity()
                segments.append(.rich(standalone))
            }
        }

        flushActivity()
        return segments
    }
}

enum ThoughtDurationFormatting {
    static func format(seconds: TimeInterval) -> String {
        let total = max(1, Int(seconds.rounded()))
        if total < 60 {
            return "\(total)s"
        }
        let minutes = total / 60
        let remainder = total % 60
        if remainder == 0 {
            return "\(minutes)m"
        }
        return "\(minutes)m \(remainder)s"
    }

    static func estimatedFromText(_ text: String) -> TimeInterval {
        let count = text.trimmingCharacters(in: .whitespacesAndNewlines).count
        return max(1, Double(count) / 60.0)
    }

    static func duration(for presentation: ThinkingDisclosurePresentation, at now: Date = Date()) -> TimeInterval {
        if presentation.isStreaming, let startedAt = presentation.startedAt {
            return max(1, now.timeIntervalSince(startedAt))
        }
        if let startedAt = presentation.startedAt, let endedAt = presentation.endedAt {
            return max(1, endedAt.timeIntervalSince(startedAt))
        }
        if let estimated = presentation.estimatedDurationSeconds {
            return max(1, estimated)
        }
        return estimatedFromText(presentation.text)
    }

    static func thoughtHeaderLabel(for presentation: ThinkingDisclosurePresentation, at now: Date = Date()) -> String {
        "Thought for \(format(seconds: duration(for: presentation, at: now)))"
    }
}

enum ActivitySummaryBuilder {
    static func summarize(items: [ActivityItemModel]) -> String {
        var thoughts = 0
        var reads = 0
        var writes = 0
        var searches = 0
        var executes = 0
        var other = 0

        for item in items {
            switch item.kind {
            case .thinking:
                thoughts += 1
            case .tool(let presentation):
                let lower = presentation.title.lowercased()
                if lower.contains("read") {
                    reads += 1
                } else if lower.contains("write") || lower.contains("edit") {
                    writes += 1
                } else if lower.contains("search") || lower.contains("grep") {
                    searches += 1
                } else if lower.contains("execute") || lower.contains("bash") || lower.contains("shell") || lower.contains("command") {
                    executes += 1
                } else {
                    other += 1
                }
            }
        }

        var parts: [String] = []
        if thoughts > 0 { parts.append(thoughts == 1 ? "1 thought" : "\(thoughts) thoughts") }
        if reads > 0 { parts.append(reads == 1 ? "1 read" : "\(reads) reads") }
        if writes > 0 { parts.append(writes == 1 ? "1 edit" : "\(writes) edits") }
        if searches > 0 { parts.append(searches == 1 ? "1 search" : "\(searches) searches") }
        if executes > 0 { parts.append(executes == 1 ? "1 command" : "\(executes) commands") }
        if other > 0 { parts.append(other == 1 ? "1 tool" : "\(other) tools") }
        return parts.joined(separator: ", ")
    }

    static func lineLabel(for item: ActivityItemModel) -> String {
        switch item.kind {
        case .thinking(let presentation):
            return ThoughtDurationFormatting.thoughtHeaderLabel(for: presentation)
        case .tool(let presentation):
            let title = presentation.title.trimmingCharacters(in: .whitespacesAndNewlines)
            let subtitle = presentation.subtitle.trimmingCharacters(in: .whitespacesAndNewlines)
            let statusWords = Set(["started", "completed", "failed", "pending", "in_progress", "in progress"])
            if statusWords.contains(subtitle.lowercased()) {
                return title
            }
            if subtitle.isEmpty { return title }
            if subtitle.contains("•") || subtitle.contains("/") || subtitle.contains(".") {
                return subtitle.count > title.count ? subtitle : "\(title) · \(subtitle)"
            }
            return "\(title) · \(subtitle)"
        }
    }

    static func detailText(for item: ActivityItemModel) -> String? {
        switch item.kind {
        case .thinking(let presentation):
            let text = presentation.text.trimmingCharacters(in: .whitespacesAndNewlines)
            return text.isEmpty ? nil : text
        case .tool(let presentation):
            if presentation.diff != nil { return nil }
            let detail = presentation.detail.trimmingCharacters(in: .whitespacesAndNewlines)
            return detail.isEmpty ? nil : detail
        }
    }

    static func systemImage(for item: ActivityItemModel) -> String {
        switch item.kind {
        case .thinking:
            return "brain.head.profile"
        case .tool(let presentation):
            let lower = presentation.title.lowercased()
            if lower.contains("read") { return "doc.text" }
            if lower.contains("write") || lower.contains("edit") { return "pencil" }
            if lower.contains("search") || lower.contains("grep") { return "magnifyingglass" }
            if lower.contains("execute") || lower.contains("bash") || lower.contains("shell") || lower.contains("command") {
                return "terminal"
            }
            return "wrench.and.screwdriver"
        }
    }
}
