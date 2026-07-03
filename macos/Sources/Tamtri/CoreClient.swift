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
    let transcriptJSON: String
    let parsedMessages: [ParsedTranscriptMessage]

    init(
        id: String,
        title: String,
        harnessId: String?,
        modelId: String?,
        transcriptJSON: String
    ) {
        self.id = id
        self.title = title
        self.harnessId = harnessId
        self.modelId = modelId
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

protocol CoreClient: Sendable {
    var events: AsyncStream<CoreEvent> { get }

    func listConversations() async throws -> [ConversationSummary]
    func loadConversation(id: String) async throws -> ConversationRecord
    func createConversation(title: String, harnessId: String, modelId: String) async throws -> ConversationRecord
    func forkConversation(id: String, harnessId: String, modelId: String) async throws -> ConversationRecord
    func sendMessage(conversationId: String, text: String) async throws
    func syncRuntimeRoots(conversationId: String, roots: [RootDto]) async throws
    func copyFileToWorkdir(conversationId: String, sourcePath: String) async throws -> String
    func listWorkdirFiles(conversationId: String) async throws -> [WorkdirFileRecord]
    func conversationWorkdirPath(conversationId: String) async throws -> String
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
    func cancelTask(conversationId: String, taskId: String) async throws
    func listRoots(conversationId: String) async throws -> [RootRecord]
    func attachRoot(conversationId: String, name: String, uri: String, kind: String, scope: String) async throws -> RootRecord
    func removeRoot(conversationId: String, rootId: String) async throws
    func listGatewayServers() async throws -> [GatewayServerRecord]
    func refreshGatewayCapabilities() async throws -> [GatewayServerRecord]
    func saveGatewayServers(_ servers: [GatewayServerRecord]) async throws
    func setGatewayCredential(credentialRef: String, value: String) async throws
    func exportGatewayCredential(credentialRef: String) async throws -> String?
    func startOAuthFlow(serverId: String, redirectURI: String) async throws -> OAuthHandoff
    func completeOAuthCallback(callbackURL: String) async throws -> OAuthCompletion
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

    func sendMessage(conversationId: String, text: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "text_delta", payloadJSON: #"{"text":"Thinking about it..."}"#))
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "permission_requested", payloadJSON: #"{"request_id":"mock-permission","action":"edit","options":[{"id":"allow_once","label":"Allow once"},{"id":"deny","label":"Deny"}]}"#))
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

    func readWorkdirFile(conversationId: String, relativePath: String) async throws -> WorkdirFilePreview {
        WorkdirFilePreview(relativePath: relativePath, mimeType: "text/plain", text: "mock", imageData: nil, error: nil)
    }

    func verifyArtifactInline(size: UInt64, sha256: String, inlineContent: String) async throws {}

    func logArtifactNavigationBlocked(conversationId: String, url: String) async throws {}

    func resolveAppTemplate(conversationId: String, serverId: String, templateRef: String) async throws -> AppTemplateRecord? { nil }

    func submitAppBridgeRequest(conversationId: String, serverId: String, appId: String, templateRef: String, requestJSON: String) async throws -> AppBridgeSubmission {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "app_bridge_consent_requested", payloadJSON: #"{"request_id":"mock-bridge","server_id":"mock","app_id":"demo","template_ref":"ui://demo","summary":"Mock app wants to call echo","options":[{"id":"deny","label":"Deny"},{"id":"allow_once","label":"Allow once"}]}"#))
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

    func saveGatewayServers(_ servers: [GatewayServerRecord]) async throws {}

    func setGatewayCredential(credentialRef: String, value: String) async throws {}
    func exportGatewayCredential(credentialRef: String) async throws -> String? { nil }
    func startOAuthFlow(serverId: String, redirectURI: String) async throws -> OAuthHandoff {
        OAuthHandoff(serverId: serverId, authorizationURL: "https://example.com", state: "mock", redirectURI: redirectURI)
    }
    func completeOAuthCallback(callbackURL: String) async throws -> OAuthCompletion {
        OAuthCompletion(serverId: "mock", oauthStatus: "connected")
    }
}
