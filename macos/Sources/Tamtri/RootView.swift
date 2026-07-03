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
                store.refreshGatewayServers()
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
                        WebTranscriptView(transcriptJSON: conversation.transcriptJSON)
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
                        ForEach(liveEvents) { item in
                            EventRow(event: item.event)
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

private struct IdentifiedCoreEvent: Identifiable {
    let id: Int
    let event: CoreEvent
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
        case "gateway_server_connected", "gateway_tool_routed", "gateway_progress", "gateway_log", "gateway_cancellation", "gateway_credential_injected", "gateway_downstream_error":
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
                ForEach(Array(message.content.enumerated()), id: \.offset) { _, block in
                    ContentBlockView(conversationId: conversationId, block: block)
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
        case "artifact":
            // Frozen attachments stay in messages.jsonl for replay/export;
            // live previews belong in the Files inspector, not the transcript.
            EmptyView()
        default:
            EmptyView()
        }
    }
}

struct ArtifactCard: View {
    @EnvironmentObject private var store: AppStore
    let conversationId: String
    let block: TranscriptContentBlock
    @State private var verifiedInline: String?
    @State private var verifiedText: String?
    @State private var verifiedImage: NSImage?
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
            Text(content)
                .textSelection(.enabled)
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

struct SandboxedHTMLView: NSViewRepresentable {
    let html: String
    var onBlockedNavigation: ((URL) -> Void)?

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = context.coordinator
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        context.coordinator.onBlockedNavigation = onBlockedNavigation
        webView.loadHTMLString(artifactSandboxedHTML(html), baseURL: nil)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(onBlockedNavigation: onBlockedNavigation)
    }

    final class Coordinator: NSObject, WKNavigationDelegate {
        var onBlockedNavigation: ((URL) -> Void)?

        init(onBlockedNavigation: ((URL) -> Void)?) {
            self.onBlockedNavigation = onBlockedNavigation
        }

        func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void) {
            guard let url = navigationAction.request.url else {
                decisionHandler(.allow)
                return
            }
            if ArtifactNavigationPolicy.allows(url) {
                decisionHandler(.allow)
            } else {
                onBlockedNavigation?(url)
                decisionHandler(.cancel)
            }
        }
    }
}

struct ArtifactNavigationPolicy {
    static func allows(_ url: URL?) -> Bool {
        guard let url else { return true }
        return url.scheme == "about" || url.scheme == nil
    }
}

func artifactSandboxedHTML(_ html: String) -> String {
    let csp = "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; base-uri 'none'; form-action 'none'\">"
    if let headRange = html.range(of: "<head", options: [.caseInsensitive]),
       let close = html[headRange.upperBound...].firstIndex(of: ">") {
        var copy = html
        copy.insert(contentsOf: csp, at: html.index(after: close))
        return copy
    }
    return "<!doctype html><html><head>\(csp)</head><body>\(html)</body></html>"
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
                Text(request.summary)
                    .textSelection(.enabled)
            }
            HStack {
                if let request {
                    ForEach(request.rejectOptions) { option in
                        Button(option.label) {
                            store.respondPermission(requestId: request.requestId, optionId: option.id)
                        }
                    }
                    ForEach(request.allowOptions) { option in
                        Button(option.label) {
                            store.respondPermission(requestId: request.requestId, optionId: option.id)
                        }
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

private struct PermissionPayload: Decodable {
    let requestId: String
    let action: String
    let options: [PermissionOptionPayload]
    let detail: PermissionDetailPayload?

    enum CodingKeys: String, CodingKey {
        case requestId = "request_id"
        case action
        case options
        case detail
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

            if store.workdirFiles.isEmpty {
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
                List(store.workdirFiles, selection: selectedFileBinding) { file in
                    FilesSidebarRow(file: file)
                        .tag(file.relativePath)
                }
                .listStyle(.sidebar)
                .frame(minHeight: 100, maxHeight: 220)

                Divider()

                if let preview = store.workdirPreview {
                    WorkdirFilePreviewView(preview: preview)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if store.selectedWorkdirFile != nil {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
        }
        .inspectorColumnWidth(min: 260, ideal: 340, max: 520)
        .accessibilityLabel("Working files")
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
            Text(text)
                .textSelection(.enabled)
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
            TextField("Harness", text: $harnessId)
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
            }
            TextField("Harness", text: $harnessId)
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
    }
}

struct SettingsView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Settings")
                    .font(.title2.bold())
                Spacer()
                Button("Done") {
                    dismiss()
                }
            }
            Text("Tamtri gateway tools")
                .font(.headline)
            if store.gatewayServers.isEmpty {
                Text("No gateway servers are configured in config.json.")
                    .foregroundStyle(.secondary)
            } else {
                List(store.gatewayServers) { server in
                    GatewayServerRow(server: server)
                }
                .frame(minHeight: 220)
            }
            Text("Agent-native tools are not exposed by this harness yet.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(width: 520, height: 380)
        .onAppear {
            store.refreshGatewayServers()
        }
    }
}

struct GatewayServerRow: View {
    @EnvironmentObject private var store: AppStore
    let server: GatewayServerRecord
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
            }
            .font(.caption)
            if server.credentialRefs.isEmpty {
                Text("No credentials required")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
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
}
