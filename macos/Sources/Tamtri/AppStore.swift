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
    @Published var transcriptArtifacts: [TranscriptArtifactRecord] = []
    @Published var selectedTranscriptArtifact: TranscriptArtifactRecord?
    @Published var attachmentPreview: AttachmentFilePreview?
    @Published var showFilesPanel = false
    @Published var showNewConversation = false
    @Published var showSettings = false
    @Published var showForkConversation = false
    @Published var gatewayServers: [GatewayServerRecord] = []
    @Published var errorMessage: String?
    @Published var isLoadingConversation = false
    @Published var bridgeDelivery: BridgeDelivery?
    @Published var liveTaskStates: [String: LiveTaskState] = [:]
    @Published var showConversationRoots = false

    private let core: CoreClient
    private var conversationCache: [String: ConversationRecord] = [:]
    private var pendingSelectionId: String?
    private var pendingBridgeTargets: [String: UUID] = [:]

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
                        if event.kind == "turn_ended", event.conversationId == self.selectedConversationId {
                            Task { await self.reloadSelectedConversation() }
                        }
                    }
                    if event.kind == "gateway_credential_updated" {
                        Task { @MainActor in
                            await self.persistUpdatedCredentialToKeychain(payloadJSON: event.payloadJSON)
                        }
                    }
                    if event.kind == "app_bridge_resolved" {
                        self.handleAppBridgeResolved(payloadJSON: event.payloadJSON)
                    }
                    if event.kind == "task_started" || event.kind == "task_updated" {
                        let state = LiveTaskState(payloadJSON: event.payloadJSON)
                        self.liveTaskStates[state.taskId] = state
                    }
                    if event.kind == "task_completed" {
                        let state = LiveTaskState(payloadJSON: event.payloadJSON)
                        self.liveTaskStates[state.taskId] = state
                    }
                    if event.kind == "turn_ended" {
                        self.liveTaskStates = [:]
                        RootBookmarkAccess.endAccess(conversationId: event.conversationId)
                    }
                }
            }
        }
    }

    private func handleAppBridgeResolved(payloadJSON: String) {
        guard let data = payloadJSON.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let requestId = object["request_id"] as? String,
              let responseJSON = object["response_json"] as? String,
              let webViewID = pendingBridgeTargets.removeValue(forKey: requestId)
        else {
            return
        }
        bridgeDelivery = BridgeDelivery(webViewID: webViewID, responseJSON: responseJSON)
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
        selectedTranscriptArtifact = nil
        attachmentPreview = nil
        transcriptArtifacts = TranscriptArtifacts.extract(from: record.parsedMessages)
        selectedConversation = record
        if selectedConversationId != record.id {
            selectedConversationId = record.id
        }
        Task { await refreshWorkdirFiles() }
    }

    private func reloadSelectedConversation() async {
        guard let id = selectedConversationId else { return }
        do {
            let record = try await core.loadConversation(id: id)
            conversationCache[id] = record
            guard selectedConversationId == id else { return }
            selectedConversation = record
            transcriptArtifacts = TranscriptArtifacts.extract(from: record.parsedMessages)
        } catch {
            errorMessage = error.localizedDescription
        }
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
                let roots = try await listRoots(conversationId: conversation.id)
                let resolved = try RootBookmarkAccess.beginAccess(conversationId: conversation.id, roots: roots)
                try await core.syncRuntimeRoots(conversationId: conversation.id, roots: resolved)
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
            transcriptArtifacts = []
            selectedTranscriptArtifact = nil
            attachmentPreview = nil
            return
        }
        transcriptArtifacts = TranscriptArtifacts.extract(from: conversation.parsedMessages)
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
        selectedTranscriptArtifact = nil
        attachmentPreview = nil
        showFilesPanel = true
        await loadWorkdirPreview(for: file)
    }

    func selectTranscriptArtifact(_ artifact: TranscriptArtifactRecord) async {
        selectedTranscriptArtifact = artifact
        selectedWorkdirFile = nil
        workdirPreview = nil
        showFilesPanel = true
        await loadAttachmentPreview(for: artifact)
    }

    func loadAttachmentPreview(for artifact: TranscriptArtifactRecord) async {
        guard let conversation = selectedConversation else { return }
        do {
            let data = try await core.readAttachmentVerified(
                conversationId: conversation.id,
                path: artifact.path,
                size: artifact.size,
                sha256: artifact.sha256
            )
            if artifactIsImageMime(artifact.mimeType) {
                attachmentPreview = AttachmentFilePreview(
                    path: artifact.path,
                    mimeType: artifact.mimeType,
                    text: nil,
                    imageData: data,
                    error: nil
                )
            } else if artifactIsTextLikeMime(artifact.mimeType), let text = String(data: data, encoding: .utf8) {
                attachmentPreview = AttachmentFilePreview(
                    path: artifact.path,
                    mimeType: artifact.mimeType,
                    text: text,
                    imageData: nil,
                    error: nil
                )
            } else {
                attachmentPreview = AttachmentFilePreview(
                    path: artifact.path,
                    mimeType: artifact.mimeType,
                    text: nil,
                    imageData: nil,
                    error: "No in-app preview for this file type."
                )
            }
        } catch {
            attachmentPreview = AttachmentFilePreview(
                path: artifact.path,
                mimeType: artifact.mimeType,
                text: nil,
                imageData: nil,
                error: error.localizedDescription
            )
        }
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

    func revealTranscriptArtifact(_ artifact: TranscriptArtifactRecord) {
        guard let conversation = selectedConversation else { return }
        revealAttachment(
            conversationId: conversation.id,
            path: artifact.path,
            size: artifact.size,
            sha256: artifact.sha256
        )
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

    func resolveAppTemplate(conversationId: String, serverId: String, templateRef: String) async throws -> AppTemplateRecord? {
        try await core.resolveAppTemplate(conversationId: conversationId, serverId: serverId, templateRef: templateRef)
    }

    func submitAppBridgeRequest(
        conversationId: String,
        serverId: String,
        appId: String,
        templateRef: String,
        requestJSON: String,
        webViewID: UUID
    ) {
        Task {
            do {
                let submission = try await core.submitAppBridgeRequest(
                    conversationId: conversationId,
                    serverId: serverId,
                    appId: appId,
                    templateRef: templateRef,
                    requestJSON: requestJSON
                )
                if submission.needsConsent {
                    pendingBridgeTargets[submission.requestId] = webViewID
                }
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func respondAppBridgeConsent(requestId: String, optionId: String) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                try await core.respondAppBridgeConsent(
                    conversationId: conversation.id,
                    requestId: requestId,
                    optionId: optionId
                )
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func logAppNavigationBlocked(conversationId: String, serverId: String, templateRef: String, url: String) {
        Task {
            do {
                try await core.logAppNavigationBlocked(
                    conversationId: conversationId,
                    serverId: serverId,
                    templateRef: templateRef,
                    url: url
                )
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func cancelTask(conversationId: String, taskId: String) {
        Task {
            do {
                try await core.cancelTask(conversationId: conversationId, taskId: taskId)
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func listRoots(conversationId: String) async throws -> [RootRecord] {
        try await core.listRoots(conversationId: conversationId)
    }

    func attachRoot(conversationId: String, name: String, uri: String, kind: String, scope: String) async throws -> RootRecord {
        try await core.attachRoot(conversationId: conversationId, name: name, uri: uri, kind: kind, scope: scope)
    }

    func removeRoot(conversationId: String, rootId: String) async throws {
        try await core.removeRoot(conversationId: conversationId, rootId: rootId)
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
            for server in gatewayServers {
                for credentialRef in server.credentialRefs {
                    if let stored = KeychainCredentialStore.load(for: credentialRef) {
                        try await core.setGatewayCredential(
                            credentialRef: credentialRef,
                            value: stored
                        )
                    }
                }
                if !server.oauthTokenRef.isEmpty,
                   let stored = OAuthTokenStore.load(for: server.oauthTokenRef) {
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
