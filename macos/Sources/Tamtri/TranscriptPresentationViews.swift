import SwiftUI

struct ThoughtDisclosureLabel: View {
    let presentation: ThinkingDisclosurePresentation

    var body: some View {
        if presentation.isStreaming, presentation.startedAt != nil {
            TimelineView(.periodic(from: .now, by: 1.0)) { context in
                labelContent(at: context.date)
            }
        } else {
            labelContent(at: Date())
        }
    }

    private func labelContent(at now: Date) -> some View {
        let duration = ThoughtDurationFormatting.format(
            seconds: ThoughtDurationFormatting.duration(for: presentation, at: now)
        )
        return HStack(spacing: 0) {
            Text("Thought")
                .foregroundStyle(TamtriTheme.activityPrimaryLabel())
            Text(" for \(duration)")
                .foregroundStyle(TamtriTheme.activityMutedDuration())
        }
        .font(TamtriTheme.mutedActionFont())
        .lineLimit(1)
    }
}

struct ThinkingDisclosureView: View {
    let presentation: ThinkingDisclosurePresentation
    var isExpanded: Binding<Bool>?
    @State private var localExpanded: Bool

    init(
        presentation: ThinkingDisclosurePresentation,
        isExpanded: Binding<Bool>? = nil
    ) {
        self.presentation = presentation
        self.isExpanded = isExpanded
        _localExpanded = State(initialValue: presentation.initiallyExpanded)
    }

    private var expansion: Binding<Bool> {
        isExpanded ?? $localExpanded
    }

    private var accessibilityTitle: String {
        ThoughtDurationFormatting.thoughtHeaderLabel(for: presentation)
    }

    var body: some View {
        DisclosureGroup(isExpanded: expansion) {
            TranscriptMarkdownText(content: presentation.text)
                .padding(.top, TamtriSpacing.xs)
        } label: {
            ThoughtDisclosureLabel(presentation: presentation)
        }
        .disclosureGroupStyle(MutedActionDisclosureStyle())
        .accessibilityLabel(accessibilityTitle)
    }
}

struct ToolCardView: View {
    let presentation: ToolCardPresentation
    @State private var isExpanded = false

    var body: some View {
        if presentation.title == "Tool call" && presentation.subtitle.isEmpty && presentation.detail.isEmpty && presentation.diff == nil {
            EmptyView()
        } else if hasExpandableContent {
            DisclosureGroup(isExpanded: $isExpanded) {
                expandedContent
                    .padding(.top, TamtriSpacing.xs)
            } label: {
                MutedActionLabel(systemImage: toolIconName, title: summaryLine, trailing: statusLabel)
            }
            .disclosureGroupStyle(MutedActionDisclosureStyle())
            .accessibilityLabel(presentation.accessibilityLabel)
        } else {
            MutedActionLabel(systemImage: toolIconName, title: summaryLine, trailing: statusLabel)
                .accessibilityLabel(presentation.accessibilityLabel)
        }
    }

    private var hasExpandableContent: Bool {
        presentation.diff != nil || !presentation.detail.isEmpty
    }

    @ViewBuilder
    private var expandedContent: some View {
        if let diff = presentation.diff {
            FileDiffView(diff: diff)
        } else if !presentation.detail.isEmpty {
            ToolOutputText(content: presentation.detail)
                .padding(TamtriSpacing.sm)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.primary.opacity(0.04), in: RoundedRectangle(cornerRadius: 6))
        }
    }

    private var summaryLine: String {
        let title = presentation.title.trimmingCharacters(in: .whitespacesAndNewlines)
        let subtitle = presentation.subtitle.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !subtitle.isEmpty else { return title }
        let statusWords = Set(["started", "completed", "failed", "pending", "in_progress", "in progress"])
        if statusWords.contains(subtitle.lowercased()) {
            return title
        }
        if subtitle.contains("•") || subtitle.contains("/") || subtitle.contains(".") {
            return subtitle.count > title.count ? subtitle : "\(title) · \(subtitle)"
        }
        return "\(title) · \(subtitle)"
    }

    private var statusLabel: String? {
        let subtitle = presentation.subtitle.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let statusWords = ["started", "completed", "failed", "pending", "in_progress", "in progress"]
        guard statusWords.contains(subtitle) else { return nil }
        return subtitle.replacingOccurrences(of: "_", with: " ")
    }

    private var toolIconName: String {
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

struct MutedActionLabel: View {
    let systemImage: String
    let title: String
    var trailing: String?

    var body: some View {
        HStack(spacing: TamtriSpacing.sm) {
            Image(systemName: systemImage)
                .font(.caption2)
                .foregroundStyle(.tertiary)
                .frame(width: 14)
            Text(title)
                .font(TamtriTheme.mutedActionFont())
                .foregroundStyle(.secondary)
                .lineLimit(1)
            if let trailing {
                Text(trailing)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
    }
}

struct MutedActionDisclosureStyle: DisclosureGroupStyle {
    var compact: Bool = false

    func makeBody(configuration: Configuration) -> some View {
        MutedActionDisclosureBody(configuration: configuration, compact: compact)
    }
}

private struct MutedActionDisclosureBody: View {
    let configuration: DisclosureGroupStyleConfiguration
    var compact: Bool
    @State private var isHovered = false

    var body: some View {
        VStack(alignment: .leading, spacing: compact ? 0 : TamtriSpacing.xs) {
            Button {
                withAnimation(.easeInOut(duration: 0.15)) {
                    configuration.isExpanded.toggle()
                }
            } label: {
                HStack(spacing: TamtriSpacing.xs) {
                    configuration.label
                    Image(systemName: "chevron.right")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(.tertiary)
                        .rotationEffect(.degrees(configuration.isExpanded ? 90 : 0))
                        .opacity(isHovered || configuration.isExpanded ? 1 : 0)
                        .animation(.easeInOut(duration: 0.12), value: isHovered)
                        .animation(.easeInOut(duration: 0.12), value: configuration.isExpanded)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.tamtriPlain)
            .onHover { isHovered = $0 }
            if configuration.isExpanded {
                configuration.content
            }
        }
        .padding(.vertical, compact ? 1 : 2)
    }
}

struct ActivityClusterView: View {
    let cluster: ActivityClusterModel
    @State private var isExpanded = false

    private var summary: String {
        ActivitySummaryBuilder.summarize(items: cluster.items)
    }

    var body: some View {
        if cluster.items.count == 1, let item = cluster.items.first {
            ActivityItemRow(item: item)
                .id(item.id)
        } else {
            DisclosureGroup(isExpanded: $isExpanded) {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(cluster.items) { item in
                        ActivityItemRow(item: item)
                    }
                }
                .padding(.top, TamtriSpacing.xs)
            } label: {
                Text(summary)
                    .font(TamtriTheme.mutedActionFont())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            .disclosureGroupStyle(MutedActionDisclosureStyle(compact: true))
            .padding(.vertical, TamtriLayout.transcriptActivityClusterMargin)
            .accessibilityLabel("Activity: \(summary)")
        }
    }
}

struct ActivityItemRow: View {
    let item: ActivityItemModel
    @State private var isExpanded = false

    private var label: String { ActivitySummaryBuilder.lineLabel(for: item) }
    private var detail: String? { ActivitySummaryBuilder.detailText(for: item) }
    private var diff: DiffPayload? {
        if case .tool(let presentation) = item.kind {
            return presentation.diff
        }
        return nil
    }

    var body: some View {
        switch item.kind {
        case .thinking(let presentation):
            ThinkingDisclosureView(presentation: presentation, isExpanded: $isExpanded)
        case .tool:
            if diff != nil || detail != nil {
                DisclosureGroup(isExpanded: $isExpanded) {
                    itemBody
                        .padding(.top, TamtriSpacing.xs)
                } label: {
                    itemLabel
                }
                .disclosureGroupStyle(MutedActionDisclosureStyle())
            } else {
                itemLabel
            }
        }
    }

    private var itemLabel: some View {
        MutedActionLabel(
            systemImage: ActivitySummaryBuilder.systemImage(for: item),
            title: label
        )
    }

    @ViewBuilder
    private var itemBody: some View {
        if let diff {
            FileDiffView(diff: diff)
        } else if let detail {
            ToolOutputText(content: detail)
        }
    }
}

struct PermissionResolvedReceiptView: View {
    let receipt: PermissionResolvedReceipt

    var body: some View {
        MutedActionLabel(systemImage: "checkmark.shield", title: receipt.summary)
            .accessibilityLabel(receipt.accessibilityLabel)
    }
}

struct PermissionCardView: View {
    let presentation: PermissionCardPresentation
    var onSelectOption: ((String) -> Void)?

    var body: some View {
        TamtriCard(
            title: "Permission requested",
            systemImage: "hand.raised",
            tone: .consent,
            accentBar: true
        ) {
            if let harnessName = presentation.harnessDisplayName, !harnessName.isEmpty {
                Text("From \(harnessName)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            if let command = presentation.command, !command.isEmpty {
                Text(command)
                    .font(TamtriTheme.monoDetailFont())
                    .textSelection(.enabled)
            } else if let diff = presentation.diff {
                FileDiffView(diff: diff)
            } else if !presentation.summary.isEmpty {
                Text(presentation.summary)
                    .textSelection(.enabled)
            }
            if let onSelectOption {
                HStack {
                    ForEach(presentation.allowOptions) { option in
                        Button(option.label) {
                            onSelectOption(option.id)
                        }
                        .buttonStyle(.borderedProminent)
                        .keyboardShortcut(.defaultAction)
                    }
                    ForEach(presentation.rejectOptions) { option in
                        Button(option.label, role: .destructive) {
                            onSelectOption(option.id)
                        }
                        .buttonStyle(.bordered)
                    }
                }
            }
        }
        .accessibilityLabel(presentation.accessibilityLabel)
    }
}

struct FileDiffView: View {
    let diff: DiffPayload

    var body: some View {
        VStack(alignment: .leading, spacing: TamtriSpacing.xs) {
            if let path = diff.path, !path.isEmpty {
                Text(path)
                    .font(TamtriTheme.mutedActionFont())
                    .foregroundStyle(.secondary)
            }
            if let change = diff.change, !change.isEmpty {
                Text(change.capitalized)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            if let oldText = diff.oldText, !oldText.isEmpty {
                Text("Before")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                Text(oldText)
                    .font(TamtriTheme.monoDetailFont())
                    .foregroundStyle(.tertiary)
                    .textSelection(.enabled)
            }
            if let newText = diff.newText, !newText.isEmpty {
                Text("After")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                Text(newText)
                    .font(TamtriTheme.monoDetailFont())
                    .foregroundStyle(.tertiary)
                    .textSelection(.enabled)
            }
        }
    }
}