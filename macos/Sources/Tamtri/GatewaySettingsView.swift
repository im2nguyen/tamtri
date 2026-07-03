import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct GatewayServerDraft: Equatable {
    var id: String
    var displayName: String
    var enabled: Bool
    var scope: String
    var transport: String
    var stdioCommand: String
    var stdioArgsText: String
    var stdioEnvText: String
    var httpEndpoint: String

    static let empty = GatewayServerDraft(
        id: "",
        displayName: "",
        enabled: true,
        scope: "user",
        transport: "stdio",
        stdioCommand: "",
        stdioArgsText: "",
        stdioEnvText: "",
        httpEndpoint: ""
    )

    init(
        id: String,
        displayName: String,
        enabled: Bool,
        scope: String,
        transport: String,
        stdioCommand: String,
        stdioArgsText: String,
        stdioEnvText: String,
        httpEndpoint: String
    ) {
        self.id = id
        self.displayName = displayName
        self.enabled = enabled
        self.scope = scope
        self.transport = transport
        self.stdioCommand = stdioCommand
        self.stdioArgsText = stdioArgsText
        self.stdioEnvText = stdioEnvText
        self.httpEndpoint = httpEndpoint
    }

    init(from server: GatewayServerRecord) {
        id = server.id
        displayName = server.displayName
        enabled = server.enabled
        scope = server.scope
        transport = server.transport
        stdioCommand = server.stdioCommand
        stdioArgsText = server.stdioArgs.joined(separator: "\n")
        stdioEnvText = server.stdioEnv.map { "\($0.name)=\($0.value)" }.joined(separator: "\n")
        httpEndpoint = server.httpEndpoint
    }

    func toRecord(preservingCredentialsFrom existing: GatewayServerRecord?) -> GatewayServerRecord {
        GatewayServerRecord(
            id: id.trimmingCharacters(in: .whitespacesAndNewlines),
            displayName: displayName.trimmingCharacters(in: .whitespacesAndNewlines),
            enabled: enabled,
            scope: scope,
            transport: transport,
            stdioCommand: stdioCommand.trimmingCharacters(in: .whitespacesAndNewlines),
            stdioArgs: GatewayServerDraft.parseLines(stdioArgsText),
            stdioEnv: GatewayServerDraft.parseEnv(stdioEnvText),
            httpEndpoint: httpEndpoint.trimmingCharacters(in: .whitespacesAndNewlines),
            credentialRefs: existing?.credentialRefs ?? [],
            missingCredentialRefs: existing?.missingCredentialRefs ?? [],
            oauthStatus: existing?.oauthStatus ?? "not_configured",
            oauthTokenRef: existing?.oauthTokenRef ?? "",
            oauthClientId: existing?.oauthClientId ?? "",
            oauthAuthorizationEndpoint: existing?.oauthAuthorizationEndpoint ?? ""
        )
    }

    static func parseLines(_ text: String) -> [String] {
        text
            .split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    static func parseEnv(_ text: String) -> [GatewayEnvVar] {
        parseLines(text).compactMap { line in
            let parts = line.split(separator: "=", maxSplits: 1).map(String.init)
            guard parts.count == 2 else { return nil }
            let name = parts[0].trimmingCharacters(in: .whitespacesAndNewlines)
            let value = parts[1].trimmingCharacters(in: .whitespacesAndNewlines)
            guard !name.isEmpty else { return nil }
            return GatewayEnvVar(name: name, value: value)
        }
    }
}

enum TwentyQuestionsPreset {
    static func apply(to draft: inout GatewayServerDraft) {
        draft.id = "twenty_questions"
        draft.displayName = "20 Questions"
        draft.enabled = true
        draft.scope = "user"
        draft.transport = "stdio"
        draft.stdioArgsText = ""
        draft.stdioEnvText = "TWENTY_QUESTIONS_SEED=42"
        if let path = locateBinary() {
            draft.stdioCommand = path
        }
    }

    static func locateBinary() -> String? {
        let fileManager = FileManager.default
        let cwd = fileManager.currentDirectoryPath
        let home = fileManager.homeDirectoryForCurrentUser.path
        let candidates = [
            "\(cwd)/target/debug/twenty-questions-mcp",
            "\(cwd)/../target/debug/twenty-questions-mcp",
            "\(home)/Desktop/tamtri/target/debug/twenty-questions-mcp"
        ]
        return candidates.first { fileManager.isExecutableFile(atPath: $0) }
    }
}

struct GatewayServerEditorSheet: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss

    let mode: Mode
    let existingServers: [GatewayServerRecord]
    let onSave: ([GatewayServerRecord]) -> Void

    @State private var draft: GatewayServerDraft
    @State private var validationMessage: String?

    enum Mode {
        case add
        case edit(GatewayServerRecord)
    }

    init(mode: Mode, existingServers: [GatewayServerRecord], onSave: @escaping ([GatewayServerRecord]) -> Void) {
        self.mode = mode
        self.existingServers = existingServers
        self.onSave = onSave
        switch mode {
        case .add:
            _draft = State(initialValue: .empty)
        case .edit(let server):
            _draft = State(initialValue: GatewayServerDraft(from: server))
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(modeTitle)
                .font(.title2.bold())

            Form {
                if case .add = mode {
                    TextField("Server ID", text: $draft.id)
                        .textFieldStyle(.roundedBorder)
                } else {
                    LabeledContent("Server ID", value: draft.id)
                }
                TextField("Display name", text: $draft.displayName)
                    .textFieldStyle(.roundedBorder)
                Toggle("Enabled", isOn: $draft.enabled)
                Picker("Scope", selection: $draft.scope) {
                    Text("System").tag("system")
                    Text("User").tag("user")
                    Text("Project").tag("project")
                }
                Picker("Transport", selection: $draft.transport) {
                    Text("stdio").tag("stdio")
                    Text("Streamable HTTP").tag("streamable_http")
                }

                if draft.transport == "stdio" {
                    HStack {
                        TextField("Command path", text: $draft.stdioCommand)
                            .textFieldStyle(.roundedBorder)
                        Button("Browse…") {
                            pickExecutable()
                        }
                    }
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Arguments (one per line)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        TextEditor(text: $draft.stdioArgsText)
                            .font(.body.monospaced())
                            .frame(height: 56)
                            .overlay(RoundedRectangle(cornerRadius: 6).stroke(.quaternary))
                    }
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Environment (KEY=VALUE per line)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        TextEditor(text: $draft.stdioEnvText)
                            .font(.body.monospaced())
                            .frame(height: 56)
                            .overlay(RoundedRectangle(cornerRadius: 6).stroke(.quaternary))
                    }
                } else {
                    TextField("Endpoint URL", text: $draft.httpEndpoint)
                        .textFieldStyle(.roundedBorder)
                }
            }
            .formStyle(.grouped)

            if case .add = mode {
                Button("Use 20 Questions template") {
                    TwentyQuestionsPreset.apply(to: &draft)
                }
            }

            if let validationMessage {
                Text(validationMessage)
                    .font(.caption)
                    .foregroundStyle(.red)
            }

            HStack {
                Spacer()
                Button("Cancel") {
                    dismiss()
                }
                Button("Save") {
                    save()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 480)
    }

    private var modeTitle: String {
        switch mode {
        case .add: "Add MCP server"
        case .edit: "Edit MCP server"
        }
    }

    private func pickExecutable() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.title = "Choose MCP server executable"
        if panel.runModal() == .OK, let url = panel.url {
            draft.stdioCommand = url.path
        }
    }

    private func save() {
        let existing = existingRecord
        let record = draft.toRecord(preservingCredentialsFrom: existing)
        guard !record.id.isEmpty else {
            validationMessage = "Server ID is required."
            return
        }
        guard !record.displayName.isEmpty else {
            validationMessage = "Display name is required."
            return
        }
        if record.transport == "stdio", record.stdioCommand.isEmpty {
            validationMessage = "Choose the server executable path."
            return
        }
        if record.transport == "streamable_http", record.httpEndpoint.isEmpty {
            validationMessage = "Endpoint URL is required."
            return
        }
        if case .add = mode, existingServers.contains(where: { $0.id == record.id }) {
            validationMessage = "A server with this ID already exists."
            return
        }

        var servers = existingServers
        switch mode {
        case .add:
            servers.append(record)
        case .edit:
            guard let index = servers.firstIndex(where: { $0.id == record.id }) else {
                validationMessage = "Server not found."
                return
            }
            servers[index] = record
        }
        onSave(servers)
        dismiss()
    }

    private var existingRecord: GatewayServerRecord? {
        if case .edit(let server) = mode {
            return server
        }
        return nil
    }
}
