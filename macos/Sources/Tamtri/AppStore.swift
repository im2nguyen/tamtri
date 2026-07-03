import AppKit
import Foundation
import UniformTypeIdentifiers

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
    @Published var gatewayTools: [GatewayToolRecord] = []
    @Published var defaultCallTimeoutSecs: UInt64 = 300
    @Published var harnessAgents: [HarnessAgentRecord] = []
    @Published var errorMessage: String?
    @Published var isLoadingConversation = false
    @Published var bridgeDelivery: BridgeDelivery?
    @Published var liveTaskStates: [String: LiveTaskState] = [:]
    @Published var showConversationRoots = false
    @Published var showHarnessHealth = false
    @Published var showSearch = false
    @Published var harnessHealthEntries: [HarnessHealthRecord] = []
    @Published var searchQuery = ""
    @Published var searchResults: [SearchHitRecord] = []
    @Published var searchScopeMessage = ""
    @Published var importSummaryMessage: String?
    @Published var designedErrorState: DesignedErrorState?
    @Published var missingBookmarkState: DesignedErrorState?
    @Published private(set) var isRunActive = false

    var hasReadyHarness: Bool {
        harnessHealthEntries.contains { $0.status == "ready" }
    }

    var isVaultEmpty: Bool {
        conversations.isEmpty
    }

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
                    if event.kind == "turn_started", event.conversationId == self.selectedConversationId {
                        self.isRunActive = true
                    }
                    if event.kind == "turn_ended" || event.kind == "message_committed" {
                        self.clearLiveEvents(for: event.conversationId)
                        self.conversationCache.removeValue(forKey: event.conversationId)
                        let preferNewest = event.kind == "turn_ended"
                        Task { await self.refreshWorkdirFiles(preferNewest: preferNewest, force: preferNewest) }
                        if event.conversationId == self.selectedConversationId {
                            if event.kind == "message_committed" || event.kind == "turn_ended" {
                                Task { await self.reloadSelectedConversation() }
                            }
                        }
                    }
                    if event.kind == "turn_ended", event.conversationId == self.selectedConversationId {
                        self.isRunActive = false
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
            if conversations.isEmpty {
                designedErrorState = TamtriErrorClassifier.emptyVaultState()
                selectedConversationId = nil
                selectedConversation = nil
            } else if designedErrorState?.kind == .emptyVault {
                designedErrorState = nil
            }
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
            presentError(error)
        }
    }

    func selectConversation(_ summary: ConversationSummary) {
        designedErrorState = nil
        if selectedConversation?.id == summary.id {
            selectedConversationId = summary.id
            return
        }
        if pendingSelectionId == summary.id, isLoadingConversation {
            return
        }

        clearLiveEvents()
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
            selectedConversation = nil
            if let state = classifiedErrorState(error, conversationId: id) {
                designedErrorState = state
            } else {
                errorMessage = TamtriErrorClassifier.coreMessage(from: error)
            }
        }
    }

    private func clearLiveEvents(for conversationId: String? = nil) {
        if let conversationId {
            liveEvents.removeAll { $0.conversationId == conversationId }
        } else {
            liveEvents.removeAll()
        }
        liveTaskStates = [:]
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
        Task { await refreshMissingBookmarkState() }
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
        isRunActive = true
        Task {
            do {
                let roots = try await listRoots(conversationId: conversation.id)
                let resolved = try RootBookmarkAccess.beginAccess(conversationId: conversation.id, roots: roots)
                try await core.syncRuntimeRoots(conversationId: conversation.id, roots: resolved)
                try await core.sendMessage(conversationId: conversation.id, text: text)
            } catch {
                isRunActive = false
                if let state = classifiedErrorState(
                    error,
                    conversationId: conversation.id,
                    harnessId: conversation.harnessId
                ) {
                    designedErrorState = state
                } else {
                    errorMessage = TamtriErrorClassifier.coreMessage(from: error)
                }
            }
        }
    }

    func cancelRun() {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                try await core.cancelRun(conversationId: conversation.id)
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

    func openWorkdirFile(_ file: WorkdirFileRecord) {
        guard let conversation = selectedConversation else { return }
        Task {
            do {
                let workdir = try await core.conversationWorkdirPath(conversationId: conversation.id)
                let path = (workdir as NSString).appendingPathComponent(file.relativePath)
                NSWorkspace.shared.open(URL(fileURLWithPath: path))
            } catch {
                presentError(error)
            }
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

    func presentConversationRoots() {
        showConversationRoots = true
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

    func refreshGatewayServers() async {
        await reloadGatewayServers()
    }

    func refreshGatewayCapabilities() {
        Task {
            do {
                gatewayServers = try await core.refreshGatewayCapabilities()
                gatewayTools = try await core.listGatewayTools()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func refreshHarnessAgents() async {
        do {
            harnessAgents = try await core.listAcpAgents()
            await refreshHarnessHealth()
        } catch {
            presentError(error)
        }
    }

    func refreshHarnessHealth() async {
        do {
            harnessHealthEntries = try await core.listHarnessHealth()
        } catch {
            presentError(error)
        }
    }

    func harnessHealthChecklist() async -> String {
        do {
            return try await core.harnessHealthChecklist()
        } catch {
            presentError(error)
            return ""
        }
    }

    func evaluateFirstRunHarnessHealth() async {
        await refreshHarnessHealth()
        if !hasReadyHarness {
            showHarnessHealth = true
        }
    }

    func loadSearchScopeMessage() async {
        searchScopeMessage = await core.searchScopeMessage()
    }

    func runSearch() {
        Task {
            do {
                searchResults = try await core.searchConversations(query: searchQuery)
                if searchScopeMessage.isEmpty {
                    await loadSearchScopeMessage()
                }
            } catch {
                presentError(error)
            }
        }
    }

    func selectSearchHit(_ hit: SearchHitRecord) {
        if let summary = conversations.first(where: { $0.id == hit.conversationId }) {
            selectConversation(summary)
        } else {
            selectedConversationId = hit.conversationId
            Task { await refresh() }
        }
    }

    func exportSelectedConversation() {
        guard let conversationId = selectedConversationId else { return }
        let panel = NSSavePanel()
        if let tamtriType = UTType(filenameExtension: "tamtri") {
            panel.allowedContentTypes = [tamtriType]
        }
        panel.nameFieldStringValue = "\(displayedConversation?.title ?? "conversation").tamtri"
        panel.begin { response in
            guard response == .OK, let url = panel.url else { return }
            Task {
                do {
                    try await self.core.exportConversationBundle(
                        conversationId: conversationId,
                        destPath: url.path
                    )
                } catch {
                    await MainActor.run { self.presentError(error) }
                }
            }
        }
    }

    func importConversationBundle() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        if let tamtriType = UTType(filenameExtension: "tamtri") {
            panel.allowedContentTypes = [tamtriType, .folder]
        }
        panel.begin { response in
            guard response == .OK, let url = panel.url else { return }
            Task {
                do {
                    let result = try await self.core.importBundleOrFolder(sourcePath: url.path)
                    await self.refresh()
                    self.selectedConversationId = result.conversation.id
                    self.selectedConversation = result.conversation
                    self.conversationCache[result.conversation.id] = result.conversation
                    if result.warnings.isEmpty {
                        self.importSummaryMessage = "Imported \"\(result.conversation.title)\" with no warnings."
                    } else {
                        let lines = result.warnings.map { "• \($0.detail)" }.joined(separator: "\n")
                        self.importSummaryMessage =
                            "Imported \"\(result.conversation.title)\" with \(result.warnings.count) warning(s):\n\(lines)"
                    }
                } catch {
                    await MainActor.run { self.presentError(error) }
                }
            }
        }
    }

    private func presentError(_ error: Error) {
        let message = TamtriErrorClassifier.coreMessage(from: error)
        if let state = classifiedErrorState(error, conversationId: selectedConversationId) {
            designedErrorState = state
            if state.kind == .unavailableHarness {
                showHarnessHealth = true
            }
            return
        }
        errorMessage = message
    }

    func performDesignedErrorRecovery(_ recovery: DesignedErrorRecovery) {
        switch recovery {
        case .newConversation:
            designedErrorState = nil
            showNewConversation = true
        case .revealInFinder(let conversationId):
            revealConversationFolder(conversationId: conversationId)
        case .cancelRun:
            cancelRun()
            designedErrorState = nil
        case .repickFolder:
            showConversationRoots = true
        case .openHarnessHealth:
            showHarnessHealth = true
        case .forkConversation:
            showForkConversation = true
        case .wait:
            designedErrorState = nil
        }
    }

    func dismissMissingBookmarkState() {
        missingBookmarkState = nil
    }

    func revealConversationFolder(conversationId: String) {
        guard !conversationId.isEmpty else { return }
        Task {
            do {
                let path = try await core.conversationFolderPath(conversationId: conversationId)
                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
            } catch {
                presentError(error)
            }
        }
    }

    func refreshMissingBookmarkState() async {
        guard let conversation = selectedConversation else {
            missingBookmarkState = nil
            return
        }
        do {
            let roots = try await listRoots(conversationId: conversation.id)
            if let missing = roots.first(where: { $0.bookmarkMissing && $0.kind == "filesystem" }) {
                missingBookmarkState = TamtriErrorClassifier.missingBookmark(rootName: missing.name)
            } else {
                missingBookmarkState = nil
            }
        } catch {
            missingBookmarkState = nil
        }
    }

    private func classifiedErrorState(
        _ error: Error,
        conversationId: String?,
        harnessId: String? = nil,
        rootName: String? = nil
    ) -> DesignedErrorState? {
        TamtriErrorClassifier.classify(
            message: TamtriErrorClassifier.coreMessage(from: error),
            conversationId: conversationId,
            harnessId: harnessId,
            rootName: rootName
        )
    }

    func listAgentModels(agentId: String) async -> [ModelInfoRecord] {
        do {
            return try await core.listAcpAgentModels(agentId: agentId)
        } catch {
            errorMessage = error.localizedDescription
            return []
        }
    }

    func saveGatewayDefaultTimeout(_ seconds: UInt64) {
        Task {
            do {
                try await core.setGatewayDefaultTimeout(seconds)
                defaultCallTimeoutSecs = seconds
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func reloadGatewayServers() async {
        do {
            gatewayServers = try await core.listGatewayServers()
            gatewayTools = try await core.listGatewayTools()
            defaultCallTimeoutSecs = try await core.getGatewaySettings()
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
                await refreshGatewayServers()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func prepareForAppQuitSync() {
        try? core.prepareForAppQuitSync()
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
                            await self.refreshGatewayServers()
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
