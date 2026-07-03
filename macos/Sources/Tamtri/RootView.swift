import Foundation
import SwiftUI

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
        List(store.conversations, selection: selectedBinding) { conversation in
            Button {
                store.select(conversation)
            } label: {
                VStack(alignment: .leading, spacing: 2) {
                    Text(conversation.title)
                        .font(.headline)
                    Text(conversation.updatedAt)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .buttonStyle(.plain)
        }
        .navigationTitle("tamtri")
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

    private var selectedBinding: Binding<String?> {
        Binding(
            get: { store.selectedConversation?.id },
            set: { _ in }
        )
    }
}

struct TranscriptView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        VStack(spacing: 0) {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 12) {
                    if let conversation = store.selectedConversation {
                        ConversationHeader(conversation: conversation)
                        ForEach(Array(conversation.messagesJSON.enumerated()), id: \.offset) { _, messageJSON in
                            MessageRow(messageJSON: messageJSON)
                        }
                    }
                    ForEach(Array(liveEvents.enumerated()), id: \.offset) { _, event in
                        EventRow(event: event)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
            }
            Divider()
            ComposerView()
        }
    }

    private var liveEvents: [CoreEvent] {
        guard let selectedId = store.selectedConversation?.id else {
            return []
        }
        return store.liveEvents.filter { $0.conversationId == selectedId }
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
            MessageRow(messageJSON: event.payloadJSON)
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
    let messageJSON: String

    private var message: TranscriptMessage? {
        try? JSONDecoder().decode(TranscriptMessage.self, from: Data(messageJSON.utf8))
    }

    var body: some View {
        if let message {
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
                    ContentBlockView(block: block)
                }
            }
            .padding(.vertical, 6)
            .accessibilityElement(children: .contain)
        } else {
            Text(messageJSON)
                .font(.body.monospaced())
                .textSelection(.enabled)
        }
    }
}

struct ContentBlockView: View {
    fileprivate let block: TranscriptContentBlock

    var body: some View {
        switch block.type {
        case "text":
            Text(block.text ?? "")
                .textSelection(.enabled)
        case "thinking":
            DisclosureGroup("Thinking") {
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
            CompactCard(title: block.path ?? "Artifact", systemImage: "doc.richtext") {
                if let inline = block.inline {
                    Text(inline)
                        .textSelection(.enabled)
                } else {
                    Text(block.mimeType ?? "artifact")
                        .foregroundStyle(.secondary)
                }
            }
        default:
            EmptyView()
        }
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

private struct TranscriptMessage: Decodable {
    let role: String
    let harnessId: String?
    let content: [TranscriptContentBlock]

    enum CodingKeys: String, CodingKey {
        case role
        case harnessId = "harness_id"
        case content
    }
}

private struct TranscriptContentBlock: Decodable {
    let type: String
    let text: String?
    let name: String?
    let input: JSONValue?
    let callId: String?
    let output: JSONValue?
    let path: String?
    let mimeType: String?
    let inline: String?

    enum CodingKeys: String, CodingKey {
        case type
        case text
        case name
        case input
        case callId = "call_id"
        case output
        case path
        case mimeType = "mime_type"
        case inline
    }

    var inputSummary: String {
        input?.description ?? ""
    }

    var outputSummary: String {
        output?.description ?? ""
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

private enum JSONValue: Decodable, CustomStringConvertible {
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
}

struct ComposerView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        HStack(alignment: .bottom, spacing: 8) {
            TextField("Message", text: $store.composerText, axis: .vertical)
                .textFieldStyle(.roundedBorder)
                .lineLimit(1...5)
            Button {
                store.send()
            } label: {
                Label("Send", systemImage: "paperplane.fill")
            }
            .disabled(store.selectedConversation == nil)
        }
        .padding()
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
