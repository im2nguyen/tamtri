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

    func cancelRun(conversationId: String) async throws {
        try core.cancelRun(conversationId: conversationId)
    }

    func listGatewayServers() async throws -> [GatewayServerRecord] {
        try core.listGatewayServers().map {
            GatewayServerRecord(
                id: $0.id,
                displayName: $0.displayName,
                enabled: $0.enabled,
                scope: $0.scope,
                transport: $0.transport,
                credentialRefs: $0.credentialRefs,
                missingCredentialRefs: $0.missingCredentialRefs
            )
        }
    }

    func setGatewayCredential(credentialRef: String, value: String) async throws {
        try KeychainCredentialStore.save(value: value, for: credentialRef)
        try core.setGatewayCredential(credentialRef: credentialRef, value: value)
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

private enum KeychainCredentialStore {
    static func save(value: String, for credentialRef: String) throws {
        let data = Data(value.utf8)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "tamtri.gateway",
            kSecAttrAccount as String: credentialRef
        ]
        SecItemDelete(query as CFDictionary)
        var item = query
        item[kSecValueData as String] = data
        let status = SecItemAdd(item as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw NSError(domain: NSOSStatusErrorDomain, code: Int(status))
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
        messagesJSON: dto.messagesJson
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
