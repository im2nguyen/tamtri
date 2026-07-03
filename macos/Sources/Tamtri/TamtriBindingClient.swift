import Foundation

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

    func sendMessage(conversationId: String, text: String) async throws {
        try core.sendMessage(conversationId: conversationId, text: text)
    }

    func respondPermission(conversationId: String, requestId: String, optionId: String) async throws {
        try core.respondPermission(conversationId: conversationId, requestId: requestId, optionId: optionId)
    }

    func cancelRun(conversationId: String) async throws {
        try core.cancelRun(conversationId: conversationId)
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
