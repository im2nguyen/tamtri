import Foundation

@MainActor
final class AppStore: ObservableObject {
    @Published var conversations: [ConversationSummary] = []
    @Published var selectedConversation: ConversationRecord?
    @Published var liveEvents: [CoreEvent] = []
    @Published var composerText = ""
    @Published var showNewConversation = false
    @Published var showSettings = false
    @Published var showForkConversation = false
    @Published var gatewayServers: [GatewayServerRecord] = []
    @Published var errorMessage: String?

    private let core: CoreClient

    init(core: CoreClient) {
        self.core = core
        Task {
            for await event in core.events {
                await MainActor.run {
                    self.liveEvents.append(event)
                }
            }
        }
    }

    func refresh() async {
        do {
            conversations = try await core.listConversations()
            if selectedConversation == nil, let first = conversations.first {
                selectedConversation = try await core.loadConversation(id: first.id)
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func select(_ summary: ConversationSummary) {
        Task {
            do {
                selectedConversation = try await core.loadConversation(id: summary.id)
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func createConversation(title: String, harnessId: String, modelId: String) {
        Task {
            do {
                selectedConversation = try await core.createConversation(title: title, harnessId: harnessId, modelId: modelId)
                await refresh()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func forkSelectedConversation(harnessId: String, modelId: String) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                selectedConversation = try await core.forkConversation(id: conversation.id, harnessId: harnessId, modelId: modelId)
                await refresh()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func send() {
        guard let conversation = selectedConversation else { return }
        let text = composerText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        composerText = ""
        Task {
            do {
                try await core.sendMessage(conversationId: conversation.id, text: text)
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func respondPermission(requestId: String, optionId: String) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                try await core.respondPermission(conversationId: conversation.id, requestId: requestId, optionId: optionId)
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func refreshGatewayServers() {
        Task {
            do {
                gatewayServers = try await core.listGatewayServers()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func setGatewayCredential(credentialRef: String, value: String) {
        Task {
            do {
                try await core.setGatewayCredential(credentialRef: credentialRef, value: value)
                refreshGatewayServers()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}
