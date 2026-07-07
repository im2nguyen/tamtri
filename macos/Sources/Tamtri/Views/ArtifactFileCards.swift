import SwiftUI

struct ArtifactFileCardGroup<Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(spacing: 0) {
            content()
        }
        .background(TamtriTheme.cardBackground(colorScheme), in: RoundedRectangle(cornerRadius: TamtriRadius.card))
        .overlay(
            RoundedRectangle(cornerRadius: TamtriRadius.card)
                .strokeBorder(TamtriTheme.hairline(colorScheme), lineWidth: 0.5)
        )
    }
}

struct ArtifactFileCardDivider: View {
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        Divider()
            .overlay(TamtriTheme.hairline(colorScheme))
            .padding(.leading, TamtriSpacing.md + 28 + TamtriSpacing.md)
    }
}

enum ArtifactFileRowStyle {
    case transcript
    case browse
}

struct ArtifactFileRowActions: View {
    let style: ArtifactFileRowStyle
    let onReveal: () -> Void
    let onOpen: () -> Void
    var revealEnabled = true
    var openEnabled = true

    var body: some View {
        switch style {
        case .transcript:
            HStack(spacing: TamtriSpacing.sm) {
                Button("Reveal in Finder", action: onReveal)
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .disabled(!revealEnabled)
                Button("Open", action: onOpen)
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .disabled(!openEnabled)
            }
        case .browse:
            Button(action: onOpen) {
                Label("Open", systemImage: "arrow.up.forward.square")
            }
            .labelStyle(.iconOnly)
            .help("Open")
            .buttonStyle(.bordered)
            .controlSize(.small)
            .disabled(!openEnabled)
        }
    }
}

struct ArtifactFileRowView: View {
    @Environment(\.colorScheme) private var colorScheme
    let style: ArtifactFileRowStyle
    let fileName: String
    let subtitle: String?
    let systemImage: String
    var iconColor: Color = .teal
    var isSelected = false
    var canPreview = true
    let accessibilityLabel: String
    let onRowTap: () -> Void
    let onReveal: () -> Void
    let onOpen: () -> Void
    var actionsEnabled = true

    @State private var isHovered = false

    var body: some View {
        HStack(spacing: TamtriSpacing.md) {
            Button(action: onRowTap) {
                HStack(spacing: TamtriSpacing.md) {
                    Image(systemName: systemImage)
                        .font(.title3)
                        .foregroundStyle(iconColor)
                        .frame(width: 28)
                    if let subtitle {
                        VStack(alignment: .leading, spacing: TamtriSpacing.xs) {
                            Text(fileName)
                                .font(TamtriTheme.rowTitleFont())
                                .lineLimit(1)
                                .foregroundStyle(.primary)
                            Text(subtitle)
                                .font(TamtriTheme.metadataFont())
                                .foregroundStyle(.secondary)
                        }
                    } else {
                        Text(fileName)
                            .font(TamtriTheme.rowTitleFont())
                            .lineLimit(1)
                            .foregroundStyle(.primary)
                    }
                    Spacer(minLength: 0)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.tamtriPlain)
            .disabled(!canPreview)

            ArtifactFileRowActions(
                style: style,
                onReveal: onReveal,
                onOpen: onOpen,
                revealEnabled: actionsEnabled,
                openEnabled: actionsEnabled
            )
        }
        .padding(TamtriSpacing.md)
        .background(rowBackground, in: RoundedRectangle(cornerRadius: TamtriRadius.card - 2))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(accessibilityLabel)
        .accessibilityAction(named: "Open preview") {
            guard canPreview else { return }
            onRowTap()
        }
        .accessibilityAction(named: "Reveal in Finder") {
            guard style == .transcript, actionsEnabled else { return }
            onReveal()
        }
        .accessibilityAction(named: "Open") {
            guard actionsEnabled else { return }
            onOpen()
        }
        .onHover { isHovered = $0 }
    }

    private var rowBackground: Color {
        if isSelected {
            return Color.accentColor.opacity(0.12)
        }
        if isHovered {
            return TamtriTheme.sidebarHover(colorScheme)
        }
        return .clear
    }
}

struct ArtifactCardGroup: View {
    @EnvironmentObject private var store: AppStore
    let conversationId: String
    let blocks: [TranscriptContentBlock]

    var body: some View {
        ArtifactFileCardGroup {
            ForEach(Array(blocks.enumerated()), id: \.offset) { index, block in
                if index > 0 {
                    ArtifactFileCardDivider()
                }
                ArtifactTranscriptFileRow(conversationId: conversationId, block: block)
            }
        }
    }
}

private struct ArtifactTranscriptFileRow: View {
    @EnvironmentObject private var store: AppStore
    let conversationId: String
    let block: TranscriptContentBlock

    private var integrityFailed: Bool { block.integrityFailed == true }

    private var fileName: String {
        (block.path as NSString?)?.lastPathComponent ?? "Artifact"
    }

    private var subtitle: String {
        var parts = [artifactMimeLabel(block.mimeType)]
        if let size = block.size {
            parts.append(ByteCountFormatter.string(fromByteCount: Int64(size), countStyle: .file))
        }
        if integrityFailed {
            parts.append("Integrity failed")
        }
        return parts.joined(separator: " · ")
    }

    var body: some View {
        ArtifactFileRowView(
            style: .transcript,
            fileName: fileName,
            subtitle: subtitle,
            systemImage: artifactFileIcon(block.mimeType),
            iconColor: integrityFailed ? .red : .teal,
            canPreview: !integrityFailed,
            accessibilityLabel: artifactCardAccessibilityLabel(
                path: block.path,
                mimeType: block.mimeType,
                integrityFailed: integrityFailed
            ),
            onRowTap: {
                Task { await store.openArtifactFromTranscript(block: block) }
            },
            onReveal: { revealAttachment() },
            onOpen: { openAttachment() },
            actionsEnabled: block.path != nil && block.size != nil && block.sha256 != nil
        )
    }

    private func revealAttachment() {
        guard let path = block.path, let size = block.size, let sha256 = block.sha256 else { return }
        store.revealAttachment(conversationId: conversationId, path: path, size: size, sha256: sha256)
    }

    private func openAttachment() {
        guard let path = block.path, let size = block.size, let sha256 = block.sha256 else { return }
        store.openAttachment(conversationId: conversationId, path: path, size: size, sha256: sha256)
    }
}

struct ArtifactBrowseWorkdirCardGroup: View {
    @EnvironmentObject private var store: AppStore
    let files: [WorkdirFileRecord]

    var body: some View {
        ArtifactFileCardGroup {
            ForEach(Array(files.enumerated()), id: \.element.relativePath) { index, file in
                if index > 0 {
                    ArtifactFileCardDivider()
                }
                ArtifactBrowseWorkdirRow(
                    file: file,
                    isSelected: store.selectedWorkdirFile?.relativePath == file.relativePath
                )
            }
        }
    }
}

private struct ArtifactBrowseWorkdirRow: View {
    @EnvironmentObject private var store: AppStore
    let file: WorkdirFileRecord
    let isSelected: Bool

    var body: some View {
        ArtifactFileRowView(
            style: .browse,
            fileName: file.relativePath,
            subtitle: nil,
            systemImage: workdirFileIcon(file.mimeType),
            iconColor: .secondary,
            isSelected: isSelected,
            accessibilityLabel: file.relativePath,
            onRowTap: {
                Task { await store.openFilesPreviewWorkdir(file) }
            },
            onReveal: { store.revealWorkdirFile(file) },
            onOpen: { store.openWorkdirFile(file) }
        )
    }

    private func workdirFileIcon(_ mimeType: String?) -> String {
        artifactFileIcon(mimeType)
    }
}

struct ArtifactBrowseAttachmentCardGroup: View {
    @EnvironmentObject private var store: AppStore
    let artifacts: [TranscriptArtifactRecord]

    var body: some View {
        ArtifactFileCardGroup {
            ForEach(Array(artifacts.enumerated()), id: \.element.id) { index, artifact in
                if index > 0 {
                    ArtifactFileCardDivider()
                }
                ArtifactBrowseAttachmentRow(
                    artifact: artifact,
                    isSelected: store.selectedTranscriptArtifact?.id == artifact.id
                )
            }
        }
    }
}

private struct ArtifactBrowseAttachmentRow: View {
    @EnvironmentObject private var store: AppStore
    let artifact: TranscriptArtifactRecord
    let isSelected: Bool

    private var fileName: String {
        (artifact.path as NSString).lastPathComponent
    }

    var body: some View {
        ArtifactFileRowView(
            style: .browse,
            fileName: fileName,
            subtitle: nil,
            systemImage: artifactFileIcon(artifact.mimeType),
            iconColor: .teal,
            isSelected: isSelected,
            accessibilityLabel: fileName,
            onRowTap: {
                Task { await store.openFilesPreviewArtifact(artifact) }
            },
            onReveal: { store.revealTranscriptArtifact(artifact) },
            onOpen: {
                store.openAttachment(
                    conversationId: store.selectedConversation?.id ?? "",
                    path: artifact.path,
                    size: artifact.size,
                    sha256: artifact.sha256
                )
            }
        )
    }
}
