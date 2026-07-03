import Foundation
import SwiftUI
import UniformTypeIdentifiers
import WebKit

struct RootView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        NavigationSplitView {
            SidebarView()
        } detail: {
            TranscriptView()
        }
        .sheet(isPresented: $store.showNewConversation) {
            NewConversationView()
        }
        .sheet(isPresented: $store.showSettings) {
            SettingsView()
        }
        .sheet(isPresented: $store.showForkConversation) {
            ForkConversationView()
        }
        .sheet(isPresented: $store.showConversationRoots) {
            if let conversation = store.displayedConversation {
                NavigationStack {
                    ConversationRootsSettingsView(conversationId: conversation.id)
                        .padding()
                        .navigationTitle("Conversation Roots")
                        .toolbar {
                            Button("Done") {
                                store.showConversationRoots = false
                            }
                        }
                }
                .frame(minWidth: 480, minHeight: 320)
            }
        }
        .alert("Tamtri", isPresented: errorPresented) {
            Button("OK") {
                store.errorMessage = nil
            }
        } message: {
            Text(store.errorMessage ?? "")
        }
    }

    private var errorPresented: Binding<Bool> {
        Binding(
            get: { store.errorMessage != nil },
            set: { isPresented in
                if !isPresented {
                    store.errorMessage = nil
                }
            }
        )
    }
}

struct SidebarView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        List(store.conversations, selection: $store.selectedConversationId) { conversation in
            VStack(alignment: .leading, spacing: 2) {
                Text(conversation.title)
                    .font(.headline)
                Text(conversation.updatedAt)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .tag(conversation.id)
        }
        .navigationTitle("tamtri")
        .onChange(of: store.selectedConversationId) { _, newId in
            guard let newId,
                  store.selectedConversation?.id != newId,
                  let summary = store.conversations.first(where: { $0.id == newId })
            else { return }
            store.selectConversation(summary)
        }
        .toolbar {
            Button {
                store.showSettings = true
                Task { await store.refreshGatewayServers() }
            } label: {
                Label("Settings", systemImage: "gearshape")
            }
            Button {
                store.showNewConversation = true
            } label: {
                Label("New Conversation", systemImage: "plus")
            }
        }
    }
}

struct TranscriptView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        let conversation = store.displayedConversation
        let switching = store.isSwitchingConversation

        VStack(spacing: 0) {
            ZStack {
                if let conversation {
                    VStack(alignment: .leading, spacing: 0) {
                        ConversationHeader(conversation: conversation)
                            .padding(.horizontal)
                            .padding(.top, 12)
                        ScrollView {
                            LazyVStack(alignment: .leading, spacing: 12) {
                                ForEach(conversation.parsedMessages) { message in
                                    MessageRow(
                                        conversationId: conversation.id,
                                        message: message
                                    )
                                }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.horizontal)
                            .padding(.bottom, 16)
                        }
                        .id(conversation.id)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                } else if switching {
                    VStack(spacing: 12) {
                        ProgressView()
                            .controlSize(.large)
                        Text("Loading conversation…")
                            .foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }

                if switching, conversation != nil {
                    VStack {
                        HStack {
                            Spacer()
                            ProgressView()
                                .padding(10)
                                .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 8))
                                .padding()
                        }
                        Spacer()
                    }
                }
            }

            if !liveEvents.isEmpty {
                Divider()
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        ForEach(LiveEventGrouping.build(from: liveEvents)) { group in
                            if let toolEvent = group.toolEvent {
                                VStack(alignment: .leading, spacing: 8) {
                                    ToolCard(event: toolEvent)
                                    ForEach(Array(group.nested.enumerated()), id: \.offset) { _, nested in
                                        EventRow(event: nested)
                                            .padding(.leading, 16)
                                    }
                                }
                            } else if let event = group.standalone {
                                EventRow(event: event)
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding()
                }
                .frame(maxHeight: 220)
            }

            Divider()
            ComposerView()
        }
        .toolbar {
            ToolbarItem(placement: .principal) {
                if switching {
                    HStack(spacing: 8) {
                        ProgressView()
                            .controlSize(.small)
                        Text("Loading…")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            ToolbarItem(placement: .automatic) {
                Button {
                    store.showFilesPanel.toggle()
                } label: {
                    Label("Files", systemImage: "doc.text")
                }
                .help("Show working files")
                .disabled(store.selectedConversationId == nil)
            }
        }
        .inspector(isPresented: $store.showFilesPanel) {
            FilesSidebarView()
        }
        .onChange(of: store.showFilesPanel) { _, isPresented in
            if isPresented {
                Task { await store.refreshWorkdirFiles() }
            }
        }
    }

    private var liveEvents: [IdentifiedCoreEvent] {
        guard let selectedId = store.selectedConversationId else {
            return []
        }
        return store.liveEvents
            .enumerated()
            .filter { $0.element.conversationId == selectedId }
            .map { IdentifiedCoreEvent(id: $0.offset, event: $0.element) }
    }
}

struct ConversationHeader: View {
    @EnvironmentObject private var store: AppStore
    let conversation: ConversationRecord

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(conversation.title)
                    .font(.title2.bold())
                Spacer()
                Button {
                    store.showConversationRoots = true
                } label: {
                    Label("Roots", systemImage: "folder")
                }
                .labelStyle(.iconOnly)
                .help("Manage conversation roots")
                Button {
                    store.showForkConversation = true
                } label: {
                    Label("Fork Into", systemImage: "arrow.triangle.branch")
                }
                .labelStyle(.iconOnly)
                .help("Fork into another harness or model")
            }
            HStack {
                Text(conversation.harnessId ?? "No harness")
                Text(conversation.modelId ?? "No model")
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .accessibilityElement(children: .combine)
    }
}

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
            DisclosureGroup("Thinking") {
                Text(EventPayload.text(from: event.payloadJSON))
                    .textSelection(.enabled)
            }
            .padding(.vertical, 4)
        case "tool_call_started", "tool_call_progress", "file_changed":
            ToolCard(event: event)
        case "text_delta":
            Text(EventPayload.text(from: event.payloadJSON))
                .textSelection(.enabled)
        case "message_committed":
            MessageRow(
                conversationId: event.conversationId,
                message: ParsedTranscriptMessage(json: event.payloadJSON, fallbackIndex: 0)
            )
        case "permission_resolved":
            Text("Permission resolved")
                .font(.caption)
                .foregroundStyle(.secondary)
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

    var body: some View {
        CompactCard(title: title, systemImage: systemImage) {
            Text(JSONValue.from(json: event.payloadJSON)?.description ?? event.payloadJSON)
                .font(.body.monospaced())
                .textSelection(.enabled)
        }
        .accessibilityLabel(title)
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

struct MessageRow: View {
    let conversationId: String
    let message: ParsedTranscriptMessage

    var body: some View {
        if let rawJSON = message.rawJSON {
            Text(rawJSON)
                .font(.body.monospaced())
                .textSelection(.enabled)
        } else {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 6) {
                    Text(message.role.capitalized)
                        .font(.caption.bold())
                    if let harnessId = message.harnessId {
                        Text(harnessId)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                ForEach(TranscriptContentGrouping.build(from: message.content)) { group in
                    if let toolBlock = group.toolBlock {
                        VStack(alignment: .leading, spacing: 8) {
                            ContentBlockView(conversationId: conversationId, block: toolBlock)
                            ForEach(Array(group.nested.enumerated()), id: \.offset) { _, nested in
                                ContentBlockView(conversationId: conversationId, block: nested)
                                    .padding(.leading, 16)
                            }
                        }
                    } else if let standalone = group.standalone {
                        ContentBlockView(conversationId: conversationId, block: standalone)
                    }
                }
            }
            .padding(.vertical, 6)
            .accessibilityElement(children: .contain)
        }
    }
}

struct ContentBlockView: View {
    let conversationId: String
    let block: TranscriptContentBlock

    var body: some View {
        switch block.type {
        case "text":
            Text(block.text ?? "")
                .textSelection(.enabled)
        case "thinking":
            DisclosureGroup("Thinking", isExpanded: .constant(false)) {
                Text(block.text ?? "")
                    .textSelection(.enabled)
            }
        case "tool_call":
            CompactCard(title: block.name ?? "Tool call", systemImage: "wrench.and.screwdriver") {
                Text(block.inputSummary)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            }
        case "tool_result":
            CompactCard(title: "Tool result", systemImage: "checkmark.circle") {
                Text(block.outputSummary)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
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
            ArtifactCard(conversationId: conversationId, block: block)
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
                    Text("App offline — template unavailable without an active gateway run.")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("MCP App \(appTitle)")
        .accessibilityValue(template == nil ? "offline" : "loaded")
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
            if !state.isTerminal {
                Button("Cancel task") {
                    store.cancelTask(conversationId: conversationId, taskId: state.taskId)
                }
                .keyboardShortcut(.cancelAction)
            }
        }
        .padding(10)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Task \(taskTitle)")
        .accessibilityValue(accessibilityStatus)
        .accessibilityAddTraits(.updatesFrequently)
    }

    private var taskTitle: String { state.title ?? state.taskId }
    private var statusIcon: String {
        switch state.status {
        case "completed": "checkmark.circle"
        case "failed": "xmark.circle"
        default: "clock"
        }
    }
    private var accessibilityStatus: String {
        [state.status, state.progressMessage].compactMap { $0 }.joined(separator: ", ")
    }
}

struct TaskRefCard: View {
    let block: TranscriptContentBlock

    var body: some View {
        CompactCard(title: block.taskTitle ?? block.taskId ?? "Task", systemImage: "checklist") {
            Text("Status: \(block.taskStatus ?? "unknown")")
            if let summary = block.taskResultSummary, !summary.isEmpty {
                Text(summary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        }
        .accessibilityLabel("Task \(block.taskId ?? "unknown")")
        .accessibilityValue([block.taskStatus, block.taskResultSummary].compactMap { $0 }.joined(separator: ", "))
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
            Label("App action needs consent", systemImage: "hand.raised")
                .font(.headline)
            if let request {
                Text(request.summary)
                    .textSelection(.enabled)
                Text("Server: \(request.serverId) · App: \(request.appId)")
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

struct ArtifactCard: View {
    @EnvironmentObject private var store: AppStore
    let conversationId: String
    let block: TranscriptContentBlock
    @State private var verifiedInline: String?
    @State private var verifiedText: String?
    @State private var verifiedImage: NSImage?
    @State private var verifiedNonPreviewable = false
    @State private var loadError: String?

    var body: some View {
        CompactCard(title: block.path ?? "Artifact", systemImage: "doc.richtext") {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(block.mimeType ?? "artifact")
                    if let size = block.size {
                        Text("\(size) bytes")
                    }
                    if let sha256 = block.sha256 {
                        Text(String(sha256.prefix(8)))
                    }
                    Spacer()
                    Button {
                        revealAttachment()
                    } label: {
                        Label("Reveal in Finder", systemImage: "folder")
                    }
                    .labelStyle(.iconOnly)
                    .help("Reveal artifact in Finder")
                    .disabled(!hasVerifiedAttachmentMetadata)
                    Button {
                        openAttachment()
                    } label: {
                        Label("Open Artifact", systemImage: "arrow.up.right.square")
                    }
                    .labelStyle(.iconOnly)
                    .help("Open artifact in default app")
                    .disabled(!hasVerifiedAttachmentMetadata)
                }
                .font(.caption)
                .foregroundStyle(.secondary)
                if let verifiedInline {
                    preview(for: verifiedInline)
                } else if let verifiedImage {
                    Image(nsImage: verifiedImage)
                        .resizable()
                        .scaledToFit()
                        .frame(maxHeight: 360)
                } else if let verifiedText {
                    preview(for: verifiedText)
                } else if verifiedNonPreviewable {
                    TypedFileCard(path: block.path, mimeType: block.mimeType, size: block.size)
                } else if let loadError {
                    Text("Integrity check failed: \(loadError)")
                        .foregroundStyle(.red)
                } else if block.inline != nil || hasVerifiedAttachmentMetadata {
                    ProgressView()
                } else {
                    Text("Stored in attachments")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .accessibilityLabel(artifactAccessibilityLabel)
        .task(id: artifactVerificationKey) {
            await loadVerifiedContent()
        }
    }

    private var artifactVerificationKey: String {
        [block.path, block.sha256, block.inline].compactMap { $0 }.joined(separator: "|")
    }

    private var artifactAccessibilityLabel: String {
        let title = block.path ?? "Artifact"
        let type = block.mimeType ?? "unknown type"
        let size = block.size.map { "\($0) bytes" } ?? "unknown size"
        if loadError != nil {
            return "\(title), integrity check failed"
        }
        return "\(title), \(type), \(size)"
    }

    @ViewBuilder
    private func preview(for content: String) -> some View {
        switch block.mimeType {
        case "text/html", "image/svg+xml":
            SandboxedHTMLView(html: content, onBlockedNavigation: logBlockedNavigation)
                .frame(minHeight: 260)
        case "text/csv", "text/tab-separated-values":
            CSVPreview(text: content, separator: block.mimeType == "text/tab-separated-values" ? "\t" : ",")
        case "text/markdown":
            MarkdownPreview(content: content)
        default:
            Text(content)
                .font(.body.monospaced())
                .lineLimit(24)
                .textSelection(.enabled)
        }
    }

    private func logBlockedNavigation(_ url: URL) {
        store.logBlockedNavigation(url: url.absoluteString)
    }

    private func loadVerifiedContent() async {
        guard verifiedInline == nil,
              verifiedText == nil,
              verifiedImage == nil,
              !verifiedNonPreviewable,
              loadError == nil
        else {
            return
        }

        if let inline = block.inline, let size = block.size, let sha256 = block.sha256 {
            do {
                try await store.verifyArtifactInline(size: size, sha256: sha256, inline: inline)
                verifiedInline = inline
            } catch {
                loadError = error.localizedDescription
            }
            return
        }

        guard let path = block.path,
              let size = block.size,
              let sha256 = block.sha256
        else {
            return
        }
        do {
            let data = try await store.readAttachmentVerified(
                conversationId: conversationId,
                path: path,
                size: size,
                sha256: sha256
            )
            if artifactIsImageMime(block.mimeType), let image = NSImage(data: data) {
                verifiedImage = image
            } else if artifactIsTextLikeMime(block.mimeType), let text = String(data: data, encoding: .utf8) {
                verifiedText = text
            } else {
                verifiedNonPreviewable = true
            }
        } catch {
            loadError = error.localizedDescription
        }
    }

    private var hasVerifiedAttachmentMetadata: Bool {
        block.path != nil && block.size != nil && block.sha256 != nil
    }

    private func revealAttachment() {
        guard let path = block.path, let size = block.size, let sha256 = block.sha256 else { return }
        store.revealAttachment(conversationId: conversationId, path: path, size: size, sha256: sha256)
    }

    private func openAttachment() {
        guard let path = block.path, let size = block.size, let sha256 = block.sha256 else { return }
        store.openAttachment(conversationId: conversationId, path: path, size: size, sha256: sha256)
    }
}

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

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: artifactFileIcon(mimeType))
                .font(.title2)
                .foregroundStyle(.secondary)
                .frame(width: 28)
            VStack(alignment: .leading, spacing: 4) {
                Text((path as NSString?)?.lastPathComponent ?? "Attachment")
                    .font(.body.weight(.medium))
                    .lineLimit(1)
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
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
        if let size {
            let formatted = ByteCountFormatter.string(fromByteCount: Int64(size), countStyle: .file)
            return "\(type) · \(formatted)"
        }
        return type
    }

    private var accessibilityLabel: String {
        let name = (path as NSString?)?.lastPathComponent ?? "Attachment"
        let type = artifactMimeLabel(mimeType)
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
        if let attributed = try? AttributedString(markdown: content) {
            Text(attributed)
                .textSelection(.enabled)
        } else {
            Text(content)
                .textSelection(.enabled)
        }
    }
}

struct SandboxedHTMLView: NSViewRepresentable {
    let html: String
    var policy: WebContentPolicy = .artifactNoNetwork
    var bridgeContext: AppBridgeContext?
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
        webView.loadHTMLString(sandboxedHTML(for: html, policy: policy, bridgeScript: bridgeScript), baseURL: nil)
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

    private var rows: [[String]] {
        text
            .split(whereSeparator: \.isNewline)
            .prefix(20)
            .map { line in
                line.split(separator: separator, omittingEmptySubsequences: false)
                    .prefix(8)
                    .map(String.init)
            }
    }

    var body: some View {
        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                GridRow {
                    ForEach(Array(row.enumerated()), id: \.offset) { _, cell in
                        Text(cell)
                            .font(.caption.monospaced())
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
    private var summary: ToolSummary {
        ToolSummary(event: event)
    }

    var body: some View {
        CompactCard(title: summary.title, systemImage: "wrench.and.screwdriver") {
            if !summary.subtitle.isEmpty {
                Text(summary.subtitle)
                    .foregroundStyle(.secondary)
            }
            if !summary.detail.isEmpty {
                Text(summary.detail)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            }
        }
        .accessibilityLabel("Tool event")
    }
}

struct CompactCard<Content: View>: View {
    let title: String
    let systemImage: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(title, systemImage: systemImage)
                .font(.headline)
            content
        }
        .padding(10)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
    }
}

struct PermissionCard: View {
    @EnvironmentObject private var store: AppStore
    let event: CoreEvent
    private var request: PermissionPayload? {
        try? JSONDecoder().decode(PermissionPayload.self, from: Data(event.payloadJSON.utf8))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label("Permission requested", systemImage: "hand.raised")
                .font(.headline)
            if let request {
                if let harnessName = request.harnessDisplayName, !harnessName.isEmpty {
                    Text("From \(harnessName)")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                if let command = request.detail?.command, !command.isEmpty {
                    Text(command)
                        .font(.body.monospaced())
                        .textSelection(.enabled)
                } else if let diff = request.detail?.diff {
                    PermissionDiffView(diff: diff)
                } else if !request.summary.isEmpty {
                    Text(request.summary)
                        .textSelection(.enabled)
                }
            }
            HStack {
                if let request {
                    ForEach(request.rejectOptions) { option in
                        Button(option.label, role: .destructive) {
                            store.respondPermission(requestId: request.requestId, optionId: option.id)
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    ForEach(request.allowOptions) { option in
                        Button(option.label) {
                            store.respondPermission(requestId: request.requestId, optionId: option.id)
                        }
                        .buttonStyle(.bordered)
                    }
                    .keyboardShortcut(.defaultAction)
                } else {
                    Text("Unable to parse permission request")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(10)
        .background(.yellow.opacity(0.18), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityLabel("Permission requested")
    }
}

private struct PermissionDiffView: View {
    let diff: DiffPayload

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let path = diff.path, !path.isEmpty {
                Text(path)
                    .font(.subheadline.bold())
            }
            if let change = diff.change, !change.isEmpty {
                Text(change.capitalized)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if let oldText = diff.oldText, !oldText.isEmpty {
                Text("Before")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(oldText)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            }
            if let newText = diff.newText, !newText.isEmpty {
                Text("After")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(newText)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            }
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
        if let request, request.mode == "url" {
            URLConsentCard(
                request: URLConsentRequest(
                    requestId: request.requestId,
                    serverId: request.serverId,
                    originToolCallId: request.originToolCallId,
                    message: request.message,
                    url: request.url
                )
            )
        } else if ElicitationSchemaPolicy.schemaLooksSecret(request?.schema) {
            SecretElicitationBlockedCard(
                onDecline: { respond(action: "decline") },
                onCancel: { respond(action: "cancel") }
            )
        } else if !ElicitationSchemaPolicy.schemaIsRenderable(request?.schema) {
            UnsupportedElicitationSchemaCard(
                message: request?.message ?? "The server sent a form tamtri cannot render.",
                onDecline: { respond(action: "decline") },
                onCancel: { respond(action: "cancel") }
            )
        } else {
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

private struct PermissionPayload: Decodable {
    let requestId: String
    let action: String
    let options: [PermissionOptionPayload]
    let detail: PermissionDetailPayload?
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

    var rejectOptions: [PermissionOptionPayload] {
        options.filter { $0.isReject }
    }

    var allowOptions: [PermissionOptionPayload] {
        options.filter { !$0.isReject }
    }
}

private struct PermissionDetailPayload: Decodable {
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

private struct DiffPayload: Decodable {
    let path: String?
    let change: String?
    let oldText: String?
    let newText: String?

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

private struct PermissionOptionPayload: Decodable, Identifiable {
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

    static func string(from json: String, key: String) -> String? {
        JSONValue.from(json: json)?.string(at: key)
    }
}

private struct ToolSummary {
    let title: String
    let subtitle: String
    let detail: String

    init(event: CoreEvent) {
        let json = JSONValue.from(json: event.payloadJSON)
        let name = json?.string(at: "name") ?? json?.string(at: "title")
        let path = json?.string(at: "path") ?? json?.value(at: "diff")?.string(at: "path")
        let status = json?.string(at: "status")
        self.title = name ?? event.kind.replacingOccurrences(of: "_", with: " ").capitalized
        self.subtitle = [status, path].compactMap { $0 }.joined(separator: " • ")
        self.detail = json?.description ?? event.payloadJSON
    }
}

struct FilesSidebarView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Files")
                    .font(.headline)
                Spacer()
                Button {
                    store.revealWorkdir()
                } label: {
                    Label("Open folder", systemImage: "folder")
                }
                .labelStyle(.iconOnly)
                .help("Reveal workdir in Finder")
            }
            .padding(.horizontal)
            .padding(.top, 12)
            .padding(.bottom, 8)

            if store.transcriptArtifacts.isEmpty && store.workdirFiles.isEmpty {
                Spacer()
                VStack(spacing: 8) {
                    Image(systemName: "doc")
                        .font(.title2)
                        .foregroundStyle(.secondary)
                    Text("No files yet")
                        .font(.subheadline.weight(.medium))
                    Text("Drop files into the composer or ask the agent to create them.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                }
                .padding()
                Spacer()
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        if !store.transcriptArtifacts.isEmpty {
                            Text("Artifacts")
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(.secondary)
                                .padding(.horizontal)
                                .padding(.bottom, 4)
                            List(store.transcriptArtifacts, selection: selectedArtifactBinding) { artifact in
                                TranscriptArtifactRow(artifact: artifact)
                                    .tag(artifact.id)
                            }
                            .listStyle(.sidebar)
                            .frame(minHeight: 80, maxHeight: 160)
                        }
                        if !store.workdirFiles.isEmpty {
                            Text("Working files")
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(.secondary)
                                .padding(.horizontal)
                                .padding(.top, store.transcriptArtifacts.isEmpty ? 0 : 8)
                                .padding(.bottom, 4)
                            List(store.workdirFiles, selection: selectedFileBinding) { file in
                                FilesSidebarRow(file: file)
                                    .tag(file.relativePath)
                            }
                            .listStyle(.sidebar)
                            .frame(minHeight: 80, maxHeight: 160)
                        }
                    }
                }
                .frame(maxHeight: 220)

                Divider()

                if let preview = store.attachmentPreview {
                    AttachmentFilePreviewView(preview: preview)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if let preview = store.workdirPreview {
                    WorkdirFilePreviewView(preview: preview)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if store.selectedTranscriptArtifact != nil || store.selectedWorkdirFile != nil {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
        }
        .inspectorColumnWidth(min: 260, ideal: 340, max: 520)
        .accessibilityLabel("Working files and artifacts")
    }

    private var selectedArtifactBinding: Binding<String?> {
        Binding(
            get: { store.selectedTranscriptArtifact?.id },
            set: { id in
                guard let id,
                      let artifact = store.transcriptArtifacts.first(where: { $0.id == id })
                else { return }
                Task { await store.selectTranscriptArtifact(artifact) }
            }
        )
    }

    private var selectedFileBinding: Binding<String?> {
        Binding(
            get: { store.selectedWorkdirFile?.relativePath },
            set: { path in
                guard let path,
                      let file = store.workdirFiles.first(where: { $0.relativePath == path })
                else { return }
                Task { await store.selectWorkdirFile(file) }
            }
        )
    }
}

struct TranscriptArtifactRow: View {
    let artifact: TranscriptArtifactRecord

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: artifactFileIcon(artifact.mimeType))
                .foregroundStyle(.secondary)
                .frame(width: 18)
            VStack(alignment: .leading, spacing: 2) {
                Text((artifact.path as NSString).lastPathComponent)
                    .lineLimit(1)
                Text("Attachment · \(ByteCountFormatter.string(fromByteCount: Int64(artifact.size), countStyle: .file))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .accessibilityLabel("\((artifact.path as NSString).lastPathComponent), frozen attachment")
    }
}

struct AttachmentFilePreviewView: View {
    @EnvironmentObject private var store: AppStore
    let preview: AttachmentFilePreview

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text((preview.path as NSString).lastPathComponent)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)
                Spacer()
                if let artifact = store.selectedTranscriptArtifact {
                    Button {
                        store.revealTranscriptArtifact(artifact)
                    } label: {
                        Label("Reveal in Finder", systemImage: "folder")
                    }
                    .labelStyle(.iconOnly)
                    .help("Reveal frozen attachment in Finder")
                }
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            ScrollView {
                Group {
                    if let error = preview.error {
                        Text(error)
                            .foregroundStyle(.secondary)
                            .padding()
                    } else if let text = preview.text {
                        attachmentTextPreview(text)
                    } else if let imageData = preview.imageData, let image = NSImage(data: imageData) {
                        Image(nsImage: image)
                            .resizable()
                            .scaledToFit()
                            .frame(maxWidth: .infinity)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal)
                .padding(.bottom, 12)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    @ViewBuilder
    private func attachmentTextPreview(_ text: String) -> some View {
        switch preview.mimeType {
        case "text/html", "image/svg+xml":
            SandboxedHTMLView(html: text, onBlockedNavigation: { url in
                store.logBlockedNavigation(url: url.absoluteString)
            })
                .frame(minHeight: 280)
        case "text/csv", "text/tab-separated-values":
            CSVPreview(text: text, separator: preview.mimeType == "text/tab-separated-values" ? "\t" : ",")
        case "text/markdown":
            Text(text)
                .textSelection(.enabled)
        default:
            Text(text)
                .font(.body.monospaced())
                .textSelection(.enabled)
        }
    }
}

struct FilesSidebarRow: View {
    let file: WorkdirFileRecord

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: fileIcon)
                .foregroundStyle(.secondary)
                .frame(width: 18)
            VStack(alignment: .leading, spacing: 2) {
                Text(file.relativePath)
                    .lineLimit(1)
                Text(fileSubtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .accessibilityLabel("\(file.relativePath), \(fileSubtitle)")
    }

    private var fileIcon: String {
        switch file.mimeType {
        case "text/html": "doc.richtext"
        case "text/csv", "text/tab-separated-values": "tablecells"
        case "text/markdown": "text.quote"
        case "image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml": "photo"
        default: "doc"
        }
    }

    private var fileSubtitle: String {
        let size = ByteCountFormatter.string(fromByteCount: Int64(file.size), countStyle: .file)
        if let mimeType = file.mimeType {
            return "\(mimeLabel(mimeType)) · \(size)"
        }
        return size
    }

    private func mimeLabel(_ mimeType: String) -> String {
        switch mimeType {
        case "text/html": "HTML"
        case "text/csv": "CSV"
        case "text/tab-separated-values": "TSV"
        case "text/markdown": "Markdown"
        case "text/plain": "Text"
        case "application/json": "JSON"
        case "image/png": "PNG"
        case "image/jpeg": "JPEG"
        case "image/gif": "GIF"
        case "image/webp": "WebP"
        case "image/svg+xml": "SVG"
        default: mimeType
        }
    }
}

struct WorkdirFilePreviewView: View {
    @EnvironmentObject private var store: AppStore
    let preview: WorkdirFilePreview

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(preview.relativePath)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)
                Spacer()
                Button {
                    if let file = store.selectedWorkdirFile {
                        store.revealWorkdirFile(file)
                    }
                } label: {
                    Label("Reveal in Finder", systemImage: "folder")
                }
                .labelStyle(.iconOnly)
                .help("Reveal in Finder")
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            ScrollView {
                Group {
                    if let error = preview.error {
                        Text(error)
                            .foregroundStyle(.secondary)
                            .padding()
                    } else if let text = preview.text {
                        workdirTextPreview(text)
                    } else if let imageData = preview.imageData, let image = NSImage(data: imageData) {
                        Image(nsImage: image)
                            .resizable()
                            .scaledToFit()
                            .frame(maxWidth: .infinity)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal)
                .padding(.bottom, 12)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    @ViewBuilder
    private func workdirTextPreview(_ text: String) -> some View {
        switch preview.mimeType {
        case "text/html", "image/svg+xml":
            SandboxedHTMLView(html: text, onBlockedNavigation: { url in
                store.logBlockedNavigation(url: url.absoluteString)
            })
                .frame(minHeight: 280)
        case "text/csv", "text/tab-separated-values":
            CSVPreview(text: text, separator: preview.mimeType == "text/tab-separated-values" ? "\t" : ",")
        case "text/markdown":
            MarkdownPreview(content: text)
        default:
            Text(text)
                .font(.body.monospaced())
                .textSelection(.enabled)
        }
    }
}

struct ComposerView: View {
    @EnvironmentObject private var store: AppStore
    @State private var isDropTargeted = false

    var body: some View {
        HStack(alignment: .bottom, spacing: 8) {
            Button {
                store.showConversationRoots = true
            } label: {
                Label("Attach Root", systemImage: "folder.badge.plus")
            }
            .labelStyle(.iconOnly)
            .help("Attach a filesystem root for this conversation")
            .disabled(store.selectedConversation == nil)
            TextField("Message", text: $store.composerText, axis: .vertical)
                .textFieldStyle(.roundedBorder)
                .lineLimit(1...5)
                .onDrop(of: [.fileURL], isTargeted: nil, perform: handleFileDrop)
            Button {
                store.send()
            } label: {
                Label("Send", systemImage: "paperplane.fill")
            }
            .disabled(store.selectedConversation == nil)
        }
        .padding()
        .background(isDropTargeted ? Color.accentColor.opacity(0.12) : Color.clear)
        .onDrop(of: [.fileURL], isTargeted: $isDropTargeted, perform: handleFileDrop)
    }

    private func handleFileDrop(_ providers: [NSItemProvider]) -> Bool {
        let group = DispatchGroup()
        let collector = DropFileCollector()
        for provider in providers {
            group.enter()
            provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, _ in
                defer { group.leave() }
                let path: String?
                if let data = item as? Data,
                   let url = URL(dataRepresentation: data, relativeTo: nil),
                   url.isFileURL {
                    path = url.path
                } else if let url = item as? URL, url.isFileURL {
                    path = url.path
                } else {
                    path = nil
                }
                if let path {
                    collector.append(path)
                }
            }
        }
        group.notify(queue: .main) {
            store.attachFiles(paths: collector.paths())
        }
        return true
    }
}

final class DropFileCollector: @unchecked Sendable {
    private let lock = NSLock()
    private var collected: [String] = []

    func append(_ path: String) {
        lock.lock()
        collected.append(path)
        lock.unlock()
    }

    func paths() -> [String] {
        lock.lock()
        let copy = collected
        lock.unlock()
        return copy
    }
}

struct NewConversationView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @State private var title = ""
    @State private var harnessId = defaultHarnessId()
    @State private var modelId = defaultModelId()

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("New Conversation")
                .font(.title2.bold())
            TextField("Title", text: $title)
            HarnessPicker(harnessId: $harnessId, agents: store.harnessAgents)
            TextField("Model", text: $modelId)
            HStack {
                Spacer()
                Button("Cancel") {
                    dismiss()
                }
                Button("Create") {
                    store.createConversation(title: title.isEmpty ? "Untitled" : title, harnessId: harnessId, modelId: modelId)
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 360)
        .task {
            await store.refreshHarnessAgents()
            if !store.harnessAgents.contains(where: { $0.id == harnessId }),
               let first = store.harnessAgents.first {
                harnessId = first.id
            }
        }
    }
}

struct HarnessPicker: View {
    @Binding var harnessId: String
    let agents: [HarnessAgentRecord]

    var body: some View {
        if agents.isEmpty {
            TextField("Harness", text: $harnessId)
        } else {
            Picker("Harness", selection: $harnessId) {
                ForEach(agents) { agent in
                    Text(agent.displayName).tag(agent.id)
                }
            }
        }
    }
}

struct ForkConversationView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @State private var harnessId = defaultHarnessId()
    @State private var modelId = defaultModelId()

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Fork Into")
                .font(.title2.bold())
            if let conversation = store.selectedConversation {
                Text(conversation.title)
                    .foregroundStyle(.secondary)
                if let forkedFrom = conversation.forkedFrom {
                    Text("Fork lineage: branched from \(forkedFrom)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            HarnessPicker(harnessId: $harnessId, agents: store.harnessAgents)
            TextField("Model", text: $modelId)
            HStack {
                Spacer()
                Button("Cancel") {
                    dismiss()
                }
                Button("Fork") {
                    store.forkSelectedConversation(harnessId: harnessId, modelId: modelId)
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 360)
        .task {
            await store.refreshHarnessAgents()
            if !store.harnessAgents.contains(where: { $0.id == harnessId }),
               let first = store.harnessAgents.first {
                harnessId = first.id
            }
        }
    }
}

struct SettingsView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @State private var showAddServer = false
    @State private var serverToEdit: GatewayServerRecord?
    @State private var serverToRemove: GatewayServerRecord?
    @State private var pendingRemoveServerId: String?
    @State private var timeoutDraft = ""

    private var vaultConfigPath: String {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".tamtri/vault/config.json")
            .path
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Settings")
                    .font(.title2.bold())
                Spacer()
                Button {
                    store.refreshGatewayCapabilities()
                } label: {
                    Label("Probe capabilities", systemImage: "antenna.radiowaves.left.and.right")
                }
                Button {
                    Task { await store.refreshGatewayServers() }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                Button("Done") {
                    dismiss()
                }
            }

            Text("Tamtri gateway tools")
                .font(.headline)

            HStack {
                Text("Default call timeout (seconds)")
                TextField("300", text: $timeoutDraft)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 80)
                Button("Save") {
                    if let seconds = UInt64(timeoutDraft.trimmingCharacters(in: .whitespacesAndNewlines)) {
                        store.saveGatewayDefaultTimeout(seconds)
                    }
                }
            }
            .font(.caption)

            if store.gatewayTools.isEmpty {
                Text("No gateway tools loaded. Add servers and tap Refresh.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(store.gatewayTools) { tool in
                        Text("\(tool.exposedName) ← \(tool.serverId)/\(tool.originalName)")
                            .font(.caption.monospaced())
                    }
                }
            }

            Text("Servers are stored in \(vaultConfigPath). Edits here and in an external editor both update the same file; use Refresh after external changes.")
                .font(.caption)
                .foregroundStyle(.secondary)

            if store.gatewayServers.isEmpty {
                VStack(spacing: 12) {
                    Text("No gateway servers configured yet.")
                        .foregroundStyle(.secondary)
                    Button("Add MCP server") {
                        showAddServer = true
                    }
                    .buttonStyle(.borderedProminent)
                }
                .frame(maxWidth: .infinity, minHeight: 180)
            } else {
                List {
                    ForEach(store.gatewayServers) { server in
                        GatewayServerRow(
                            server: server,
                            onEdit: { serverToEdit = server },
                            onRemove: {
                                pendingRemoveServerId = server.id
                                serverToRemove = server
                            }
                        )
                    }
                }
                .frame(minHeight: 220)
            }

            HStack {
                Button("Add MCP server") {
                    showAddServer = true
                }
                Spacer()
                Link("20 Questions testing guide", destination: URL(string: "https://github.com/im2nguyen/tamtri/blob/main/docs/testing/twenty-questions.md")!)
                    .font(.caption)
            }

            Text("Agent-native tools are not exposed by this harness yet.")
                .font(.caption)
                .foregroundStyle(.secondary)

            Text("Capability badges: supported (green) means tamtri and the server both wire the feature. server only (orange) means the downstream server advertises it but tamtri has not enabled it yet. Sampling is always declined — the harness owns the model.")
                .font(.caption)
                .foregroundStyle(.secondary)

            Link("Apps, Tasks, and Roots guides", destination: URL(string: "https://github.com/im2nguyen/tamtri/blob/main/docs/testing/README.md")!)
                .font(.caption)
        }
        .padding()
        .frame(width: 560, height: 560)
        .onAppear {
            timeoutDraft = String(store.defaultCallTimeoutSecs)
            Task { await store.refreshGatewayServers() }
        }
        .sheet(isPresented: $showAddServer) {
            GatewayServerEditorSheet(
                mode: .add,
                existingServers: store.gatewayServers,
                onSave: store.saveGatewayServers
            )
        }
        .sheet(item: $serverToEdit) { server in
            GatewayServerEditorSheet(
                mode: .edit(server),
                existingServers: store.gatewayServers,
                onSave: store.saveGatewayServers
            )
        }
        .confirmationDialog(
            "Remove \(serverToRemove?.displayName ?? "server")?",
            isPresented: Binding(
                get: { serverToRemove != nil },
                set: { if !$0 { serverToRemove = nil } }
            ),
            titleVisibility: .visible
        ) {
            Button("Remove", role: .destructive) {
                if let pendingRemoveServerId {
                    store.removeGatewayServer(id: pendingRemoveServerId)
                }
                serverToRemove = nil
                self.pendingRemoveServerId = nil
            }
            Button("Cancel", role: .cancel) {
                serverToRemove = nil
                pendingRemoveServerId = nil
            }
        } message: {
            Text("This removes the server from config.json. Credential bindings for this server are removed too.")
        }
    }
}

struct GatewayCapabilityBadges: View {
    let server: GatewayServerRecord

    private let features: [(label: String, keyPath: KeyPath<GatewayServerRecord, String>)] = [
        ("Tools", \.capTools),
        ("Resources", \.capResources),
        ("Prompts", \.capPrompts),
        ("Elicitation", \.capElicitation),
        ("Apps", \.capApps),
        ("Tasks", \.capTasks),
        ("Roots", \.capRoots),
        ("Sampling", \.capSampling),
    ]

    var body: some View {
        FlowLayout(spacing: 6) {
            ForEach(features, id: \.label) { feature in
                CapabilityBadge(title: feature.label, status: server[keyPath: feature.keyPath])
            }
        }
    }
}

private struct CapabilityBadge: View {
    let title: String
    let status: String

    var body: some View {
        Text("\(title): \(displayStatus)")
            .font(.caption2)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(backgroundColor.opacity(0.15), in: Capsule())
            .foregroundStyle(foregroundColor)
            .accessibilityLabel(accessibilityText)
            .help(helpText)
    }

    private var effectiveStatus: String {
        if title == "Sampling" { "declined" }
        else { status }
    }

    private var displayStatus: String {
        effectiveStatus.replacingOccurrences(of: "_", with: " ")
    }

    private var accessibilityText: String {
        if title == "Sampling" {
            "Sampling declined by design. The model lives in the harness."
        } else {
            "\(title) \(displayStatus)"
        }
    }

    private var helpText: String {
        if title == "Sampling" {
            "tamtri declines MCP sampling; the harness owns the model."
        } else {
            ""
        }
    }

    private var foregroundColor: Color {
        switch effectiveStatus {
        case "supported": .green
        case "server_only": .orange
        case "declined": .secondary
        case "unknown": .secondary
        default: .secondary
        }
    }

    private var backgroundColor: Color {
        foregroundColor
    }
}

/// Simple horizontal flow for capability chips.
private struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let result = arrange(proposal: proposal, subviews: subviews)
        return result.size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let result = arrange(proposal: proposal, subviews: subviews)
        for (index, position) in result.positions.enumerated() {
            subviews[index].place(
                at: CGPoint(x: bounds.minX + position.x, y: bounds.minY + position.y),
                proposal: .unspecified
            )
        }
    }

    private func arrange(proposal: ProposedViewSize, subviews: Subviews) -> (size: CGSize, positions: [CGPoint]) {
        let maxWidth = proposal.width ?? .infinity
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowHeight: CGFloat = 0
        var positions: [CGPoint] = []

        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x + size.width > maxWidth, x > 0 {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            positions.append(CGPoint(x: x, y: y))
            rowHeight = max(rowHeight, size.height)
            x += size.width + spacing
        }

        return (CGSize(width: maxWidth, height: y + rowHeight), positions)
    }
}

struct GatewayServerRow: View {
    @EnvironmentObject private var store: AppStore
    let server: GatewayServerRecord
    let onEdit: () -> Void
    let onRemove: () -> Void
    @State private var credentialValues: [String: String] = [:]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: server.enabled ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(server.enabled ? .green : .secondary)
                Text(server.displayName)
                    .font(.headline)
                Spacer()
                Text(server.scope)
                Text(server.transport)
                Button("Edit", action: onEdit)
                Button("Remove", role: .destructive, action: onRemove)
            }
            .font(.caption)
            if server.transport == "stdio", !server.stdioCommand.isEmpty {
                Text(server.stdioCommand)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            HStack(spacing: 8) {
                connectionStatusIcon(for: server.connectionStatus)
                Text("Connection: \(server.connectionStatus.replacingOccurrences(of: "_", with: " "))")
                if let timeout = server.timeoutSecs {
                    Text("Timeout: \(timeout)s")
                }
            }
            .font(.caption)
            if !server.lastError.isEmpty {
                Text(server.lastError)
                    .font(.caption2)
                    .foregroundStyle(.red)
                    .lineLimit(2)
            }
            GatewayCapabilityBadges(server: server)
            if server.credentialRefs.isEmpty && server.oauthTokenRef.isEmpty {
                Text("No credentials required")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                if !server.oauthTokenRef.isEmpty {
                    HStack {
                        oauthStatusIcon(for: server.oauthStatus)
                        Text("OAuth: \(server.oauthStatus.replacingOccurrences(of: "_", with: " "))")
                        Spacer()
                        if server.oauthStatus == "connected" {
                            Text("Connected").foregroundStyle(.green)
                        } else {
                            Button("Connect") {
                                store.connectOAuth(for: server)
                            }
                        }
                    }
                    .font(.caption)
                }
                ForEach(server.credentialRefs, id: \.self) { credentialRef in
                    HStack {
                        Image(systemName: server.missingCredentialRefs.contains(credentialRef) ? "key.slash" : "key.fill")
                        Text(credentialRef)
                            .lineLimit(1)
                        SecureField("Value", text: binding(for: credentialRef))
                        Button("Save") {
                            store.setGatewayCredential(
                                credentialRef: credentialRef,
                                value: credentialValues[credentialRef, default: ""]
                            )
                            credentialValues[credentialRef] = ""
                        }
                        .disabled(credentialValues[credentialRef, default: ""].isEmpty)
                    }
                    .font(.caption)
                }
            }
        }
        .padding(.vertical, 6)
    }

    private func binding(for credentialRef: String) -> Binding<String> {
        Binding(
            get: { credentialValues[credentialRef, default: ""] },
            set: { credentialValues[credentialRef] = $0 }
        )
    }

    @ViewBuilder
    private func connectionStatusIcon(for status: String) -> some View {
        switch status {
        case "connected":
            Image(systemName: "circle.fill").foregroundStyle(.green)
        case "error":
            Image(systemName: "exclamationmark.circle.fill").foregroundStyle(.red)
        case "disabled":
            Image(systemName: "minus.circle").foregroundStyle(.secondary)
        default:
            Image(systemName: "questionmark.circle").foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private func oauthStatusIcon(for status: String) -> some View {
        switch status {
        case "connected":
            Image(systemName: "checkmark.seal.fill").foregroundStyle(.green)
        case "reauth_required", "expired":
            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
        case "missing":
            Image(systemName: "key.slash").foregroundStyle(.secondary)
        default:
            Image(systemName: "key").foregroundStyle(.secondary)
        }
    }
}
