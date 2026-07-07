import SwiftUI
import WebKit
import AppKit
import Foundation

struct NewConversationView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @State private var title = ""
    @State private var harnessId = defaultHarnessId()
    @State private var modelId = defaultModelId()
    @State private var availableModels: [ModelInfoRecord] = []

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("New Conversation")
                .font(.title2.bold())
            TextField("Title", text: $title)
            HarnessPicker(harnessId: $harnessId, agents: store.harnessAgents)
            ModelPicker(modelId: $modelId, models: availableModels)
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
            await refreshModels()
        }
        .onChange(of: harnessId) { _, _ in
            Task { await refreshModels() }
        }
    }

    private func refreshModels() async {
        availableModels = await store.listAgentModels(agentId: harnessId)
        if let first = availableModels.first,
           !availableModels.contains(where: { $0.id == modelId }) {
            modelId = first.id
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

struct ModelPicker: View {
    @Binding var modelId: String
    let models: [ModelInfoRecord]

    var body: some View {
        if models.isEmpty {
            TextField("Model", text: $modelId)
        } else {
            Picker("Model", selection: $modelId) {
                ForEach(models) { model in
                    Text(model.displayName).tag(model.id)
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
    @State private var availableModels: [ModelInfoRecord] = []

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
            ModelPicker(modelId: $modelId, models: availableModels)
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
            await refreshModels()
        }
        .onChange(of: harnessId) { _, _ in
            Task { await refreshModels() }
        }
    }

    private func refreshModels() async {
        availableModels = await store.listAgentModels(agentId: harnessId)
        if let first = availableModels.first,
           !availableModels.contains(where: { $0.id == modelId }) {
            modelId = first.id
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
                    store.showHarnessHealth = true
                } label: {
                    Label("Harness health", systemImage: "heart.text.square")
                }
                Button {
                    store.showDiagnostics = true
                } label: {
                    Label("Report issue", systemImage: "ladybug")
                }
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

            GroupBox("Vault and launch") {
                VStack(alignment: .leading, spacing: 8) {
                    LabeledContent("Vault path") {
                        Text(store.vaultPath.isEmpty ? "—" : store.vaultPath)
                            .font(.caption.monospaced())
                            .textSelection(.enabled)
                    }
                    HStack {
                        Button("Reveal vault") {
                            store.revealVaultInFinder()
                        }
                        Button("Roster config") {
                            if !store.vaultPath.isEmpty {
                                let configURL = URL(fileURLWithPath: store.vaultPath).appendingPathComponent("config.json")
                                NSWorkspace.shared.open(configURL)
                            }
                        }
                    }
                    Toggle("Global launch hotkey (⌘⇧Space)", isOn: Binding(
                        get: { UserPreferences.globalHotkeyEnabled },
                        set: { UserPreferences.globalHotkeyEnabled = $0 }
                    ))
                    .font(.caption)
                    if let coldStart = store.coldStartElapsedMs {
                        Text("Last cold start: \(coldStart) ms (budget \(UserPreferences.coldStartBudgetMs) ms)")
                            .font(.caption)
                            .foregroundStyle(coldStart > UserPreferences.coldStartBudgetMs ? .orange : .secondary)
                    }
                }
            }

            GroupBox("Credentials") {
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(store.gatewayServers) { server in
                        let missing = server.missingCredentialRefs.count
                        let oauth = GatewayOAuthPresentation.forStatus(server.oauthStatus)
                        Text("\(server.displayName): \(missing == 0 ? "credentials ready" : "\(missing) missing") · OAuth \(oauth.statusLabel)")
                            .font(.caption)
                    }
                    if store.gatewayServers.isEmpty {
                        Text("No gateway servers configured.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }

            Text(GatewaySettingsStrings.tamtriGatewayToolsHeading)
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

            Text(GatewaySettingsStrings.agentNativeToolsDisclaimer)
                .font(.caption)
                .foregroundStyle(.secondary)

            Text("Capability badges: supported (green) means tamtri and the server both wire the feature. server only (orange) means the downstream server advertises it but tamtri has not enabled it yet. Sampling is always declined — the harness owns the model.")
                .font(.caption)
                .foregroundStyle(.secondary)

            Link("Apps, Tasks, and Roots guides", destination: URL(string: "https://github.com/im2nguyen/tamtri/blob/main/docs/testing/README.md")!)
                .font(.caption)
        }
        .padding()
        .frame(width: 560, height: 640)
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
        CapabilityBadgeViewModel.effectiveStatus(title: title, status: status)
    }

    private var displayStatus: String {
        effectiveStatus.replacingOccurrences(of: "_", with: " ")
    }

    private var accessibilityText: String {
        CapabilityBadgeViewModel.accessibilityText(title: title, status: status)
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
                    let oauthPresentation = GatewayOAuthPresentation.forStatus(server.oauthStatus)
                    HStack {
                        oauthStatusIcon(for: oauthPresentation)
                        Text("OAuth: \(oauthPresentation.statusLabel)")
                        Spacer()
                        if oauthPresentation.showsConnectButton {
                            Button("Connect") {
                                store.connectOAuth(for: server)
                            }
                        } else {
                            Text("Connected").foregroundStyle(.green)
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
    private func oauthStatusIcon(for presentation: GatewayOAuthPresentation) -> some View {
        switch presentation.iconTone {
        case .connected:
            Image(systemName: presentation.iconSystemName).foregroundStyle(.green)
        case .warning:
            Image(systemName: presentation.iconSystemName).foregroundStyle(.orange)
        case .neutral:
            Image(systemName: presentation.iconSystemName).foregroundStyle(.secondary)
        }
    }
}
