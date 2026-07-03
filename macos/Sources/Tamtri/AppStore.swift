import AppKit
import Foundation

@MainActor
final class AppStore: ObservableObject {
    @Published var conversations: [ConversationSummary] = []
    @Published var selectedConversation: ConversationRecord?
    @Published var liveEvents: [CoreEvent] = []
    @Published var composerText = ""
    @Published var workdirFiles: [WorkdirFileRecord] = []
    @Published var selectedWorkdirFile: WorkdirFileRecord?
    @Published var workdirPreview: WorkdirFilePreview?
    @Published var showFilesPanel = true
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
                    if event.kind == "turn_ended" || event.kind == "message_committed" {
                        let preferNewest = event.kind == "turn_ended"
                        Task { await self.refreshWorkdirFiles(preferNewest: preferNewest) }
                    }
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
            await refreshWorkdirFiles()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func select(_ summary: ConversationSummary) {
        Task {
            do {
                selectedWorkdirFile = nil
                workdirPreview = nil
                selectedConversation = try await core.loadConversation(id: summary.id)
                await refreshWorkdirFiles()
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
                    await refreshWorkdirFiles()
                }
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    func refreshWorkdirFiles(preferNewest: Bool = false) async {
        guard let conversation = selectedConversation else {
            workdirFiles = []
            selectedWorkdirFile = nil
            workdirPreview = nil
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
