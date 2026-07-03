import Foundation

enum FilesPanelCopy {
    static let artifactsSectionTitle = "Frozen attachments"
    static let artifactsSectionSubtitle = "Deliverables saved with the transcript. Portable in export."
    static let workdirSectionTitle = "Working files"
    static let workdirSectionSubtitle = "Live workspace. Files may change until you refresh."
    static let liveWorkingFileBadge = "Live working file"
    static let frozenAttachmentBadge = "Frozen attachment"
}

enum FileActionRole: Equatable {
    case previewPrimary
    case openSecondary
    case revealTertiary
}

struct FileActionButtonPresentation: Equatable {
    let label: String
    let systemImage: String
    let role: FileActionRole
    let isEnabled: Bool
}

struct FileRowActionsPresentation: Equatable {
    let preview: FileActionButtonPresentation?
    let open: FileActionButtonPresentation?
    let reveal: FileActionButtonPresentation?

    var primaryAction: FileActionButtonPresentation? {
        preview ?? open
    }

    var secondaryAction: FileActionButtonPresentation? {
        if preview != nil {
            return open
        }
        return nil
    }

    static func artifact(canPreviewInline: Bool, previewLoaded: Bool = false) -> FileRowActionsPresentation {
        if canPreviewInline {
            return FileRowActionsPresentation(
                preview: FileActionButtonPresentation(
                    label: previewLoaded ? "Previewing" : "Preview",
                    systemImage: "eye",
                    role: .previewPrimary,
                    isEnabled: !previewLoaded
                ),
                open: FileActionButtonPresentation(
                    label: "Open",
                    systemImage: "arrow.up.right.square",
                    role: .openSecondary,
                    isEnabled: true
                ),
                reveal: FileActionButtonPresentation(
                    label: "Reveal in Finder",
                    systemImage: "folder",
                    role: .revealTertiary,
                    isEnabled: true
                )
            )
        }
        return FileRowActionsPresentation(
            preview: nil,
            open: FileActionButtonPresentation(
                label: "Open",
                systemImage: "arrow.up.right.square",
                role: .openSecondary,
                isEnabled: true
            ),
            reveal: FileActionButtonPresentation(
                label: "Reveal in Finder",
                systemImage: "folder",
                role: .revealTertiary,
                isEnabled: true
            )
        )
    }

    static func workdir(canPreviewInline: Bool) -> FileRowActionsPresentation {
        FileRowActionsPresentation(
            preview: canPreviewInline
                ? FileActionButtonPresentation(
                    label: "Preview",
                    systemImage: "eye",
                    role: .previewPrimary,
                    isEnabled: true
                )
                : nil,
            open: FileActionButtonPresentation(
                label: "Open",
                systemImage: "arrow.up.right.square",
                role: .openSecondary,
                isEnabled: true
            ),
            reveal: FileActionButtonPresentation(
                label: "Reveal in Finder",
                systemImage: "folder",
                role: .revealTertiary,
                isEnabled: true
            )
        )
    }
}

enum DesignedErrorKind: Equatable {
    case emptyVault
    case malformedConversation
    case busyConversation
    case missingBookmark
    case unsupportedSchema
    case unavailableHarness
}

enum DesignedErrorRecovery: Equatable {
    case newConversation
    case revealInFinder(conversationId: String)
    case cancelRun
    case repickFolder
    case openHarnessHealth
    case forkConversation
    case wait
}

struct DesignedErrorActionPresentation: Equatable {
    let label: String
    let recovery: DesignedErrorRecovery
    let isPrimary: Bool
}

struct DesignedErrorState: Equatable, Identifiable {
    let kind: DesignedErrorKind
    let title: String
    let message: String
    let detail: String?
    let primaryAction: DesignedErrorActionPresentation
    let secondaryAction: DesignedErrorActionPresentation?
    let accessibilityLabel: String

    var id: String {
        switch kind {
        case .emptyVault: "empty-vault"
        case .malformedConversation: "malformed-conversation"
        case .busyConversation: "busy-conversation"
        case .missingBookmark: "missing-bookmark"
        case .unsupportedSchema: "unsupported-schema"
        case .unavailableHarness: "unavailable-harness"
        }
    }
}

enum TamtriErrorClassifier {
    static func coreMessage(from error: Error) -> String {
        if let tamtriError = error as? TamtriError {
            switch tamtriError {
            case .Core(let message):
                return message
            }
        }
        return error.localizedDescription
    }

    static func classify(
        message: String,
        conversationId: String? = nil,
        harnessId: String? = nil,
        rootName: String? = nil
    ) -> DesignedErrorState? {
        let lowered = message.lowercased()

        if lowered.contains("malformed vault") {
            return malformedConversation(message: message, conversationId: conversationId)
        }
        if lowered.contains("conversation is being written by another process") {
            return busyConversation(conversationId: conversationId)
        }
        if lowered.contains("unsupported schema version") {
            return unsupportedSchema(message: message, conversationId: conversationId)
        }
        if lowered.contains("unknown harness") || lowered.contains("unknown acp agent") {
            return unavailableHarness(message: message, harnessId: harnessId)
        }
        if lowered.contains("missing access bookmark") || lowered.contains("missing bookmark") {
            return missingBookmark(rootName: rootName)
        }
        return nil
    }

    static func emptyVaultState() -> DesignedErrorState {
        DesignedErrorState(
            kind: .emptyVault,
            title: "No conversations yet",
            message: "Create a conversation to start working with your files and agents.",
            detail: nil,
            primaryAction: DesignedErrorActionPresentation(
                label: "New Conversation",
                recovery: .newConversation,
                isPrimary: true
            ),
            secondaryAction: DesignedErrorActionPresentation(
                label: "Open Harness Health",
                recovery: .openHarnessHealth,
                isPrimary: false
            ),
            accessibilityLabel: "No conversations yet. Create a conversation to get started."
        )
    }

    static func malformedConversation(message: String, conversationId: String?) -> DesignedErrorState {
        DesignedErrorState(
            kind: .malformedConversation,
            title: "This conversation could not be read",
            message: "Something in the vault folder is damaged or incomplete. Other conversations are still available.",
            detail: message,
            primaryAction: DesignedErrorActionPresentation(
                label: "Reveal in Finder",
                recovery: .revealInFinder(conversationId: conversationId ?? ""),
                isPrimary: true
            ),
            secondaryAction: nil,
            accessibilityLabel: "Malformed conversation. Reveal in Finder to inspect the vault folder."
        )
    }

    static func busyConversation(conversationId: String?) -> DesignedErrorState {
        DesignedErrorState(
            kind: .busyConversation,
            title: "This conversation is busy",
            message: "Another tamtri window or an active run is writing to this conversation.",
            detail: conversationId,
            primaryAction: DesignedErrorActionPresentation(
                label: "Cancel Run",
                recovery: .cancelRun,
                isPrimary: true
            ),
            secondaryAction: DesignedErrorActionPresentation(
                label: "Wait",
                recovery: .wait,
                isPrimary: false
            ),
            accessibilityLabel: "Conversation busy. Cancel the active run or wait."
        )
    }

    static func missingBookmark(rootName: String?) -> DesignedErrorState {
        let name = rootName ?? "folder"
        return DesignedErrorState(
            kind: .missingBookmark,
            title: "Folder access needs to be restored",
            message: RootBookmarkStatus.missingBookmarkWarning,
            detail: RootBookmarkStatus.missingBookmarkError(rootName: name),
            primaryAction: DesignedErrorActionPresentation(
                label: "Re-pick Folder",
                recovery: .repickFolder,
                isPrimary: true
            ),
            secondaryAction: nil,
            accessibilityLabel: "Missing folder bookmark for \(name). Re-pick the folder to restore access."
        )
    }

    static func unsupportedSchema(message: String, conversationId: String?) -> DesignedErrorState {
        DesignedErrorState(
            kind: .unsupportedSchema,
            title: "This conversation needs a newer app",
            message: "The vault format is newer than this version of tamtri can read.",
            detail: message,
            primaryAction: DesignedErrorActionPresentation(
                label: "Reveal in Finder",
                recovery: .revealInFinder(conversationId: conversationId ?? ""),
                isPrimary: true
            ),
            secondaryAction: DesignedErrorActionPresentation(
                label: "Open Harness Health",
                recovery: .openHarnessHealth,
                isPrimary: false
            ),
            accessibilityLabel: "Unsupported conversation format. Update tamtri or reveal the folder in Finder."
        )
    }

    static func unavailableHarness(message: String, harnessId: String?) -> DesignedErrorState {
        let harness = harnessId ?? "agent"
        return DesignedErrorState(
            kind: .unavailableHarness,
            title: "The selected agent is unavailable",
            message: "tamtri could not start \"\(harness)\". Check install and auth in Harness Health.",
            detail: message,
            primaryAction: DesignedErrorActionPresentation(
                label: "Open Harness Health",
                recovery: .openHarnessHealth,
                isPrimary: true
            ),
            secondaryAction: DesignedErrorActionPresentation(
                label: "Fork Into…",
                recovery: .forkConversation,
                isPrimary: false
            ),
            accessibilityLabel: "Agent unavailable. Open Harness Health or fork into another agent."
        )
    }
}

enum FilePreviewSupport {
    static func canPreviewInline(mimeType: String?) -> Bool {
        artifactIsTextLikeMime(mimeType) || artifactIsImageMime(mimeType)
    }
}
