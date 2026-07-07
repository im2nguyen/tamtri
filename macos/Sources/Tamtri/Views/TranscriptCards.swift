import SwiftUI
import WebKit
import AppKit
import Foundation

struct EventRow: View {
    @EnvironmentObject private var store: AppStore
    let event: CoreEvent

    var body: some View {
        switch event.kind {
        case "permission_requested":
            PermissionCard(event: event)
        case "app_bridge_consent_requested":
            AppBridgeConsentCard(event: event)
        case "app_returned":
            AppReturnedCard(event: event)
        case "task_started", "task_updated":
            if let state = store.liveTaskStates[EventPayload.string(from: event.payloadJSON, key: "task_id") ?? ""] {
                TaskLiveCard(conversationId: event.conversationId, state: state)
            } else {
                TaskLiveCard(conversationId: event.conversationId, state: LiveTaskState(payloadJSON: event.payloadJSON))
            }
        case "task_completed":
            TaskLiveCard(
                conversationId: event.conversationId,
                state: LiveTaskState(payloadJSON: event.payloadJSON)
            )
        case "elicitation_requested":
            ElicitationCard(event: event)
        case "thought_delta":
            if let presentation = ThinkingDisclosurePresentationBuilder.fromLivePayloadJSON(event.payloadJSON) {
                ThinkingDisclosureView(presentation: presentation)
            }
        case "tool_call_started", "tool_call_progress", "tool_call_completed", "file_changed":
            ToolCard(event: event)
        case "text_delta":
            TranscriptMarkdownText(content: EventPayload.text(from: event.payloadJSON), muted: false)
        case "message_committed":
            MessageRow(
                conversationId: event.conversationId,
                message: ParsedTranscriptMessage(json: event.payloadJSON, fallbackIndex: 0),
                allMessages: store.displayedConversation?.parsedMessages ?? []
            )
        case "permission_resolved":
            if let optionID = EventPayload.string(from: event.payloadJSON, key: "option_id") {
                PermissionResolvedReceiptView(
                    receipt: PermissionResolvedReceipt(
                        summary: "Permission · \(PermissionResolvedReceiptBuilder.friendlyLabel(for: optionID))",
                        accessibilityLabel: "Permission resolved"
                    )
                )
            } else {
                Text("Permission resolved")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        case "turn_ended":
            Text("Turn ended")
                .font(.caption)
                .foregroundStyle(.secondary)
        case "gateway_server_connected", "gateway_tool_routed", "gateway_progress", "gateway_log", "gateway_cancellation", "gateway_credential_injected", "gateway_downstream_error", "elicitation_resolved":
            GatewayEventCard(event: event)
        default:
            EmptyView()
        }
    }
}

struct GatewayEventCard: View {
    let event: CoreEvent
    @State private var isExpanded = false

    var body: some View {
        DisclosureGroup(isExpanded: $isExpanded) {
            Text(JSONValue.from(json: event.payloadJSON)?.description ?? event.payloadJSON)
                .font(TamtriTheme.monoDetailFont())
                .foregroundStyle(.tertiary)
                .textSelection(.enabled)
                .padding(.top, TamtriSpacing.xs)
                .padding(.leading, TamtriSpacing.lg)
        } label: {
            MutedActionLabel(systemImage: systemImage, title: summaryLine)
        }
        .disclosureGroupStyle(MutedActionDisclosureStyle())
        .accessibilityLabel(title)
    }

    private var summaryLine: String {
        switch event.kind {
        case "gateway_downstream_error": "Gateway error"
        case "gateway_progress": "Gateway progress"
        case "gateway_credential_injected": "Credential injected"
        default: title
        }
    }

    private var title: String {
        event.kind.replacingOccurrences(of: "_", with: " ").capitalized
    }

    private var systemImage: String {
        switch event.kind {
        case "gateway_downstream_error":
            "exclamationmark.triangle"
        case "gateway_progress":
            "progress.indicator"
        case "gateway_credential_injected":
            "key"
        default:
            "point.3.connected.trianglepath.dotted"
        }
    }
}

struct TranscriptMessageActionBar: View {
    let isUser: Bool
    let timestamp: String?
    let showsRetry: Bool
    let showsEdit: Bool
    let showsCopy: Bool
    let showsFork: Bool
    let onRetry: () -> Void
    let onEdit: () -> Void
    let onCopy: () -> Void
    let onFork: () -> Void

    var body: some View {
        HStack(spacing: TamtriSpacing.xs) {
            if isUser {
                userActionItems
            } else {
                assistantActionItems
            }
        }
        .accessibilityElement(children: .contain)
    }

    @ViewBuilder
    private var userActionItems: some View {
        if let timestamp {
            Text(timestamp)
                .font(TamtriTheme.uiMetaFont())
                .foregroundStyle(.tertiary)
                .accessibilityLabel("Sent \(timestamp)")
        }
        if showsRetry {
            TranscriptIconActionButton(
                systemImage: "arrow.clockwise",
                label: "Retry message",
                action: onRetry
            )
        }
        if showsEdit {
            TranscriptIconActionButton(
                systemImage: "pencil",
                label: "Edit message",
                action: onEdit
            )
        }
        if showsCopy {
            TranscriptIconActionButton(
                systemImage: "doc.on.doc",
                label: "Copy message",
                action: onCopy
            )
        }
    }

    @ViewBuilder
    private var assistantActionItems: some View {
        if showsCopy {
            TranscriptIconActionButton(
                systemImage: "doc.on.doc",
                label: "Copy message",
                action: onCopy
            )
        }
        if showsRetry {
            TranscriptIconActionButton(
                systemImage: "arrow.clockwise",
                label: "Regenerate response",
                action: onRetry
            )
        }
        if showsFork {
            TranscriptIconActionButton(
                systemImage: "arrow.triangle.branch",
                label: "Fork conversation",
                action: onFork
            )
        }
        if let timestamp {
            Text(timestamp)
                .font(TamtriTheme.uiMetaFont())
                .foregroundStyle(.tertiary)
                .accessibilityLabel("Sent \(timestamp)")
        }
    }
}

private struct TranscriptIconActionButton: View {
    let systemImage: String
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 12, weight: .regular))
                .foregroundStyle(.tertiary)
                .frame(width: 20, height: 20)
                .contentShape(Rectangle())
        }
        .buttonStyle(.tamtriPlain)
        .help(label)
        .accessibilityLabel(label)
    }
}

struct MessageRow: View {
    @Environment(\.colorScheme) private var colorScheme
    @EnvironmentObject private var store: AppStore
    let conversationId: String
    let message: ParsedTranscriptMessage
    var previousRole: String?
    var allMessages: [ParsedTranscriptMessage] = []

    @State private var isHoverEngaged = false
    @State private var hoverExitTask: Task<Void, Never>?

    private static let actionBarHeight: CGFloat = 20
    private static let hoverExitDelayNs: UInt64 = 250_000_000

    private var isUser: Bool { message.role == "user" }
    private var plainText: String { TranscriptMessageText.plainText(from: message) }
    private var hasTextActions: Bool { !plainText.isEmpty }
    private var userTimestampLabel: String? {
        TamtriFormatting.messageBubbleTimestamp(from: message.createdAt)
    }
    private var assistantTimestampLabel: String? {
        TamtriFormatting.messageActionBarTimestamp(from: message.createdAt)
    }
    private var isLatestUserMessage: Bool {
        isUser && allMessages.last(where: { $0.role == "user" })?.id == message.id
    }
    private var isLatestAssistantMessage: Bool {
        !isUser && allMessages.last(where: { $0.role == "assistant" })?.id == message.id
    }
    private var hasActionBarContent: Bool {
        if isUser {
            return userTimestampLabel != nil || hasTextActions
        }
        return hasTextActions || assistantTimestampLabel != nil
    }
    private var isActionBarPersistent: Bool {
        isLatestAssistantMessage && hasTextActions
    }
    private var isActionBarVisible: Bool {
        isActionBarPersistent || (isHoverEngaged && hasActionBarContent)
    }
    private var reservesActionBarSpace: Bool {
        hasActionBarContent && (isActionBarPersistent || isHoverEngaged)
    }

    var body: some View {
        if let rawJSON = message.rawJSON {
            Text(rawJSON)
                .font(.body.monospaced())
                .textSelection(.enabled)
        } else {
            VStack(alignment: .leading, spacing: 0) {
                messageContent
                    .frame(maxWidth: .infinity, alignment: isUser ? .trailing : .leading)
                if hasActionBarContent {
                    messageActionBar
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .padding(.top, isUser ? TamtriSpacing.xs : 0)
            .padding(.bottom, isUser ? 0 : TamtriLayout.transcriptTurnSpacing)
            .onHover(perform: updateHoverEngagement)
            .onDisappear { hoverExitTask?.cancel() }
            .animation(.easeInOut(duration: 0.12), value: isActionBarVisible)
            .focusable()
            .focusEffectDisabled()
            .accessibilityElement(children: .contain)
            .accessibilityLabel(isUser ? "You" : "Assistant")
        }
    }

    private func updateHoverEngagement(_ inside: Bool) {
        hoverExitTask?.cancel()
        if inside {
            isHoverEngaged = true
        } else {
            hoverExitTask = Task { @MainActor in
                try? await Task.sleep(nanoseconds: Self.hoverExitDelayNs)
                guard !Task.isCancelled else { return }
                isHoverEngaged = false
            }
        }
    }

    @ViewBuilder
    private var messageContent: some View {
        if isUser {
            messageBody
                .padding(TamtriSpacing.md)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(
                    RoundedRectangle(cornerRadius: TamtriRadius.bar)
                        .fill(TamtriTheme.userMessageBackground(colorScheme))
                )
                .overlay(
                    RoundedRectangle(cornerRadius: TamtriRadius.bar)
                        .strokeBorder(TamtriTheme.hairline(colorScheme), lineWidth: 0.5)
                )
        } else {
            messageBody
                .padding(.horizontal, TamtriLayout.composerInset)
                .padding(.top, previousRole == "user" ? TamtriLayout.transcriptUserToAssistantGap : 0)
        }
    }

    private var messageActionBar: some View {
        TranscriptMessageActionBar(
            isUser: isUser,
            timestamp: isUser ? userTimestampLabel : assistantTimestampLabel,
            showsRetry: hasTextActions && (isUser || isLatestAssistantMessage),
            showsEdit: isUser && isLatestUserMessage && hasTextActions,
            showsCopy: hasTextActions,
            showsFork: !isUser && hasTextActions,
            onRetry: {
                if isUser {
                    store.retryUserMessage(message)
                } else {
                    store.retryAssistantMessage(message, in: allMessages)
                }
            },
            onEdit: { store.prefillComposerForEdit(text: plainText) },
            onCopy: { store.copyMessageText(plainText) },
            onFork: { store.presentForkConversation() }
        )
        .padding(.leading, isUser ? TamtriSpacing.md : TamtriLayout.composerInset)
        .padding(.top, reservesActionBarSpace ? 2 : 0)
        .frame(maxWidth: .infinity, alignment: .leading)
        .frame(height: reservesActionBarSpace ? Self.actionBarHeight : 0, alignment: .leading)
        .clipped()
        .opacity(isActionBarVisible ? 1 : 0)
        .allowsHitTesting(isActionBarVisible)
    }

    @ViewBuilder
    private var messageBody: some View {
        VStack(alignment: .leading, spacing: TamtriLayout.transcriptBlockSpacing) {
            ForEach(messageBodyItems) { item in
                switch item {
                case .sectionHeader(let title):
                    TranscriptSectionHeader(title: title)
                case .text(let block):
                    TranscriptMarkdownText(content: block.text ?? "", muted: false)
                case .activityCluster(let cluster):
                    ActivityClusterView(cluster: cluster)
                case .toolGroup(let group):
                    VStack(alignment: .leading, spacing: TamtriSpacing.sm) {
                        if let toolBlock = group.toolBlock {
                            ContentBlockView(conversationId: conversationId, block: toolBlock)
                        }
                        ForEach(Array(group.nested.enumerated()), id: \.offset) { _, nested in
                            ContentBlockView(conversationId: conversationId, block: nested)
                                .padding(.leading, TamtriSpacing.lg)
                        }
                    }
                case .artifactGroup(let blocks):
                    ArtifactCardGroup(conversationId: conversationId, blocks: blocks)
                        .padding(.bottom, TamtriLayout.transcriptRichBlockBottomSpacing)
                case .rich(let block):
                    richBlockView(block)
                }
            }
        }
    }

    private enum MessageBodyItem: Identifiable {
        case sectionHeader(String)
        case text(TranscriptContentBlock)
        case activityCluster(ActivityClusterModel)
        case toolGroup(TranscriptBlockGroup)
        case artifactGroup([TranscriptContentBlock])
        case rich(TranscriptContentBlock)

        var id: String {
            switch self {
            case .sectionHeader(let title):
                "header-\(title)"
            case .text(let block), .rich(let block):
                "block-\(block.type)-\(block.callId ?? block.text?.prefix(8).description ?? UUID().uuidString)"
            case .activityCluster(let cluster):
                "activity-\(cluster.id)"
            case .toolGroup(let group):
                "tool-group-\(group.id)"
            case .artifactGroup(let blocks):
                "artifact-group-\(blocks.enumerated().map { "\($0.offset)-\($0.element.path ?? "")" }.joined(separator: "|"))"
            }
        }
    }

    private var messageBodyItems: [MessageBodyItem] {
        var items: [MessageBodyItem] = []
        var filesHeaderShown = false
        let segments = messageSegments
        let hasArtifacts = segments.contains { segment in
            if case .rich(let block) = segment, block.type == "artifact" { return true }
            return false
        }

        var index = 0
        while index < segments.count {
            let segment = segments[index]
            if case .rich(let block) = segment, block.type == "artifact" {
                var artifactBlocks = [block]
                var next = index + 1
                while next < segments.count {
                    if case .rich(let candidate) = segments[next], candidate.type == "artifact" {
                        artifactBlocks.append(candidate)
                        next += 1
                    } else {
                        break
                    }
                }
                if hasArtifacts && !filesHeaderShown {
                    items.append(.sectionHeader("Output"))
                    filesHeaderShown = true
                }
                items.append(.artifactGroup(artifactBlocks))
                index = next
                continue
            }

            switch segment {
            case .text(let block):
                items.append(.text(block))
            case .activityCluster(let cluster):
                items.append(.activityCluster(cluster))
            case .toolGroup(let group):
                items.append(.toolGroup(group))
            case .rich(let block):
                items.append(.rich(block))
            }
            index += 1
        }
        return items
    }

    @ViewBuilder
    private func richBlockView(_ block: TranscriptContentBlock) -> some View {
        ContentBlockView(conversationId: conversationId, block: block)
            .padding(.bottom, richBlockNeedsExtraSpacing(block) ? TamtriLayout.transcriptRichBlockBottomSpacing : 0)
    }

    private func richBlockNeedsExtraSpacing(_ block: TranscriptContentBlock) -> Bool {
        switch block.type {
        case "artifact", "elicitation_request", "elicitation_response", "app_resource":
            return true
        case "tool_result":
            return PermissionResolvedReceiptBuilder.fromCommittedBlock(block) != nil
        default:
            return false
        }
    }

    private var messageSegments: [MessageDisplaySegment] {
        ActivityContentGrouping.messageSegments(from: message.content)
    }
}

struct ContentBlockView: View {
    let conversationId: String
    let block: TranscriptContentBlock

    var body: some View {
        switch block.type {
        case "text":
            TranscriptMarkdownText(content: block.text ?? "", muted: false)
                .textSelection(.enabled)
        case "thinking":
            if let presentation = ThinkingDisclosurePresentationBuilder.fromCommittedBlock(block) {
                ThinkingDisclosureView(presentation: presentation)
            }
        case "tool_call":
            if let presentation = ToolCardPresentationBuilder.fromCommittedCallBlock(block) {
                ToolCardView(presentation: presentation)
            }
        case "tool_result":
            if let receipt = PermissionResolvedReceiptBuilder.fromCommittedBlock(block) {
                PermissionResolvedReceiptView(receipt: receipt)
            } else if let presentation = ToolCardPresentationBuilder.fromCommittedResultBlock(block) {
                ToolCardView(presentation: presentation)
            }
        case "elicitation_request":
            ElicitationHistoryCard(block: block)
        case "elicitation_response":
            CompactCard(title: "Elicitation response", systemImage: "bubble.left.and.text.bubble.right") {
                Text(block.action ?? "unknown")
                    .font(.body)
                if let data = block.data {
                    Text(data.truncatedDescription)
                        .font(.body.monospaced())
                        .textSelection(.enabled)
                }
            }
        case "app_resource":
            AppPanelView(
                conversationId: conversationId,
                serverId: block.serverId,
                templateRef: block.templateRef,
                uri: block.uri,
                state: block.state
            )
        case "task_ref":
            TaskRefCard(block: block)
        case "artifact":
            ArtifactCardGroup(conversationId: conversationId, blocks: [block])
                .padding(.bottom, TamtriLayout.transcriptRichBlockBottomSpacing)
        default:
            EmptyView()
        }
    }
}

struct AppPanelView: View {
    @EnvironmentObject private var store: AppStore
    let conversationId: String
    let serverId: String?
    let templateRef: String?
    let uri: String?
    let state: JSONValue?
    @State private var template: AppTemplateRecord?
    @State private var loadError: String?
    @State private var webViewID = UUID()
    @State private var pendingBridgeResponse: String?

    init(conversationId: String, block: TranscriptContentBlock) {
        self.conversationId = conversationId
        self.serverId = block.serverId
        self.templateRef = block.templateRef
        self.uri = block.uri
        self.state = block.state
    }

    init(
        conversationId: String,
        serverId: String?,
        templateRef: String?,
        uri: String?,
        state: JSONValue?
    ) {
        self.conversationId = conversationId
        self.serverId = serverId
        self.templateRef = templateRef
        self.uri = uri
        self.state = state
    }

    var body: some View {
        CompactCard(title: appTitle, systemImage: "app.dashed") {
            VStack(alignment: .leading, spacing: 8) {
                if let serverId {
                    Text("Server: \(serverId)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                if let state {
                    Text(state.truncatedDescription)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
                if let template {
                    SandboxedHTMLView(
                        html: template.html,
                        policy: .app(
                            allowedOrigins: template.allowedOrigins.map(WebOrigin.init(mcpOrigin:)),
                            appId: uri ?? templateRef ?? "app",
                            serverId: template.serverId,
                            templateRef: template.templateRef
                        ),
                        bridgeContext: AppBridgeContext(
                            conversationId: conversationId,
                            serverId: template.serverId,
                            appId: uri ?? template.templateRef,
                            templateRef: template.templateRef,
                            bridgeScript: template.bridgeScript,
                            webViewID: webViewID
                        ),
                        persistedStateJSON: state?.toJSONString(),
                        pendingBridgeResponse: pendingBridgeResponse,
                        onBridgeRequest: { requestJSON, viewID in
                            store.submitAppBridgeRequest(
                                conversationId: conversationId,
                                serverId: template.serverId,
                                appId: uri ?? template.templateRef,
                                templateRef: template.templateRef,
                                requestJSON: requestJSON,
                                webViewID: viewID
                            )
                        },
                        onBlockedNavigation: { url in
                            store.logAppNavigationBlocked(
                                conversationId: conversationId,
                                serverId: template.serverId,
                                templateRef: template.templateRef,
                                url: url.absoluteString
                            )
                        }
                    )
                    .frame(minHeight: 260)
                    .accessibilityHidden(true)
                } else if let loadError {
                    Text(loadError)
                        .foregroundStyle(.secondary)
                } else {
                    Text(AppPanelViewModel.offlineMessage)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(AppPanelViewModel.accessibilityLabel(title: appTitle))
        .accessibilityValue(AppPanelViewModel.accessibilityValue(templateLoaded: template != nil))
        .task(id: appLoadKey) {
            await loadTemplate()
        }
        .onChange(of: store.bridgeDelivery) { _, delivery in
            guard let delivery, delivery.webViewID == webViewID else { return }
            pendingBridgeResponse = delivery.responseJSON
        }
    }

    private var appLoadKey: String {
        [conversationId, serverId, templateRef].compactMap { $0 }.joined(separator: "|")
    }

    private var appTitle: String {
        templateRef ?? uri ?? "MCP App"
    }

    private func loadTemplate() async {
        guard let serverId, let templateRef else {
            loadError = "App metadata missing server or template reference."
            return
        }
        do {
            template = try await store.resolveAppTemplate(
                conversationId: conversationId,
                serverId: serverId,
                templateRef: templateRef
            )
            if template == nil {
                loadError = "App template not loaded. Start a run or reconnect to the gateway server."
            }
        } catch {
            loadError = error.localizedDescription
        }
    }
}

struct AppReturnedCard: View {
    let event: CoreEvent

    var body: some View {
        AppPanelView(
            conversationId: event.conversationId,
            serverId: EventPayload.string(from: event.payloadJSON, key: "server_id"),
            templateRef: EventPayload.string(from: event.payloadJSON, key: "template_ref"),
            uri: EventPayload.string(from: event.payloadJSON, key: "uri"),
            state: JSONValue.from(json: event.payloadJSON)?.value(at: "state")
        )
    }
}

struct TaskLiveCard: View {
    @EnvironmentObject private var store: AppStore
    let conversationId: String
    let state: LiveTaskState
    @State private var lastAnnouncedAccessibilityStatus: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(taskTitle, systemImage: statusIcon)
                .font(.headline)
            HStack {
                Text(state.serverId)
                Text(state.status.replacingOccurrences(of: "_", with: " "))
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            if let progress = state.progressMessage, !progress.isEmpty {
                Text(progress)
            }
            if TaskLiveCardViewModel.showsCancelButton(for: state) {
                Button("Cancel task") {
                    store.cancelTask(conversationId: conversationId, taskId: state.taskId)
                }
                .keyboardShortcut(.cancelAction)
            }
        }
        .padding(10)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
        .focusable()
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Task \(taskTitle)")
        .accessibilityValue(accessibilityStatus)
        .accessibilityAddTraits(.updatesFrequently)
        .accessibilityAction(named: "Cancel task") {
            guard TaskLiveCardViewModel.showsCancelButton(for: state) else { return }
            store.cancelTask(conversationId: conversationId, taskId: state.taskId)
        }
        .onAppear {
            announceAccessibilityStatusIfNeeded()
        }
        .onChange(of: accessibilityStatus) { _, _ in
            announceAccessibilityStatusIfNeeded()
        }
    }

    private var taskTitle: String { state.title ?? state.taskId }
    private var statusIcon: String { TaskLiveCardViewModel.statusIcon(for: state.status) }
    private var accessibilityStatus: String { TaskLiveCardViewModel.accessibilityStatus(for: state) }

    private func announceAccessibilityStatusIfNeeded() {
        let announcement = TaskLiveCardViewModel.accessibilityAnnouncement(for: state)
        guard lastAnnouncedAccessibilityStatus != announcement else { return }
        lastAnnouncedAccessibilityStatus = announcement
        AccessibilityNotification.Announcement(announcement).post()
    }
}

struct TaskRefCard: View {
    let block: TranscriptContentBlock

    var body: some View {
        CompactCard(title: TaskRefCardViewModel.title(block: block), systemImage: "checklist") {
            Text("Status: \(block.taskStatus ?? "unknown")")
            if let summary = block.taskResultSummary, !summary.isEmpty {
                Text(summary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        }
        .accessibilityLabel("Task \(block.taskId ?? "unknown")")
        .accessibilityValue(TaskRefCardViewModel.accessibilityValue(status: block.taskStatus, resultSummary: block.taskResultSummary))
    }
}

struct AppBridgeConsentCard: View {
    @EnvironmentObject private var store: AppStore
    let event: CoreEvent
    private var request: AppBridgeConsentPayload? {
        try? JSONDecoder().decode(AppBridgeConsentPayload.self, from: Data(event.payloadJSON.utf8))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(AppBridgeConsentViewModel.headline, systemImage: "hand.raised")
                .font(.headline)
            if let request {
                Text(request.summary)
                    .textSelection(.enabled)
                Text(AppBridgeConsentViewModel.serverAttribution(serverId: request.serverId, appId: request.appId))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            HStack {
                if let request {
                    ForEach(request.options.filter { $0.id == "deny" }) { option in
                        Button(option.label) {
                            store.respondAppBridgeConsent(requestId: request.requestId, optionId: option.id)
                        }
                    }
                    ForEach(request.options.filter { $0.id != "deny" }) { option in
                        Button(option.label) {
                            store.respondAppBridgeConsent(requestId: request.requestId, optionId: option.id)
                        }
                    }
                    .keyboardShortcut(.defaultAction)
                } else {
                    Text("Unable to parse app bridge consent request")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(10)
        .background(.yellow.opacity(0.18), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityLabel("App bridge consent requested")
    }
}

let artifactPlainTextPreviewLineLimit = 24

func artifactIsTextLikeMime(_ mimeType: String?) -> Bool {
    guard let mimeType else { return false }
    return mimeType.hasPrefix("text/")
        || mimeType == "application/json"
        || mimeType == "application/xml"
        || mimeType == "image/svg+xml"
}

func artifactIsImageMime(_ mimeType: String?) -> Bool {
    guard let mimeType else { return false }
    return ["image/png", "image/jpeg", "image/gif", "image/webp"].contains(mimeType)
}

func artifactFileIcon(_ mimeType: String?) -> String {
    switch mimeType {
    case "text/html": return "doc.richtext"
    case "text/csv", "text/tab-separated-values": return "tablecells"
    case "text/markdown": return "text.quote"
    case "image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml": return "photo"
    case "application/json": return "curlybraces"
    case "application/pdf": return "doc.fill"
    default: return "doc"
    }
}

func artifactMimeLabel(_ mimeType: String?) -> String {
    guard let mimeType else { return "File" }
    switch mimeType {
    case "text/html": return "HTML"
    case "text/csv": return "CSV"
    case "text/tab-separated-values": return "TSV"
    case "text/markdown": return "Markdown"
    case "text/plain": return "Text"
    case "application/json": return "JSON"
    case "application/pdf": return "PDF"
    case "image/png": return "PNG"
    case "image/jpeg": return "JPEG"
    case "image/gif": return "GIF"
    case "image/webp": return "WebP"
    case "image/svg+xml": return "SVG"
    default: return mimeType
    }
}

struct TypedFileCard: View {
    let path: String?
    let mimeType: String?
    let size: UInt64?
    var integrityStatus: String?

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: artifactFileIcon(mimeType))
                .font(.title2)
                .foregroundStyle(integrityStatus == nil ? Color.secondary : Color.red)
                .frame(width: 28)
            VStack(alignment: .leading, spacing: 4) {
                Text((path as NSString?)?.lastPathComponent ?? "Attachment")
                    .font(.body.weight(.medium))
                    .lineLimit(1)
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(integrityStatus == nil ? Color.secondary : Color.red)
            }
            Spacer()
        }
        .padding(10)
        .background(Color.secondary.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityLabel)
    }

    private var subtitle: String {
        let type = artifactMimeLabel(mimeType)
        var parts = [type]
        if let size {
            let formatted = ByteCountFormatter.string(fromByteCount: Int64(size), countStyle: .file)
            parts.append(formatted)
        }
        if let integrityStatus {
            parts.append(integrityStatus)
        }
        return parts.joined(separator: " · ")
    }

    private var accessibilityLabel: String {
        let name = (path as NSString?)?.lastPathComponent ?? "Attachment"
        let type = artifactMimeLabel(mimeType)
        if let integrityStatus {
            if let size {
                let formatted = ByteCountFormatter.string(fromByteCount: Int64(size), countStyle: .file)
                return "\(name), \(type), \(formatted), \(integrityStatus)"
            }
            return "\(name), \(type), \(integrityStatus)"
        }
        if let size {
            let formatted = ByteCountFormatter.string(fromByteCount: Int64(size), countStyle: .file)
            return "\(name), \(type), \(formatted). Open or reveal in Finder."
        }
        return "\(name), \(type). Open or reveal in Finder."
    }
}

struct MarkdownPreview: View {
    let content: String

    var body: some View {
        if let attributed = attributedMarkdownPreview(content) {
            Text(attributed)
                .textSelection(.enabled)
        } else {
            Text(sanitizedMarkdownForPreview(content))
                .textSelection(.enabled)
        }
    }
}

struct SandboxedHTMLView: NSViewRepresentable {
    let html: String
    var policy: WebContentPolicy = .artifactNoNetwork
    var bridgeContext: AppBridgeContext?
    var persistedStateJSON: String?
    var pendingBridgeResponse: String?
    var onBridgeRequest: ((String, UUID) -> Void)?
    var onBlockedNavigation: ((URL) -> Void)?

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        if bridgeContext != nil {
            configuration.userContentController.add(context.coordinator, name: AppWebViewBridge.handlerName)
        }
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = context.coordinator
        webView.uiDelegate = context.coordinator
        context.coordinator.webView = webView
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        context.coordinator.policy = policy
        context.coordinator.bridgeContext = bridgeContext
        context.coordinator.onBridgeRequest = onBridgeRequest
        context.coordinator.onBlockedNavigation = onBlockedNavigation
        if let pendingBridgeResponse, pendingBridgeResponse != context.coordinator.lastDeliveredResponse {
            context.coordinator.deliverBridgeResponse(pendingBridgeResponse)
            context.coordinator.lastDeliveredResponse = pendingBridgeResponse
        }
        let bridgeScript = bridgeContext?.bridgeScript
        let sandboxed = sandboxedHTML(
            for: html,
            policy: policy,
            bridgeScript: bridgeScript,
            persistedStateJSON: persistedStateJSON
        )
        guard sandboxed != context.coordinator.lastLoadedHTML else { return }
        context.coordinator.lastLoadedHTML = sandboxed
        webView.loadHTMLString(sandboxed, baseURL: nil)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(
            policy: policy,
            bridgeContext: bridgeContext,
            onBridgeRequest: onBridgeRequest,
            onBlockedNavigation: onBlockedNavigation
        )
    }

    final class Coordinator: NSObject, WKNavigationDelegate, WKUIDelegate, WKScriptMessageHandler {
        var policy: WebContentPolicy
        var bridgeContext: AppBridgeContext?
        var onBridgeRequest: ((String, UUID) -> Void)?
        var onBlockedNavigation: ((URL) -> Void)?
        var lastDeliveredResponse: String?
        var lastLoadedHTML: String?
        weak var webView: WKWebView?

        init(
            policy: WebContentPolicy,
            bridgeContext: AppBridgeContext?,
            onBridgeRequest: ((String, UUID) -> Void)?,
            onBlockedNavigation: ((URL) -> Void)?
        ) {
            self.policy = policy
            self.bridgeContext = bridgeContext
            self.onBridgeRequest = onBridgeRequest
            self.onBlockedNavigation = onBlockedNavigation
        }

        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            guard message.name == AppWebViewBridge.handlerName,
                  case .app = policy,
                  let body = message.body as? String,
                  let bridgeContext
            else {
                return
            }
            onBridgeRequest?(body, bridgeContext.webViewID)
        }

        func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void) {
            guard let url = navigationAction.request.url else {
                decisionHandler(.allow)
                return
            }
            if WebNavigationPolicy.allows(url, policy: policy) {
                decisionHandler(.allow)
            } else {
                onBlockedNavigation?(url)
                decisionHandler(.cancel)
            }
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationResponse: WKNavigationResponse,
            decisionHandler: @escaping @MainActor @Sendable (WKNavigationResponsePolicy) -> Void
        ) {
            guard let url = navigationResponse.response.url else {
                decisionHandler(.allow)
                return
            }
            if WebNavigationPolicy.allows(url, policy: policy) {
                decisionHandler(.allow)
            } else {
                onBlockedNavigation?(url)
                decisionHandler(.cancel)
            }
        }

        func deliverBridgeResponse(_ responseJSON: String) {
            guard let webView else { return }
            let escaped = responseJSON
                .replacingOccurrences(of: "\\", with: "\\\\")
                .replacingOccurrences(of: "'", with: "\\'")
            webView.evaluateJavaScript("window.__tamtriAppBridgeDeliver('\(escaped)');", completionHandler: nil)
        }

        func webView(
            _ webView: WKWebView,
            createWebViewWith configuration: WKWebViewConfiguration,
            for navigationAction: WKNavigationAction,
            windowFeatures: WKWindowFeatures
        ) -> WKWebView? {
            if let url = navigationAction.request.url {
                onBlockedNavigation?(url)
            }
            return nil
        }

        func webView(
            _ webView: WKWebView,
            runJavaScriptAlertPanelWithMessage message: String,
            initiatedByFrame frame: WKFrameInfo,
            completionHandler: @escaping @MainActor @Sendable () -> Void
        ) {
            completionHandler()
        }

        func webView(
            _ webView: WKWebView,
            runJavaScriptConfirmPanelWithMessage message: String,
            initiatedByFrame frame: WKFrameInfo,
            completionHandler: @escaping @MainActor @Sendable (Bool) -> Void
        ) {
            completionHandler(false)
        }

        func webView(
            _ webView: WKWebView,
            runJavaScriptTextInputPanelWithPrompt prompt: String,
            defaultText: String?,
            initiatedByFrame frame: WKFrameInfo,
            completionHandler: @escaping @MainActor @Sendable (String?) -> Void
        ) {
            completionHandler(nil)
        }
    }
}

struct CSVPreview: View {
    let text: String
    let separator: Character
    var stylesHeaderRow = true

    private var rows: [[String]] {
        csvPreviewRows(text: text, separator: separator)
    }

    var body: some View {
        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
            ForEach(Array(rows.enumerated()), id: \.offset) { rowIndex, row in
                GridRow {
                    ForEach(Array(row.enumerated()), id: \.offset) { _, cell in
                        Text(cell)
                            .font(rowIndex == 0 && stylesHeaderRow ? .caption.bold().monospaced() : .caption.monospaced())
                            .lineLimit(2)
                    }
                }
            }
        }
        .padding(8)
        .background(.background, in: RoundedRectangle(cornerRadius: 6))
    }
}

struct ToolCard: View {
    let event: CoreEvent

    var body: some View {
        if let presentation = ToolCardPresentationBuilder.fromLiveEvent(
            kind: event.kind,
            payloadJSON: event.payloadJSON
        ) {
            ToolCardView(presentation: presentation)
        }
    }
}

struct CompactCard<Content: View>: View {
    let title: String
    let systemImage: String
    var tone: TamtriTheme.SemanticTone = .tool
    @ViewBuilder let content: Content

    var body: some View {
        TamtriCard(title: title, systemImage: systemImage, tone: tone, content: { content })
    }
}

struct PermissionCard: View {
    @EnvironmentObject private var store: AppStore
    let event: CoreEvent

    var body: some View {
        if let presentation = PermissionCardPresentationBuilder.build(payloadJSON: event.payloadJSON) {
            PermissionCardView(presentation: presentation) { optionId in
                store.respondPermission(requestId: presentation.requestId, optionId: optionId)
            }
        } else {
            Text("Unable to parse permission request")
                .foregroundStyle(.secondary)
                .padding(10)
                .background(.yellow.opacity(0.18), in: RoundedRectangle(cornerRadius: 8))
        }
    }
}

struct ElicitationCard: View {
    @EnvironmentObject private var store: AppStore
    let event: CoreEvent
    @State private var fieldValues: [String: String] = [:]
    @State private var booleanValues: [String: Bool] = [:]
    @State private var validationMessage: String?

    private var request: ElicitationPayload? {
        try? JSONDecoder().decode(ElicitationPayload.self, from: Data(event.payloadJSON.utf8))
    }

    var body: some View {
        switch ElicitationCardRouter.cardKind(mode: request?.mode, schema: request?.schema) {
        case .urlHandoff:
            if let request {
                URLConsentCard(
                    request: URLConsentRequest(
                        requestId: request.requestId,
                        serverId: request.serverId,
                        originToolCallId: request.originToolCallId,
                        message: request.message,
                        url: request.url
                    )
                )
            }
        case .secretBlocked:
            SecretElicitationBlockedCard(
                onDecline: { respond(action: "decline") },
                onCancel: { respond(action: "cancel") }
            )
        case .unsupportedSchema:
            UnsupportedElicitationSchemaCard(
                message: request?.message ?? "The server sent a form tamtri cannot render.",
                onDecline: { respond(action: "decline") },
                onCancel: { respond(action: "cancel") }
            )
        case .form:
            formCard
        }
    }

    private var formCard: some View {
        let fields = ElicitationSchemaParser.fields(from: request?.schema)
        return VStack(alignment: .leading, spacing: 10) {
            Label("Follow-up from \(request?.serverId ?? "gateway server")", systemImage: "questionmark.bubble")
                .font(.headline)
            if let request {
                Text(request.message)
                    .textSelection(.enabled)
                if fields.isEmpty {
                    TextField("Your answer", text: binding(for: "name", default: ""))
                        .textFieldStyle(.roundedBorder)
                } else {
                    ElicitationSchemaForm(
                        fields: fields,
                        values: $fieldValues,
                        booleans: $booleanValues,
                        validationMessage: $validationMessage
                    )
                }
            }
            HStack {
                Button("Decline") { respond(action: "decline") }
                Button("Cancel", role: .cancel) { respond(action: "cancel") }
                Button("Submit") { respond(action: "accept") }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(10)
        .background(.teal.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityLabel("Elicitation requested")
        .onAppear {
            seedDefaults(from: fields)
        }
    }

    private func seedDefaults(from fields: [ElicitationSchemaField]) {
        for field in fields where fieldValues[field.id] == nil {
            if field.type == "boolean" {
                booleanValues[field.id] = false
            } else if let first = field.enumValues.first {
                fieldValues[field.id] = first
            } else {
                fieldValues[field.id] = ""
            }
        }
    }

    private func binding(for key: String, default defaultValue: String) -> Binding<String> {
        Binding(
            get: { fieldValues[key, default: defaultValue] },
            set: { fieldValues[key] = $0 }
        )
    }

    private func respond(action: String) {
        guard let request else { return }
        let dataJSON: String?
            if action == "accept" {
                let fields = ElicitationSchemaParser.fields(from: request.schema)
                if fields.isEmpty {
                    let field = request.primaryFieldName ?? "name"
                    dataJSON = jsonString([field: fieldValues[field, default: ""]])
                } else {
                    guard let built = ElicitationSchemaFormBuilder.buildPayload(
                        fields: fields,
                        values: fieldValues,
                        booleans: booleanValues
                    ) else {
                        validationMessage = "Complete all required fields."
                        return
                    }
                    if let error = built.error {
                        validationMessage = error
                        return
                    }
                    dataJSON = jsonString(built.payload)
                }
            } else {
            dataJSON = nil
        }
        store.respondElicitation(requestId: request.requestId, action: action, dataJSON: dataJSON)
    }

    private func jsonString(_ payload: [String: Any]) -> String? {
        guard JSONSerialization.isValidJSONObject(payload),
              let data = try? JSONSerialization.data(withJSONObject: payload),
              let json = String(data: data, encoding: .utf8)
        else {
            return nil
        }
        return json
    }
}

struct ElicitationHistoryCard: View {
    let block: TranscriptContentBlock

    var body: some View {
        CompactCard(title: historyTitle, systemImage: historyIcon) {
            VStack(alignment: .leading, spacing: 6) {
                if let serverId = block.serverId {
                    Text("From \(serverId)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Text(block.message ?? "")
                    .textSelection(.enabled)
                if block.mode == "url", let url = block.url {
                    Text(URLHandoffPolicy.redactedDisplay(url))
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var historyTitle: String {
        block.mode == "url" ? "URL handoff requested" : "Elicitation"
    }

    private var historyIcon: String {
        block.mode == "url" ? "safari" : "questionmark.bubble"
    }
}

private struct ElicitationPayload: Decodable {
    let requestId: String
    let serverId: String?
    let originToolCallId: String?
    let mode: String
    let message: String
    let url: String?
    let schema: JSONValue?

    enum CodingKeys: String, CodingKey {
        case requestId = "request_id"
        case serverId = "server_id"
        case originToolCallId = "origin_tool_call_id"
        case mode
        case message
        case url
        case schema
    }

    var primaryFieldName: String? {
        guard case .object(let object) = schema,
              case .object(let properties) = object["properties"] ?? .null
        else {
            return "name"
        }
        if case .array(let required) = object["required"] ?? .null {
            for item in required {
                if case .string(let name) = item, properties[name] != nil {
                    return name
                }
            }
        }
        return properties.keys.sorted().first ?? "name"
    }
}

private struct EventPayload: Decodable {
    let text: String?

    static func text(from json: String) -> String {
        (try? JSONDecoder().decode(EventPayload.self, from: Data(json.utf8)).text) ?? ""
    }

    static func string(from json: String, key: String) -> String? {
        JSONValue.from(json: json)?.string(at: key)
    }
}
