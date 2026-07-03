import AppKit
import AppKit
import Foundation

@MainActor
final class AppStore: ObservableObject {
    @Published var conversations: [ConversationSummary] = []
    @Published var selectedConversationId: String?
    @Published var selectedConversation: ConversationRecord?
    @Published var liveEvents: [CoreEvent] = []
    @Published var composerText = ""
    @Published var workdirFiles: [WorkdirFileRecord] = []
    @Published var selectedWorkdirFile: WorkdirFileRecord?
    @Published var workdirPreview: WorkdirFilePreview?
    @Published var showFilesPanel = false
    @Published var showNewConversation = false
    @Published var showSettings = false
    @Published var showForkConversation = false
    @Published var gatewayServers: [GatewayServerRecord] = []
    @Published var errorMessage: String?
    @Published var isLoadingConversation = false

    private let core: CoreClient
    private var conversationCache: [String: ConversationRecord] = [:]
    private var pendingSelectionId: String?

    var isSwitchingConversation: Bool {
        guard let targetId = selectedConversationId else { return false }
        return isLoadingConversation || selectedConversation?.id != targetId
    }

    var displayedConversation: ConversationRecord? {
        guard let targetId = selectedConversationId,
              let conversation = selectedConversation,
              conversation.id == targetId
        else {
            return nil
        }
        return conversation
    }

    init(core: CoreClient) {
        self.core = core
        Task {
            for await event in core.events {
                await MainActor.run {
                    self.liveEvents.append(event)
                    if event.kind == "turn_ended" || event.kind == "message_committed" {
                        self.conversationCache.removeValue(forKey: event.conversationId)
                        let preferNewest = event.kind == "turn_ended"
                        Task { await self.refreshWorkdirFiles(preferNewest: preferNewest, force: preferNewest) }
                    }
                    if event.kind == "gateway_credential_updated" {
                        Task { @MainActor in
                            await self.persistUpdatedCredentialToKeychain(payloadJSON: event.payloadJSON)
                        }
                    }
                }
            }
        }
    }

    private func persistUpdatedCredentialToKeychain(payloadJSON: String) async {
        guard let data = payloadJSON.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let credentialRef = object["credential_ref"] as? String,
              !credentialRef.isEmpty
        else {
            return
        }
        do {
            if let value = try await core.exportGatewayCredential(credentialRef: credentialRef) {
                try KeychainCredentialStore.save(value: value, for: credentialRef)
            }
        } catch {
            // Keychain persistence is best-effort; avoid failing the UI event loop.
        }
    }

    func refresh() async {
        do {
            conversations = try await core.listConversations()
            if selectedConversation == nil, let first = conversations.first {
                selectedConversationId = first.id
                if let cached = conversationCache[first.id] {
                    applySelection(cached)
                } else {
                    await loadConversation(id: first.id, summary: first)
                }
            }
            await refreshWorkdirFiles()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func selectConversation(_ summary: ConversationSummary) {
        if selectedConversation?.id == summary.id {
            selectedConversationId = summary.id
            return
        }
        if pendingSelectionId == summary.id, isLoadingConversation {
            return
        }

        selectedConversationId = summary.id

        if let cached = conversationCache[summary.id] {
            applySelection(cached)
            return
        }

        Task { @MainActor in
            await loadConversation(id: summary.id, summary: summary)
        }
    }

    private func loadConversation(id: String, summary: ConversationSummary?) async {
        pendingSelectionId = id
        isLoadingConversation = true
        let started = ContinuousClock.now

        do {
            let record = try await core.loadConversation(id: id)
            let elapsed = started.duration(to: .now)
            #if DEBUG
            print("[tamtri] load conversation \(id) in \(elapsed)")
            #endif

            conversationCache[id] = record
            guard pendingSelectionId == id, selectedConversationId == id else { return }
            isLoadingConversation = false
            applySelection(record)
        } catch {
            guard pendingSelectionId == id, selectedConversationId == id else { return }
            isLoadingConversation = false
            errorMessage = error.localizedDescription
        }
    }

    private func applySelection(_ record: ConversationRecord) {
        selectedWorkdirFile = nil
        workdirPreview = nil
        selectedConversation = record
        if selectedConversationId != record.id {
            selectedConversationId = record.id
        }
        Task { await refreshWorkdirFiles() }
    }

    private func cacheConversation(_ record: ConversationRecord) {
        conversationCache[record.id] = record
    }

    func createConversation(title: String, harnessId: String, modelId: String) {
        Task {
            do {
                let record = try await core.createConversation(title: title, harnessId: harnessId, modelId: modelId)
                cacheConversation(record)
                selectedConversationId = record.id
                selectedConversation = record
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
                let record = try await core.forkConversation(id: conversation.id, harnessId: harnessId, modelId: modelId)
                cacheConversation(record)
                selectedConversationId = record.id
                selectedConversation = record
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

    func attachFiles(paths: [String]) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                var names: [String] = []
                for path in paths {
                    let name = try await core.copyFileToWorkdir(conversationId: conversation.id, sourcePath: path)
                    names.append(name)
                }
                if !names.isEmpty {
                    let attachmentText = names.map { "Attached: \($0)" }.joined(separator: "\n")
                    if composerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        composerText = attachmentText
                    } else {
                        composerText += "\n\(attachmentText)"
                    }
                    await refreshWorkdirFiles(force: true)
                }
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func refreshWorkdirFiles(preferNewest: Bool = false, force: Bool = false) async {
        guard let conversation = selectedConversation else {
            workdirFiles = []
            selectedWorkdirFile = nil
            workdirPreview = nil
            return
        }
        guard showFilesPanel || force else {
            return
        }
        do {
            workdirFiles = try await core.listWorkdirFiles(conversationId: conversation.id)
            if let selected = selectedWorkdirFile,
               !workdirFiles.contains(where: { $0.relativePath == selected.relativePath }) {
                selectedWorkdirFile = nil
                workdirPreview = nil
            }
            if preferNewest, let newest = workdirFiles.max(by: { $0.modifiedAt < $1.modifiedAt }) {
                await selectWorkdirFile(newest)
            } else if selectedWorkdirFile == nil, let fallback = workdirFiles.max(by: { $0.modifiedAt < $1.modifiedAt }) {
                await selectWorkdirFile(fallback)
            } else if let selected = selectedWorkdirFile {
                await loadWorkdirPreview(for: selected)
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func selectWorkdirFile(_ file: WorkdirFileRecord) async {
        selectedWorkdirFile = file
        showFilesPanel = true
        await loadWorkdirPreview(for: file)
    }

    func loadWorkdirPreview(for file: WorkdirFileRecord) async {
        guard let conversation = selectedConversation else { return }
        do {
            workdirPreview = try await core.readWorkdirFile(
                conversationId: conversation.id,
                relativePath: file.relativePath
            )
        } catch {
            workdirPreview = WorkdirFilePreview(
                relativePath: file.relativePath,
                mimeType: file.mimeType,
                text: nil,
                imageData: nil,
                error: error.localizedDescription
            )
        }
    }

    func revealWorkdirFile(_ file: WorkdirFileRecord) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                let workdir = try await core.conversationWorkdirPath(conversationId: conversation.id)
                let path = (workdir as NSString).appendingPathComponent(file.relativePath)
                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func readAttachmentVerified(conversationId: String, path: String, size: UInt64, sha256: String) async throws -> Data {
        try await core.readAttachmentVerified(conversationId: conversationId, path: path, size: size, sha256: sha256)
    }

    func verifyArtifactInline(size: UInt64, sha256: String, inline: String) async throws {
        try await core.verifyArtifactInline(size: size, sha256: sha256, inlineContent: inline)
    }

    func logBlockedNavigation(url: String) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                try await core.logArtifactNavigationBlocked(conversationId: conversation.id, url: url)
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func revealWorkdir() {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                let path = try await core.conversationWorkdirPath(conversationId: conversation.id)
                var isDirectory: ObjCBool = false
                if FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory) {
                    NSWorkspace.shared.open(URL(fileURLWithPath: path, isDirectory: true))
                } else {
                    try FileManager.default.createDirectory(atPath: path, withIntermediateDirectories: true)
                    NSWorkspace.shared.open(URL(fileURLWithPath: path, isDirectory: true))
                }
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func revealAttachment(conversationId: String, path: String, size: UInt64, sha256: String) {
        Task {
            do {
                let verifiedPath = try await core.verifiedAttachmentPath(conversationId: conversationId, path: path, size: size, sha256: sha256)
                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: verifiedPath)])
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func openAttachment(conversationId: String, path: String, size: UInt64, sha256: String) {
        Task {
            do {
                let verifiedPath = try await core.verifiedAttachmentPath(conversationId: conversationId, path: path, size: size, sha256: sha256)
                NSWorkspace.shared.open(URL(fileURLWithPath: verifiedPath))
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

    func respondElicitation(requestId: String, action: String, dataJSON: String?) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                try await core.respondElicitation(
                    conversationId: conversation.id,
                    requestId: requestId,
                    action: action,
                    dataJSON: dataJSON
                )
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func refreshGatewayServers() {
        Task {
            await reloadGatewayServers()
        }
    }

    func refreshGatewayCapabilities() {
        Task {
            do {
                gatewayServers = try await core.refreshGatewayCapabilities()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func reloadGatewayServers() async {
        do {
            gatewayServers = try await core.listGatewayServers()
            for server in gatewayServers where !server.oauthTokenRef.isEmpty {
                if let stored = OAuthTokenStore.load(for: server.oauthTokenRef) {
                    try await core.setGatewayCredential(
                        credentialRef: server.oauthTokenRef,
                        value: stored
                    )
                }
            }
            gatewayServers = try await core.listGatewayServers()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func saveGatewayServers(_ servers: [GatewayServerRecord]) {
        Task {
            do {
                try await core.saveGatewayServers(servers)
                await reloadGatewayServers()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func removeGatewayServer(id: String) {
        let updated = gatewayServers.filter { $0.id != id }
        gatewayServers = updated
        saveGatewayServers(updated)
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

    private var oauthListener: OAuthLoopbackListener?

    func connectOAuth(for server: GatewayServerRecord) {
        Task {
            do {
                let redirectURI = "http://127.0.0.1:3847/callback"
                let listener = OAuthLoopbackListener(port: 3847)
                oauthListener = listener
                try listener.start { [weak self] callbackURL in
                    Task { @MainActor in
                        guard let self else { return }
                        do {
                            _ = try await self.core.completeOAuthCallback(callbackURL: callbackURL)
                            self.oauthListener = nil
                            self.refreshGatewayServers()
                        } catch {
                            self.errorMessage = error.localizedDescription
                        }
                    }
                }
                let handoff = try await core.startOAuthFlow(
                    serverId: server.id,
                    redirectURI: redirectURI
                )
                guard let url = URL(string: handoff.authorizationURL) else {
                    throw NSError(domain: "tamtri.oauth", code: 2)
                }
                NSWorkspace.shared.open(url)
            } catch {
                oauthListener?.stop()
                oauthListener = nil
                errorMessage = error.localizedDescription
            }
        }
    }
}
