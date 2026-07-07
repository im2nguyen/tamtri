import SwiftUI
import WebKit
import AppKit
import Foundation

struct FilesPanelView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        Group {
            switch store.workspaceRailMode {
            case .closed:
                EmptyView()
            case .browse:
                FilesBrowsePanelView()
            case .preview:
                FilesPreviewPanelView()
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityLabel(store.workspaceRailMode == .preview ? "File preview" : "Working files and attachments")
    }
}

struct FilesBrowsePanelView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        Group {
            if store.transcriptArtifacts.isEmpty && store.workdirFiles.isEmpty {
                EmptyStateView(
                    systemImage: "doc",
                    title: "No files yet",
                    message: "Drop files into the composer or ask the agent to create them.",
                    primaryActionTitle: nil,
                    onPrimary: nil
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                VStack(spacing: 0) {
                    browseHeader
                    ScrollView {
                        VStack(alignment: .leading, spacing: TamtriSpacing.md) {
                            if !store.workdirFiles.isEmpty {
                                filesSectionHeader(
                                    title: FilesPanelStrings.workdirSectionTitle,
                                    subtitle: FilesPanelStrings.workdirSectionSubtitle,
                                    systemImage: "folder"
                                )
                                ArtifactBrowseWorkdirCardGroup(files: store.workdirFiles)
                            }
                            if !store.transcriptArtifacts.isEmpty {
                                filesSectionHeader(
                                    title: FilesPanelStrings.artifactsSectionTitle,
                                    subtitle: FilesPanelStrings.artifactsSectionSubtitle,
                                    systemImage: "archivebox"
                                )
                                ArtifactBrowseAttachmentCardGroup(artifacts: store.transcriptArtifacts)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, TamtriSpacing.md)
                        .padding(.bottom, TamtriSpacing.sm)
                    }
                }
            }
        }
    }

    private var browseHeader: some View {
        HStack {
            Text("Browse")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            Spacer()
            Button {
                store.revealWorkdir()
            } label: {
                Label("Open folder", systemImage: "folder")
            }
            .labelStyle(.iconOnly)
            .buttonStyle(.tamtriPlain)
            .help("Reveal workdir in Finder")
        }
        .padding(.horizontal, TamtriSpacing.md)
        .padding(.top, TamtriSpacing.sm)
        .padding(.bottom, TamtriSpacing.xs)
    }

    @ViewBuilder
    private func filesSectionHeader(title: String, subtitle: String, systemImage: String) -> some View {
        VStack(alignment: .leading, spacing: TamtriSpacing.xs) {
            Label(title, systemImage: systemImage)
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            Text(subtitle)
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(.top, TamtriSpacing.sm)
    }
}

struct FilesPreviewPanelView: View {
    @EnvironmentObject private var store: AppStore

    private var isLoading: Bool {
        store.attachmentPreview == nil && store.workdirPreview == nil
            && (store.selectedTranscriptArtifact != nil || store.selectedWorkdirFile != nil)
    }

    var body: some View {
        VStack(spacing: 0) {
            previewHeader
            previewBody
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .textBackgroundColor))
        .onExitCommand {
            store.backToFilesBrowse()
        }
    }

    private var previewHeader: some View {
        VStack(spacing: TamtriSpacing.xs) {
            HStack(spacing: TamtriSpacing.sm) {
                Button {
                    store.backToFilesBrowse()
                } label: {
                    Label("Back", systemImage: "chevron.left")
                }
                .buttonStyle(.tamtriPlain)
                .foregroundStyle(.secondary)
                .help("Back to browse")

                Text(previewFileName)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)
                    .truncationMode(.middle)

                Text(previewBadge)
                    .font(.caption2.weight(.semibold))
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(previewBadgeColor.opacity(0.15), in: Capsule())

                Spacer(minLength: 0)

                if richPreviewAvailable {
                    Picker("Display", selection: $store.filesPreviewDisplayMode) {
                        Text("Rich").tag(FilesPreviewDisplayMode.rich)
                        Text("Source").tag(FilesPreviewDisplayMode.source)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .frame(maxWidth: 140)
                }

                previewActionButtons
            }
            .padding(.horizontal, TamtriSpacing.md)
            .padding(.top, TamtriSpacing.sm)
            .padding(.bottom, TamtriSpacing.xs)
        }
    }

    @ViewBuilder
    private var previewActionButtons: some View {
        if let artifact = store.selectedTranscriptArtifact {
            Button("Open") {
                store.openAttachment(
                    conversationId: store.selectedConversation?.id ?? "",
                    path: artifact.path,
                    size: artifact.size,
                    sha256: artifact.sha256
                )
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            Button {
                store.revealTranscriptArtifact(artifact)
            } label: {
                Label("Reveal in Finder", systemImage: "folder")
            }
            .labelStyle(.iconOnly)
            .help("Reveal in Finder")
            .buttonStyle(.bordered)
            .controlSize(.small)
        } else if let file = store.selectedWorkdirFile {
            Button("Open") {
                store.openWorkdirFile(file)
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            Button {
                store.revealWorkdirFile(file)
            } label: {
                Label("Reveal in Finder", systemImage: "folder")
            }
            .labelStyle(.iconOnly)
            .help("Reveal in Finder")
            .buttonStyle(.bordered)
            .controlSize(.small)
        }
    }

    @ViewBuilder
    private var previewBody: some View {
        if isLoading {
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let preview = store.attachmentPreview {
            FilePreviewContentView(
                path: preview.path,
                mimeType: preview.mimeType,
                text: preview.text,
                imageData: preview.imageData,
                error: preview.error,
                displayMode: effectiveDisplayMode(for: preview),
                useArtifactTypography: true
            )
        } else if let preview = store.workdirPreview {
            FilePreviewContentView(
                path: preview.relativePath,
                mimeType: preview.mimeType,
                text: preview.text,
                imageData: preview.imageData,
                error: preview.error,
                displayMode: effectiveDisplayMode(for: preview),
                useArtifactTypography: false
            )
        } else {
            Text("Select a file in browse to preview it here.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private var previewFileName: String {
        if let artifact = store.selectedTranscriptArtifact {
            return (artifact.path as NSString).lastPathComponent
        }
        if let file = store.selectedWorkdirFile {
            return file.relativePath
        }
        return "Preview"
    }

    private var previewBadge: String {
        store.selectedTranscriptArtifact != nil
            ? FilesPanelStrings.attachmentBadge
            : FilesPanelStrings.liveWorkingFileBadge
    }

    private var previewBadgeColor: Color {
        store.selectedTranscriptArtifact != nil ? .secondary : .orange
    }

    private var richPreviewAvailable: Bool {
        if let preview = store.attachmentPreview {
            return FilePreviewSupport.richPreviewAvailable(
                mimeType: preview.mimeType,
                text: preview.text,
                imageData: preview.imageData,
                error: preview.error
            )
        }
        if let preview = store.workdirPreview {
            return FilePreviewSupport.richPreviewAvailable(
                mimeType: preview.mimeType,
                text: preview.text,
                imageData: preview.imageData,
                error: preview.error
            )
        }
        return false
    }

    private func effectiveDisplayMode(for preview: AttachmentFilePreview) -> FilesPreviewDisplayMode {
        let richAvailable = FilePreviewSupport.richPreviewAvailable(
            mimeType: preview.mimeType,
            text: preview.text,
            imageData: preview.imageData,
            error: preview.error
        )
        return richAvailable ? store.filesPreviewDisplayMode : .source
    }

    private func effectiveDisplayMode(for preview: WorkdirFilePreview) -> FilesPreviewDisplayMode {
        let richAvailable = FilePreviewSupport.richPreviewAvailable(
            mimeType: preview.mimeType,
            text: preview.text,
            imageData: preview.imageData,
            error: preview.error
        )
        return richAvailable ? store.filesPreviewDisplayMode : .source
    }
}

struct FilePreviewContentView: View {
    @EnvironmentObject private var store: AppStore
    let path: String
    let mimeType: String?
    let text: String?
    let imageData: Data?
    let error: String?
    let displayMode: FilesPreviewDisplayMode
    var useArtifactTypography = false

    var body: some View {
        Group {
            if let error {
                ScrollView {
                    Text(error)
                        .foregroundStyle(.secondary)
                        .padding()
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else if displayMode == .rich {
                richPreviewBody
            } else {
                sourcePreviewBody
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder
    private var richPreviewBody: some View {
        if let text, artifactUsesWebViewPreview(mimeType: mimeType) {
            SandboxedHTMLView(html: text, onBlockedNavigation: { url in
                store.logBlockedNavigation(url: url.absoluteString)
            })
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(.horizontal, 4)
            .padding(.bottom, 8)
        } else if let text {
            ScrollView {
                richTextPreview(text)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal)
                    .padding(.bottom, 12)
            }
        } else if let imageData, let image = NSImage(data: imageData) {
            ScrollView {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
                    .frame(maxWidth: .infinity)
                    .padding(.horizontal)
                    .padding(.bottom, 12)
            }
        } else {
            unavailablePreviewMessage
        }
    }

    @ViewBuilder
    private var sourcePreviewBody: some View {
        if let text {
            ScrollView {
                Text(text)
                    .font(useArtifactTypography ? TamtriTheme.artifactProseFont().monospaced() : .body.monospaced())
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal)
                    .padding(.bottom, 12)
            }
        } else if let imageData {
            ScrollView {
                VStack(alignment: .leading, spacing: TamtriSpacing.sm) {
                    Text("Binary image (\(ByteCountFormatter.string(fromByteCount: Int64(imageData.count), countStyle: .file)))")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(imageData.prefix(256).map { String(format: "%02x", $0) }.joined(separator: " "))
                        .font(.caption.monospaced())
                        .textSelection(.enabled)
                        .foregroundStyle(.secondary)
                }
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        } else {
            unavailablePreviewMessage
        }
    }

    private var unavailablePreviewMessage: some View {
        ScrollView {
            Text("No in-app preview for this file type. Use Open or Reveal in Finder.")
                .foregroundStyle(.secondary)
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder
    private func richTextPreview(_ text: String) -> some View {
        switch mimeType {
        case "text/csv", "text/tab-separated-values":
            CSVPreview(text: text, separator: mimeType == "text/tab-separated-values" ? "\t" : ",")
        case "text/markdown":
            if useArtifactTypography {
                MarkdownPreview(content: text)
            } else {
                TranscriptMarkdownText(content: text, muted: false)
            }
        default:
            Text(text)
                .font(useArtifactTypography ? TamtriTheme.artifactProseFont() : .body.monospaced())
                .textSelection(.enabled)
        }
    }
}
