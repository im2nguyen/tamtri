import AppKit
import Foundation
import Security

actor TamtriBindingClient: CoreClient {
    nonisolated let events: AsyncStream<CoreEvent>
    private let core: TamtriCore
    private let observer: BindingObserver

    init(vaultPath: String) throws {
        let stream = AsyncStream.makeStream(of: CoreEvent.self)
        self.events = stream.stream
        self.observer = BindingObserver(continuation: stream.continuation)
        self.core = try TamtriCore(vaultPath: vaultPath, observer: observer)
        try registerDevelopmentAgentsIfPresent()
    }

    func listConversations() async throws -> [ConversationSummary] {
        try core.listConversations().map {
            ConversationSummary(id: $0.id, title: $0.title, updatedAt: $0.updatedAt)
        }
    }

    func loadConversation(id: String) async throws -> ConversationRecord {
        try record(from: core.loadConversation(id: id))
    }

    func createConversation(title: String, harnessId: String, modelId: String) async throws -> ConversationRecord {
        try record(from: core.createConversation(title: title, harnessId: harnessId, modelId: modelId))
    }

    func forkConversation(id: String, harnessId: String, modelId: String) async throws -> ConversationRecord {
        try record(from: core.forkConversation(id: id, harnessId: harnessId, modelId: modelId))
    }

    func sendMessage(conversationId: String, text: String) async throws {
        try core.sendMessage(conversationId: conversationId, text: text)
    }

    func copyFileToWorkdir(conversationId: String, sourcePath: String) async throws -> String {
        try core.copyFileToWorkdir(conversationId: conversationId, sourcePath: sourcePath)
    }

    func listWorkdirFiles(conversationId: String) async throws -> [WorkdirFileRecord] {
        try core.listWorkdirFiles(conversationId: conversationId).map {
            WorkdirFileRecord(
                relativePath: $0.relativePath,
                size: $0.size,
                mimeType: $0.mimeType,
                modifiedAt: $0.modifiedAt
            )
        }
    }

    func conversationWorkdirPath(conversationId: String) async throws -> String {
        try core.conversationWorkdirPath(conversationId: conversationId)
    }

    func readWorkdirFile(conversationId: String, relativePath: String) async throws -> WorkdirFilePreview {
        let content = try core.readWorkdirFile(conversationId: conversationId, relativePath: relativePath)
        let mimeType = content.mimeType
        if artifactIsImageMime(mimeType), NSImage(data: Data(content.data)) != nil {
            return WorkdirFilePreview(
                relativePath: relativePath,
                mimeType: mimeType,
                text: nil,
                imageData: Data(content.data),
                error: nil
            )
        }
        if artifactIsTextLikeMime(mimeType), let text = String(data: Data(content.data), encoding: .utf8) {
            return WorkdirFilePreview(
                relativePath: relativePath,
                mimeType: mimeType,
                text: text,
                imageData: nil,
                error: nil
            )
        }
        return WorkdirFilePreview(
            relativePath: relativePath,
            mimeType: mimeType,
            text: nil,
            imageData: nil,
            error: "No in-app preview for this file type."
        )
    }

    func verifyArtifactInline(size: UInt64, sha256: String, inlineContent: String) async throws {
        try core.verifyArtifactInline(size: size, sha256: sha256, inlineContent: inlineContent)
    }

    func logArtifactNavigationBlocked(conversationId: String, url: String) async throws {
        try core.logArtifactNavigationBlocked(conversationId: conversationId, url: url)
    }

    func readAttachmentVerified(conversationId: String, path: String, size: UInt64, sha256: String) async throws -> Data {
        try core.readAttachmentVerified(conversationId: conversationId, path: path, size: size, sha256: sha256)
    }

    func verifiedAttachmentPath(conversationId: String, path: String, size: UInt64, sha256: String) async throws -> String {
        try core.verifiedAttachmentPath(conversationId: conversationId, path: path, size: size, sha256: sha256)
    }

    func respondPermission(conversationId: String, requestId: String, optionId: String) async throws {
        try core.respondPermission(conversationId: conversationId, requestId: requestId, optionId: optionId)
    }

    func respondElicitation(conversationId: String, requestId: String, action: String, dataJSON: String?) async throws {
        try core.respondElicitation(conversationId: conversationId, requestId: requestId, action: action, dataJson: dataJSON)
    }

    func cancelRun(conversationId: String) async throws {
        try core.cancelRun(conversationId: conversationId)
    }

    func listGatewayServers() async throws -> [GatewayServerRecord] {
        try core.listGatewayServers().map(gatewayServerRecord(from:))
    }

    func refreshGatewayCapabilities() async throws -> [GatewayServerRecord] {
        try core.refreshGatewayCapabilities().map(gatewayServerRecord(from:))
    }

    func saveGatewayServers(_ servers: [GatewayServerRecord]) async throws {
        try core.saveGatewayServers(servers: servers.map(gatewayServerDto(from:)))
    }

    func setGatewayCredential(credentialRef: String, value: String) async throws {
        try KeychainCredentialStore.save(value: value, for: credentialRef)
        try core.setGatewayCredential(credentialRef: credentialRef, value: value)
    }

    func exportGatewayCredential(credentialRef: String) async throws -> String? {
        try core.exportGatewayCredential(credentialRef: credentialRef)
    }

    func startOAuthFlow(serverId: String, redirectURI: String) async throws -> OAuthHandoff {
        let dto = try core.startOauthFlow(serverId: serverId, redirectUri: redirectURI)
        return OAuthHandoff(
            serverId: dto.serverId,
            authorizationURL: dto.authorizationUrl,
            state: dto.state,
            redirectURI: dto.redirectUri
        )
    }

    func completeOAuthCallback(callbackURL: String) async throws -> OAuthCompletion {
        let dto = try core.completeOauthCallback(callbackUrl: callbackURL)
        if let server = try await listGatewayServers().first(where: { $0.id == dto.serverId }),
           !server.oauthTokenRef.isEmpty,
           let value = try core.exportGatewayCredential(credentialRef: server.oauthTokenRef) {
            try KeychainCredentialStore.save(value: value, for: server.oauthTokenRef)
        }
        return OAuthCompletion(serverId: dto.serverId, oauthStatus: dto.oauthStatus)
    }

    nonisolated private func registerDevelopmentAgentsIfPresent() throws {
        if let command = mockAgentPath() {
            try core.registerAcpAgent(
                id: "mock-acp",
                displayName: "Mock ACP",
                command: command,
                args: []
            )
        }

        if let command = hermesAgentPath() {
            try core.registerAcpAgent(
                id: "hermes-acp",
                displayName: "Hermes ACP",
                command: command,
                args: ["acp"]
            )
        }
    }
}

private final class BindingObserver: ConversationObserver, @unchecked Sendable {
    private let continuation: AsyncStream<CoreEvent>.Continuation

    init(continuation: AsyncStream<CoreEvent>.Continuation) {
        self.continuation = continuation
    }

    func onEvent(event: UiEvent) {
        continuation.yield(
            CoreEvent(
                conversationId: event.conversationId,
                kind: event.kind,
                payloadJSON: event.payloadJson
            )
        )
    }
}

private func record(from dto: ConversationDto) -> ConversationRecord {
    ConversationRecord(
        id: dto.id,
        title: dto.title,
        harnessId: dto.activeHarnessId,
        modelId: dto.modelId,
        transcriptJSON: dto.transcriptJson
    )
}

private func gatewayServerRecord(from dto: GatewayServerDto) -> GatewayServerRecord {
    GatewayServerRecord(
        id: dto.id,
        displayName: dto.displayName,
        enabled: dto.enabled,
        scope: dto.scope,
        transport: dto.transport,
        stdioCommand: dto.stdioCommand,
        stdioArgs: dto.stdioArgs,
        stdioEnv: dto.stdioEnv.map { GatewayEnvVar(name: $0.name, value: $0.value) },
        httpEndpoint: dto.httpEndpoint,
        credentialRefs: dto.credentialRefs,
        missingCredentialRefs: dto.missingCredentialRefs,
        oauthStatus: dto.oauthStatus,
        oauthTokenRef: dto.oauthTokenRef,
        oauthClientId: dto.oauthClientId,
        oauthAuthorizationEndpoint: dto.oauthAuthorizationEndpoint,
        oauthTokenEndpoint: dto.oauthTokenEndpoint,
        oauthScopes: dto.oauthScopes,
        capTools: dto.capTools,
        capResources: dto.capResources,
        capPrompts: dto.capPrompts,
        capElicitation: dto.capElicitation,
        capApps: dto.capApps,
        capTasks: dto.capTasks,
        capRoots: dto.capRoots,
        capSampling: dto.capSampling
    )
}

private func gatewayServerDto(from record: GatewayServerRecord) -> GatewayServerDto {
    GatewayServerDto(
        id: record.id,
        displayName: record.displayName,
        enabled: record.enabled,
        scope: record.scope,
        transport: record.transport,
        stdioCommand: record.stdioCommand,
        stdioArgs: record.stdioArgs,
        stdioEnv: record.stdioEnv.map { GatewayEnvVarDto(name: $0.name, value: $0.value) },
        httpEndpoint: record.httpEndpoint,
        credentialRefs: record.credentialRefs,
        missingCredentialRefs: record.missingCredentialRefs,
        oauthStatus: record.oauthStatus,
        oauthTokenRef: record.oauthTokenRef,
        oauthClientId: record.oauthClientId,
        oauthAuthorizationEndpoint: record.oauthAuthorizationEndpoint,
        oauthTokenEndpoint: record.oauthTokenEndpoint,
        oauthScopes: record.oauthScopes,
        capTools: record.capTools,
        capResources: record.capResources,
        capPrompts: record.capPrompts,
        capElicitation: record.capElicitation,
        capApps: record.capApps,
        capTasks: record.capTasks,
        capRoots: record.capRoots,
        capSampling: record.capSampling
    )
}

private func mockAgentPath() -> String? {
    let fileManager = FileManager.default
    let cwd = fileManager.currentDirectoryPath
    let candidates = [
        "\(cwd)/target/debug/mock-acp-agent",
        "\(cwd)/../target/debug/mock-acp-agent"
    ]
    return candidates.first { fileManager.isExecutableFile(atPath: $0) }
}

func hermesAgentPath() -> String? {
    let fileManager = FileManager.default
    let home = fileManager.homeDirectoryForCurrentUser.path
    let candidates = [
        "\(home)/.local/bin/hermes",
        "/opt/homebrew/bin/hermes",
        "/usr/local/bin/hermes"
    ]
    return candidates.first { fileManager.isExecutableFile(atPath: $0) }
}

func defaultHarnessId() -> String {
    hermesAgentPath() == nil ? "mock-acp" : "hermes-acp"
}

func defaultModelId() -> String {
    hermesAgentPath() == nil ? "mock" : "default"
}

func makeDefaultCoreClient() -> CoreClient {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let vaultURL = home.appendingPathComponent(".tamtri/vault", isDirectory: true)
    if let client = try? TamtriBindingClient(vaultPath: vaultURL.path) {
        return client
    }
    return MockCoreClient()
}
