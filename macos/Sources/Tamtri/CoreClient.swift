import Foundation

struct ConversationSummary: Identifiable, Equatable {
    let id: String
    let title: String
    let updatedAt: String
}

struct RootRecord: Identifiable, Equatable {
    let id: String
    let name: String
    let uri: String
    let kind: String
    let scope: String
    let bookmarkMissing: Bool
}

struct ConversationRecord: Equatable {
    let id: String
    let title: String
    let harnessId: String?
    let modelId: String?
    let forkedFrom: String?
    let transcriptJSON: String
    let parsedMessages: [ParsedTranscriptMessage]

    init(
        id: String,
        title: String,
        harnessId: String?,
        modelId: String?,
        forkedFrom: String? = nil,
        transcriptJSON: String
    ) {
        self.id = id
        self.title = title
        self.harnessId = harnessId
        self.modelId = modelId
        self.forkedFrom = forkedFrom
        self.transcriptJSON = transcriptJSON
        self.parsedMessages = TranscriptParsing.parseTranscript(transcriptJSON)
    }
}

struct CoreEvent: Equatable {
    let conversationId: String
    let kind: String
    let payloadJSON: String
}

struct GatewayEnvVar: Equatable {
    let name: String
    let value: String
}

struct GatewayServerRecord: Identifiable, Equatable {
    let id: String
    let displayName: String
    let enabled: Bool
    let scope: String
    let transport: String
    let stdioCommand: String
    let stdioArgs: [String]
    let stdioEnv: [GatewayEnvVar]
    let httpEndpoint: String
    let credentialRefs: [String]
    let missingCredentialRefs: [String]
    let oauthStatus: String
    let oauthTokenRef: String
    let oauthClientId: String
    let oauthAuthorizationEndpoint: String
    let oauthTokenEndpoint: String
    let oauthScopes: [String]
    let capTools: String
    let capResources: String
    let capPrompts: String
    let capElicitation: String
    let capApps: String
    let capTasks: String
    let capRoots: String
    let capSampling: String
    let connectionStatus: String
    let lastError: String
    let timeoutSecs: UInt64?
}

struct WorkdirFileRecord: Equatable, Identifiable, Hashable {
    var id: String { relativePath }
    let relativePath: String
    let size: UInt64
    let mimeType: String?
    let modifiedAt: UInt64
}

struct WorkdirFilePreview: Equatable {
    let relativePath: String
    let mimeType: String?
    let text: String?
    let imageData: Data?
    let error: String?
}

struct AttachmentFilePreview: Equatable {
    let path: String
    let mimeType: String?
    let text: String?
    let imageData: Data?
    let error: String?
}

struct HarnessAgentRecord: Identifiable, Equatable {
    let id: String
    let displayName: String
}

struct HarnessHealthRecord: Identifiable, Equatable {
    let id: String
    let displayName: String
    let command: String
    let status: String
    let installDocURL: String
}

struct SearchHitRecord: Identifiable, Equatable {
    var id: String { "\(conversationId)-\(matchField)-\(snippet.hashValue)" }
    let conversationId: String
    let title: String
    let snippet: String
    let matchField: String
}

struct ImportWarningRecord: Equatable {
    let kind: String
    let detail: String
}

struct ImportResultRecord: Equatable {
    let conversation: ConversationRecord
    let warnings: [ImportWarningRecord]
}

struct VaultIssueRecord: Identifiable, Equatable {
    var id: String { "\(kind)-\(conversationId ?? path ?? detail)" }
    let kind: String
    let conversationId: String?
    let path: String?
    let reason: String?
    let winnerPath: String?
    let loserPaths: [String]
    let detail: String
}

struct ModelInfoRecord: Identifiable, Equatable {
    let id: String
    let displayName: String
}

struct GatewayToolRecord: Identifiable, Equatable {
    var id: String { exposedName }
    let exposedName: String
    let serverId: String
    let originalName: String
}

protocol CoreClient: Sendable {
    var events: AsyncStream<CoreEvent> { get }

    func listConversations() async throws -> [ConversationSummary]
    func loadConversation(id: String) async throws -> ConversationRecord
    func createConversation(title: String, harnessId: String, modelId: String) async throws -> ConversationRecord
    func forkConversation(id: String, harnessId: String, modelId: String) async throws -> ConversationRecord
    func listAcpAgents() async throws -> [HarnessAgentRecord]
    func listAcpAgentModels(agentId: String) async throws -> [ModelInfoRecord]
    func sendMessage(conversationId: String, text: String) async throws
    func syncRuntimeRoots(conversationId: String, roots: [RootDto]) async throws
    func copyFileToWorkdir(conversationId: String, sourcePath: String) async throws -> String
    func listWorkdirFiles(conversationId: String) async throws -> [WorkdirFileRecord]
    func conversationWorkdirPath(conversationId: String) async throws -> String
    func conversationFolderPath(conversationId: String) async throws -> String
    func readWorkdirFile(conversationId: String, relativePath: String) async throws -> WorkdirFilePreview
    func verifyArtifactInline(size: UInt64, sha256: String, inlineContent: String) async throws
    func logArtifactNavigationBlocked(conversationId: String, url: String) async throws
    func resolveAppTemplate(conversationId: String, serverId: String, templateRef: String) async throws -> AppTemplateRecord?
    func submitAppBridgeRequest(conversationId: String, serverId: String, appId: String, templateRef: String, requestJSON: String) async throws -> AppBridgeSubmission
    func respondAppBridgeConsent(conversationId: String, requestId: String, optionId: String) async throws
    func logAppNavigationBlocked(conversationId: String, serverId: String, templateRef: String, url: String) async throws
    nonisolated func appBridgeBootstrapScript() -> String
    func readAttachmentVerified(conversationId: String, path: String, size: UInt64, sha256: String) async throws -> Data
    func verifiedAttachmentPath(conversationId: String, path: String, size: UInt64, sha256: String) async throws -> String
    func respondPermission(conversationId: String, requestId: String, optionId: String) async throws
    func respondElicitation(conversationId: String, requestId: String, action: String, dataJSON: String?) async throws
    func cancelRun(conversationId: String) async throws
    nonisolated func prepareForAppQuitSync() throws
    func cancelTask(conversationId: String, taskId: String) async throws
    func listRoots(conversationId: String) async throws -> [RootRecord]
    func attachRoot(conversationId: String, name: String, uri: String, kind: String, scope: String) async throws -> RootRecord
    func removeRoot(conversationId: String, rootId: String) async throws
    func listGatewayServers() async throws -> [GatewayServerRecord]
    func refreshGatewayCapabilities() async throws -> [GatewayServerRecord]
    func listGatewayTools() async throws -> [GatewayToolRecord]
    func getGatewaySettings() async throws -> UInt64
    func setGatewayDefaultTimeout(_ seconds: UInt64) async throws
    func saveGatewayServers(_ servers: [GatewayServerRecord]) async throws
    func setGatewayCredential(credentialRef: String, value: String) async throws
    func exportGatewayCredential(credentialRef: String) async throws -> String?
    func startOAuthFlow(serverId: String, redirectURI: String) async throws -> OAuthHandoff
    func completeOAuthCallback(callbackURL: String) async throws -> OAuthCompletion
    func exportConversationBundle(conversationId: String, destPath: String) async throws
    func importBundleOrFolder(sourcePath: String) async throws -> ImportResultRecord
    func searchConversations(query: String) async throws -> [SearchHitRecord]
    func searchScopeMessage() async -> String
    func listHarnessHealth() async throws -> [HarnessHealthRecord]
    func harnessHealthChecklist() async throws -> String
    func vaultIssues() async throws -> [VaultIssueRecord]
    func vaultPath() async -> String
    func writeDiagnosticsBundle(destPath: String, systemInfoJSON: String) async throws -> String
}

struct OAuthHandoff: Equatable {
    let serverId: String
    let authorizationURL: String
    let state: String
    let redirectURI: String
}

struct OAuthCompletion: Equatable {
    let serverId: String
    let oauthStatus: String
}

actor MockCoreClient: CoreClient {
    nonisolated let events: AsyncStream<CoreEvent>
    private let continuation: AsyncStream<CoreEvent>.Continuation
    private var conversations: [ConversationRecord] = [
        ConversationRecord(
            id: "sample",
            title: "Report from CSV",
            harnessId: "mock-acp",
            modelId: "mock",
            transcriptJSON: "[]"
        )
    ]

    init() {
        let stream = AsyncStream.makeStream(of: CoreEvent.self)
        self.events = stream.stream
        self.continuation = stream.continuation
    }

    func listConversations() async throws -> [ConversationSummary] {
        conversations.map {
            ConversationSummary(id: $0.id, title: $0.title, updatedAt: "now")
        }
    }

    func loadConversation(id: String) async throws -> ConversationRecord {
        conversations.first(where: { $0.id == id }) ?? conversations[0]
    }

    func createConversation(title: String, harnessId: String, modelId: String) async throws -> ConversationRecord {
        let record = ConversationRecord(id: UUID().uuidString, title: title, harnessId: harnessId, modelId: modelId, transcriptJSON: "[]")
        conversations.insert(record, at: 0)
        return record
    }

    func forkConversation(id: String, harnessId: String, modelId: String) async throws -> ConversationRecord {
        let parent = conversations.first(where: { $0.id == id }) ?? conversations[0]
        let record = ConversationRecord(
            id: UUID().uuidString,
            title: "\(parent.title) fork",
            harnessId: harnessId,
            modelId: modelId,
            transcriptJSON: parent.transcriptJSON
        )
        conversations.insert(record, at: 0)
        return record
    }

    func listAcpAgents() async throws -> [HarnessAgentRecord] {
        [HarnessAgentRecord(id: "mock-acp", displayName: "Mock ACP")]
    }

    func listAcpAgentModels(agentId: String) async throws -> [ModelInfoRecord] {
        _ = agentId
        return [ModelInfoRecord(id: "mock", displayName: "Mock Model")]
    }

    func sendMessage(conversationId: String, text: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "text_delta", payloadJSON: #"{"text":"Thinking about it..."}"#))
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "permission_requested", payloadJSON: #"{"request_id":"mock-permission","action":"edit","options":[{"id":"allow_once","label":"Allow once"},{"id":"allow_for_conversation","label":"Allow for this conversation"},{"id":"deny","label":"Deny"}]}"#))
    }

    func syncRuntimeRoots(conversationId: String, roots: [RootDto]) async throws {}

    func copyFileToWorkdir(conversationId: String, sourcePath: String) async throws -> String {
        URL(fileURLWithPath: sourcePath).lastPathComponent
    }

    func listWorkdirFiles(conversationId: String) async throws -> [WorkdirFileRecord] {
        []
    }

    func conversationWorkdirPath(conversationId: String) async throws -> String {
        "/tmp/workdir"
    }

    func conversationFolderPath(conversationId: String) async throws -> String {
        "/tmp/conversations/\(conversationId)"
    }

    func readWorkdirFile(conversationId: String, relativePath: String) async throws -> WorkdirFilePreview {
        WorkdirFilePreview(relativePath: relativePath, mimeType: "text/plain", text: "mock", imageData: nil, error: nil)
    }

    func verifyArtifactInline(size: UInt64, sha256: String, inlineContent: String) async throws {}

    func logArtifactNavigationBlocked(conversationId: String, url: String) async throws {}

    func resolveAppTemplate(conversationId: String, serverId: String, templateRef: String) async throws -> AppTemplateRecord? { nil }

    func submitAppBridgeRequest(conversationId: String, serverId: String, appId: String, templateRef: String, requestJSON: String) async throws -> AppBridgeSubmission {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "app_bridge_consent_requested", payloadJSON: #"{"request_id":"mock-bridge","server_id":"mock","app_id":"demo","template_ref":"ui://demo","summary":"Mock app wants to call echo","options":[{"id":"deny","label":"Deny"},{"id":"allow_once","label":"Allow once"},{"id":"allow_for_conversation","label":"Allow for this conversation"}]}"#))
        return AppBridgeSubmission(requestId: "mock-bridge", needsConsent: true)
    }

    func respondAppBridgeConsent(conversationId: String, requestId: String, optionId: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "app_bridge_resolved", payloadJSON: #"{"request_id":"mock-bridge","response_json":"{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"result\":{}}"}"#))
    }

    func logAppNavigationBlocked(conversationId: String, serverId: String, templateRef: String, url: String) async throws {}

    nonisolated func appBridgeBootstrapScript() -> String {
        "(function(){window.__tamtriAppBridgeInstalled=true;})();"
    }

    func readAttachmentVerified(conversationId: String, path: String, size: UInt64, sha256: String) async throws -> Data {
        Data()
    }

    func verifiedAttachmentPath(conversationId: String, path: String, size: UInt64, sha256: String) async throws -> String {
        path
    }

    func respondPermission(conversationId: String, requestId: String, optionId: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "permission_resolved", payloadJSON: "{}"))
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "message_committed", payloadJSON: #"{"content":[{"type":"text","text":"Done."}]}"#))
    }

    func respondElicitation(conversationId: String, requestId: String, action: String, dataJSON: String?) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "elicitation_resolved", payloadJSON: "{}"))
    }

    func cancelRun(conversationId: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "turn_ended", payloadJSON: #"{"reason":"cancelled"}"#))
    }

    nonisolated func prepareForAppQuitSync() throws {}

    func cancelTask(conversationId: String, taskId: String) async throws {}

    func listRoots(conversationId: String) async throws -> [RootRecord] { [] }

    func attachRoot(conversationId: String, name: String, uri: String, kind: String, scope: String) async throws -> RootRecord {
        RootRecord(id: UUID().uuidString, name: name, uri: uri, kind: kind, scope: scope, bookmarkMissing: true)
    }

    func removeRoot(conversationId: String, rootId: String) async throws {}

    func listGatewayServers() async throws -> [GatewayServerRecord] {
        [
            GatewayServerRecord(
                id: "mock",
                displayName: "Mock MCP",
                enabled: true,
                scope: "project",
                transport: "stdio",
                stdioCommand: "/tmp/mock-mcp",
                stdioArgs: [],
                stdioEnv: [],
                httpEndpoint: "",
                credentialRefs: [],
                missingCredentialRefs: [],
                oauthStatus: "not_configured",
                oauthTokenRef: "",
                oauthClientId: "",
                oauthAuthorizationEndpoint: "",
                oauthTokenEndpoint: "",
                oauthScopes: [],
                capTools: "unknown",
                capResources: "unknown",
                capPrompts: "unknown",
                capElicitation: "unknown",
                capApps: "unknown",
                capTasks: "unknown",
                capRoots: "unknown",
                capSampling: "declined",
                connectionStatus: "unknown",
                lastError: "",
                timeoutSecs: nil
            )
        ]
    }

    func refreshGatewayCapabilities() async throws -> [GatewayServerRecord] {
        try await listGatewayServers()
    }

    func listGatewayTools() async throws -> [GatewayToolRecord] {
        [GatewayToolRecord(exposedName: "mock__echo", serverId: "mock", originalName: "echo")]
    }

    func getGatewaySettings() async throws -> UInt64 { 300 }

    func setGatewayDefaultTimeout(_ seconds: UInt64) async throws {}

    func saveGatewayServers(_ servers: [GatewayServerRecord]) async throws {}

    func setGatewayCredential(credentialRef: String, value: String) async throws {}
    func exportGatewayCredential(credentialRef: String) async throws -> String? { nil }
    func startOAuthFlow(serverId: String, redirectURI: String) async throws -> OAuthHandoff {
        OAuthHandoff(serverId: serverId, authorizationURL: "https://example.com", state: "mock", redirectURI: redirectURI)
    }
    func completeOAuthCallback(callbackURL: String) async throws -> OAuthCompletion {
        OAuthCompletion(serverId: "mock", oauthStatus: "connected")
    }

    func exportConversationBundle(conversationId: String, destPath: String) async throws {}

    func importBundleOrFolder(sourcePath: String) async throws -> ImportResultRecord {
        ImportResultRecord(
            conversation: conversations[0],
            warnings: []
        )
    }

    func searchConversations(query: String) async throws -> [SearchHitRecord] {
        conversations
            .filter { $0.title.localizedCaseInsensitiveContains(query) }
            .map {
                SearchHitRecord(
                    conversationId: $0.id,
                    title: $0.title,
                    snippet: $0.title,
                    matchField: "title"
                )
            }
    }

    func searchScopeMessage() async -> String {
        "Search covers conversation titles plus Text and Thinking blocks only."
    }

    func listHarnessHealth() async throws -> [HarnessHealthRecord] {
        [
            HarnessHealthRecord(
                id: "mock-acp",
                displayName: "Mock ACP",
                command: "mock-acp-agent",
                status: "ready",
                installDocURL: "https://github.com/tamtri/tamtri"
            )
        ]
    }

    func harnessHealthChecklist() async throws -> String {
        "tamtri harness setup checklist\n\n- Mock ACP (mock-acp) — status: ready"
    }

    func vaultIssues() async throws -> [VaultIssueRecord] { [] }

    func vaultPath() async -> String {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".tamtri/vault")
            .path
    }

    func writeDiagnosticsBundle(destPath: String, systemInfoJSON: String) async throws -> String {
        destPath.hasSuffix(".zip") ? destPath : "\(destPath).zip"
    }
}
