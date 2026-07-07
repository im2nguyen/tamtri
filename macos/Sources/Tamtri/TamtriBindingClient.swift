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
            ConversationSummary(
                id: $0.id,
                title: $0.title,
                updatedAt: $0.updatedAt,
                activeHarnessId: $0.activeHarnessId
            )
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

    func listAcpAgents() async throws -> [HarnessAgentRecord] {
        try core.listAcpAgents().map {
            HarnessAgentRecord(id: $0.id, displayName: $0.displayName)
        }
    }

    func listAcpAgentModels(agentId: String) async throws -> [ModelInfoRecord] {
        try core.listAcpAgentModels(agentId: agentId).map {
            ModelInfoRecord(id: $0.id, displayName: $0.displayName)
        }
    }

    func sendMessage(conversationId: String, text: String) async throws {
        try core.sendMessage(conversationId: conversationId, text: text)
    }

    func syncRuntimeRoots(conversationId: String, roots: [RootDto]) async throws {
        try core.syncRuntimeRoots(conversationId: conversationId, roots: roots)
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

    func conversationFolderPath(conversationId: String) async throws -> String {
        try core.conversationFolderPath(conversationId: conversationId)
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

    func resolveAppTemplate(conversationId: String, serverId: String, templateRef: String) async throws -> AppTemplateRecord? {
        try core.resolveAppTemplate(conversationId: conversationId, serverId: serverId, templateRef: templateRef).map(appTemplateRecord(from:))
    }

    func submitAppBridgeRequest(conversationId: String, serverId: String, appId: String, templateRef: String, requestJSON: String) async throws -> AppBridgeSubmission {
        let dto = try core.submitAppBridgeRequest(
            conversationId: conversationId,
            serverId: serverId,
            appId: appId,
            templateRef: templateRef,
            requestJson: requestJSON
        )
        return AppBridgeSubmission(requestId: dto.requestId, needsConsent: dto.needsConsent)
    }

    func respondAppBridgeConsent(conversationId: String, requestId: String, optionId: String) async throws {
        try core.respondAppBridgeConsent(conversationId: conversationId, requestId: requestId, optionId: optionId)
    }

    func logAppNavigationBlocked(conversationId: String, serverId: String, templateRef: String, url: String) async throws {
        try core.logAppNavigationBlocked(
            conversationId: conversationId,
            serverId: serverId,
            templateRef: templateRef,
            url: url
        )
    }

    nonisolated func appBridgeBootstrapScript() -> String {
        core.appBridgeBootstrapScript()
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

    nonisolated func prepareForAppQuitSync() throws {
        try core.prepareForAppQuit()
    }

    func cancelTask(conversationId: String, taskId: String) async throws {
        try core.cancelTask(conversationId: conversationId, taskId: taskId)
    }

    func listRoots(conversationId: String) async throws -> [RootRecord] {
        try core.listRoots(conversationId: conversationId).map { rootRecord(from: $0, conversationId: conversationId) }
    }

    func attachRoot(conversationId: String, name: String, uri: String, kind: String, scope: String) async throws -> RootRecord {
        let dto = try core.attachRoot(
            conversationId: conversationId,
            name: name,
            uri: uri,
            kind: kind,
            scope: scope
        )
        return rootRecord(from: dto, conversationId: conversationId)
    }

    func removeRoot(conversationId: String, rootId: String) async throws {
        try core.removeRoot(conversationId: conversationId, rootId: rootId)
    }

    func listGatewayServers() async throws -> [GatewayServerRecord] {
        try core.listGatewayServers().map(gatewayServerRecord(from:))
    }

    func refreshGatewayCapabilities() async throws -> [GatewayServerRecord] {
        try core.refreshGatewayCapabilities().map(gatewayServerRecord(from:))
    }

    func listGatewayTools() async throws -> [GatewayToolRecord] {
        try core.listGatewayTools().map {
            GatewayToolRecord(
                exposedName: $0.exposedName,
                serverId: $0.serverId,
                originalName: $0.originalName
            )
        }
    }

    func getGatewaySettings() async throws -> UInt64 {
        try core.getGatewaySettings().defaultCallTimeoutSecs
    }

    func setGatewayDefaultTimeout(_ seconds: UInt64) async throws {
        try core.setGatewayDefaultTimeout(defaultCallTimeoutSecs: seconds)
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

    func exportConversationBundle(conversationId: String, destPath: String) async throws {
        try core.exportConversationBundle(conversationId: conversationId, destPath: destPath)
    }

    func importBundleOrFolder(sourcePath: String) async throws -> ImportResultRecord {
        let result = try core.importBundleOrFolderAsNew(sourcePath: sourcePath)
        return ImportResultRecord(
            conversation: record(from: result.conversation),
            warnings: result.warnings.map {
                ImportWarningRecord(kind: $0.kind, detail: $0.detail)
            }
        )
    }

    func searchConversations(query: String) async throws -> [SearchHitRecord] {
        try core.searchConversations(query: query).map {
            SearchHitRecord(
                conversationId: $0.conversationId,
                title: $0.title,
                snippet: $0.snippet,
                matchField: $0.matchField
            )
        }
    }

    func searchScopeMessage() async -> String {
        core.searchScopeMessage()
    }

    func listHarnessHealth() async throws -> [HarnessHealthRecord] {
        try core.listHarnessHealth().map {
            HarnessHealthRecord(
                id: $0.id,
                displayName: $0.displayName,
                command: $0.command,
                status: $0.status,
                installDocURL: $0.installDocUrl
            )
        }
    }

    func harnessHealthChecklist() async throws -> String {
        try core.harnessHealthChecklist()
    }

    func vaultIssues() async throws -> [VaultIssueRecord] {
        try core.vaultIssues().map {
            VaultIssueRecord(
                kind: $0.kind,
                conversationId: $0.conversationId,
                path: $0.path,
                reason: $0.reason,
                winnerPath: $0.winnerPath,
                loserPaths: $0.loserPaths,
                detail: $0.detail
            )
        }
    }

    func vaultPath() async -> String {
        core.vaultPath()
    }

    func writeDiagnosticsBundle(destPath: String, systemInfoJSON: String) async throws -> String {
        try core.writeDiagnosticsBundle(destPath: destPath, systemInfoJson: systemInfoJSON)
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
        forkedFrom: dto.forkedFrom,
        transcriptJSON: dto.transcriptJson
    )
}

private func rootRecord(from dto: RootDto, conversationId: String) -> RootRecord {
    RootRecord(
        id: dto.id,
        name: dto.name,
        uri: dto.uri,
        kind: dto.kind,
        scope: dto.scope,
        bookmarkMissing: RootBookmarkStatus.isBookmarkMissing(
            kind: dto.kind,
            conversationId: conversationId,
            rootId: dto.id
        )
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
        capSampling: dto.capSampling,
        connectionStatus: dto.connectionStatus,
        lastError: dto.lastError,
        timeoutSecs: dto.timeoutSecs
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
        capSampling: record.capSampling,
        connectionStatus: record.connectionStatus,
        lastError: record.lastError,
        timeoutSecs: record.timeoutSecs
    )
}

private func appTemplateRecord(from dto: AppTemplateDto) -> AppTemplateRecord {
    AppTemplateRecord(
        templateRef: dto.templateRef,
        serverId: dto.serverId,
        html: dto.html,
        allowedOrigins: dto.allowedOrigins,
        bridgeScript: dto.bridgeScript,
        contentSecurityPolicy: dto.contentSecurityPolicy
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

func configureDevelopmentGatewayStdioHelper() {
    if ProcessInfo.processInfo.environment["TAMTRI_GATEWAY_STDIO_HELPER"] != nil {
        return
    }
    let fileManager = FileManager.default
    let cwd = fileManager.currentDirectoryPath
    let home = fileManager.homeDirectoryForCurrentUser.path
    let candidates = [
        "\(cwd)/target/debug/tamtri-gateway-stdio",
        "\(cwd)/../target/debug/tamtri-gateway-stdio",
        "\(home)/Desktop/tamtri/target/debug/tamtri-gateway-stdio"
    ]
    guard let path = candidates.first(where: { fileManager.isExecutableFile(atPath: $0) }) else {
        return
    }
    setenv("TAMTRI_GATEWAY_STDIO_HELPER", path, 1)
}

func makeDefaultCoreClient() -> CoreClient {
    configureDevelopmentGatewayStdioHelper()
    let home = FileManager.default.homeDirectoryForCurrentUser
    let vaultURL = home.appendingPathComponent(".tamtri/vault", isDirectory: true)
    do {
        return try TamtriBindingClient(vaultPath: vaultURL.path)
    } catch {
        fputs("tamtri: failed to initialize native core: \(error)\n", stderr)
        if ProcessInfo.processInfo.environment["TAMTRI_USE_MOCK"] == "1" {
            fputs("tamtri: using MockCoreClient (TAMTRI_USE_MOCK=1)\n", stderr)
            return MockCoreClient()
        }
        fatalError("tamtri failed to initialize native core: \(error)")
    }
}
