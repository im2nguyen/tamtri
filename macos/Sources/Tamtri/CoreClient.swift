import Foundation

struct ConversationSummary: Identifiable, Equatable {
    let id: String
    let title: String
    let updatedAt: String
}

struct ConversationRecord: Equatable {
    let id: String
    let title: String
    let harnessId: String?
    let modelId: String?
    let messagesJSON: [String]
}

struct CoreEvent: Equatable {
    let conversationId: String
    let kind: String
    let payloadJSON: String
}

protocol CoreClient: Sendable {
    var events: AsyncStream<CoreEvent> { get }

    func listConversations() async throws -> [ConversationSummary]
    func loadConversation(id: String) async throws -> ConversationRecord
    func createConversation(title: String, harnessId: String, modelId: String) async throws -> ConversationRecord
    func sendMessage(conversationId: String, text: String) async throws
    func respondPermission(conversationId: String, requestId: String, optionId: String) async throws
    func cancelRun(conversationId: String) async throws
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
            messagesJSON: []
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
        let record = ConversationRecord(id: UUID().uuidString, title: title, harnessId: harnessId, modelId: modelId, messagesJSON: [])
        conversations.insert(record, at: 0)
        return record
    }

    func sendMessage(conversationId: String, text: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "text_delta", payloadJSON: #"{"text":"Thinking about it..."}"#))
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "permission_requested", payloadJSON: #"{"request_id":"mock-permission","action":"edit","options":[{"id":"allow_once","label":"Allow once"},{"id":"deny","label":"Deny"}]}"#))
    }

    func respondPermission(conversationId: String, requestId: String, optionId: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "permission_resolved", payloadJSON: "{}"))
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "message_committed", payloadJSON: #"{"content":[{"type":"text","text":"Done."}]}"#))
    }

    func cancelRun(conversationId: String) async throws {
        continuation.yield(CoreEvent(conversationId: conversationId, kind: "turn_ended", payloadJSON: #"{"reason":"cancelled"}"#))
    }
}
