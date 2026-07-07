import SwiftUI

/// Codex-style main area: one top bar, then a resizable split for transcript vs files rail.
struct MainWorkspaceView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.colorScheme) private var colorScheme
    @State private var workspaceRailWidth = CGFloat(UserPreferences.workspaceRailWidth)
    @State private var workspacePreviewRailWidth = CGFloat(UserPreferences.workspacePreviewRailWidth)

    var body: some View {
        VStack(spacing: 0) {
            WorkspaceTopBar()
            Divider()
            splitContent
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(TamtriTheme.surfaceBase(colorScheme))
        .ignoresSafeArea(.container, edges: .top)
        .onAppear {
            workspaceRailWidth = CGFloat(UserPreferences.workspaceRailWidth)
            workspacePreviewRailWidth = CGFloat(UserPreferences.workspacePreviewRailWidth)
            if !UserPreferences.workspaceRailCollapsed {
                store.openWorkspaceRail(mode: .browse, animated: false)
            }
        }
        .onChange(of: store.workspaceRailMode) { _, mode in
            switch mode {
            case .closed:
                break
            case .browse:
                workspaceRailWidth = CGFloat(UserPreferences.workspaceRailWidth)
                Task { await store.refreshWorkdirFiles(force: true) }
            case .preview:
                workspacePreviewRailWidth = CGFloat(UserPreferences.workspacePreviewRailWidth)
                Task { await store.refreshWorkdirFiles(force: true) }
            }
        }
        .onChange(of: workspaceRailWidth) { _, width in
            guard store.workspaceRailMode == .browse else { return }
            UserPreferences.workspaceRailWidth = Double(width)
        }
        .onChange(of: workspacePreviewRailWidth) { _, width in
            guard store.workspaceRailMode == .preview else { return }
            UserPreferences.workspacePreviewRailWidth = Double(width)
        }
    }

    private var activeRailWidth: CGFloat {
        switch store.workspaceRailMode {
        case .closed:
            return 0
        case .browse:
            return workspaceRailWidth
        case .preview:
            return workspacePreviewRailWidth
        }
    }

    private var railResizeMinWidth: CGFloat {
        store.workspaceRailMode == .preview
            ? TamtriLayout.filesPreviewRailMinWidth
            : TamtriLayout.railMinWidth
    }

    private var railResizeMaxWidth: CGFloat {
        store.workspaceRailMode == .preview
            ? TamtriLayout.filesPreviewRailMaxWidth
            : TamtriLayout.railMaxWidth
    }

    private var railWidthBinding: Binding<CGFloat> {
        Binding(
            get: {
                store.workspaceRailMode == .preview ? workspacePreviewRailWidth : workspaceRailWidth
            },
            set: { newValue in
                if store.workspaceRailMode == .preview {
                    workspacePreviewRailWidth = newValue
                } else {
                    workspaceRailWidth = newValue
                }
            }
        )
    }

    @ViewBuilder
    private var splitContent: some View {
        HStack(spacing: 0) {
            conversationColumn
                .frame(minWidth: 400, maxWidth: .infinity, maxHeight: .infinity)

            HStack(spacing: 0) {
                PanelResizeDivider(
                    width: railWidthBinding,
                    minWidth: railResizeMinWidth,
                    maxWidth: railResizeMaxWidth,
                    edge: .panelLeading,
                    accessibilityLabel: "Resize files panel"
                )
                WorkspaceRailView()
                    .frame(width: activeRailWidth)
            }
            .frame(width: store.showFilesPanel ? activeRailWidth + PanelResizeDivider.thickness : 0)
            .clipped()
            .allowsHitTesting(store.showFilesPanel)
            .accessibilityHidden(!store.showFilesPanel)
            .animation(TamtriLayout.panelSlideAnimation, value: store.workspaceRailMode)
            .animation(TamtriLayout.panelSlideAnimation, value: activeRailWidth)
        }
        .ignoresSafeArea(.container, edges: .top)
    }

    private var conversationColumn: some View {
        VStack(spacing: 0) {
            TranscriptView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            ComposerView()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .ignoresSafeArea(.container, edges: .top)
    }
}

/// Window-spanning top bar for the main workspace (covers transcript + files rail when open).
struct WorkspaceTopBar: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        ZStack(alignment: .trailing) {
            HStack(spacing: TamtriSpacing.sm) {
                if store.sidebarCollapsed {
                    Color.clear.frame(width: TamtriLayout.trafficLightInset)
                    Button {
                        store.toggleSidebar()
                    } label: {
                        Image(systemName: "sidebar.leading")
                    }
                    .buttonStyle(.tamtriPlain)
                    .foregroundStyle(.secondary)
                    .help("Show sidebar")
                } else {
                    Spacer().frame(width: TamtriSpacing.xs)
                }

                title

                Spacer(minLength: TamtriSpacing.sm)

                conversationActions
            }
            .padding(.leading, TamtriSpacing.md)
            .padding(.trailing, TamtriSpacing.md + 28)

            WorkspacePanelToggleButton()
                .padding(.trailing, TamtriSpacing.md)
        }
        .frame(height: TamtriLayout.topBarHeight, alignment: .center)
        .frame(maxWidth: .infinity)
        .background(TamtriTheme.surfaceBase(colorScheme))
    }

    @ViewBuilder
    private var title: some View {
        if let conversation = store.displayedConversation {
            Text(conversation.title)
                .font(.headline)
                .lineLimit(1)
                .truncationMode(.tail)
            if store.isSwitchingConversation {
                ProgressView().controlSize(.small)
            }
        } else if store.isSwitchingConversation {
            Text("Loading conversation…")
                .font(.headline)
                .foregroundStyle(.secondary)
                .lineLimit(1)
            ProgressView().controlSize(.small)
        } else if store.designedErrorState != nil {
            Text("Tamtri").font(.headline).foregroundStyle(.secondary)
        } else {
            Text("Select a chat").font(.headline).foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private var conversationActions: some View {
        HStack(spacing: TamtriSpacing.sm) {
            if store.displayedConversation != nil {
                Button {
                    store.showConversationRoots = true
                } label: {
                    Label("Roots", systemImage: "folder")
                }
                .labelStyle(.iconOnly)
                .help("Manage conversation roots")

                ConversationMoreMenu()
            }
        }
        .font(.body)
        .foregroundStyle(.secondary)
        .buttonStyle(.tamtriPlain)
    }
}

/// Fork / export / import with visible labels (macOS 26 icon-only menus hide titles).
private struct ConversationMoreMenu: View {
    @EnvironmentObject private var store: AppStore
    @State private var isPresented = false

    var body: some View {
        Button {
            isPresented.toggle()
        } label: {
            Image(systemName: "ellipsis")
                .frame(width: 20, height: 20)
        }
        .buttonStyle(.tamtriPlain)
        .help("Share, export, import, or fork")
        .popover(isPresented: $isPresented, arrowEdge: .bottom) {
            VStack(alignment: .leading, spacing: TamtriSpacing.xs) {
                menuRow(
                    title: "Fork Into…",
                    detail: "Branch with another harness or model",
                    systemImage: "arrow.triangle.branch"
                ) {
                    isPresented = false
                    store.showForkConversation = true
                }
                menuRow(
                    title: "Export .tamtri Bundle",
                    detail: "Save a portable copy of this chat",
                    systemImage: "square.and.arrow.up"
                ) {
                    isPresented = false
                    store.exportSelectedConversation()
                }
                menuRow(
                    title: "Import Conversation",
                    detail: "Open a .tamtri bundle or vault folder",
                    systemImage: "square.and.arrow.down"
                ) {
                    isPresented = false
                    store.importConversationBundle()
                }
            }
            .padding(TamtriSpacing.sm)
            .frame(minWidth: 260)
        }
    }

    private func menuRow(
        title: String,
        detail: String,
        systemImage: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(alignment: .top, spacing: TamtriSpacing.sm) {
                Image(systemName: systemImage)
                    .font(.body)
                    .frame(width: 20, alignment: .center)
                    .padding(.top, 1)
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(TamtriTheme.sidebarNavFont())
                        .foregroundStyle(.primary)
                    Text(detail)
                        .font(TamtriTheme.sidebarTimestampFont())
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.tamtriPlain)
        .padding(.horizontal, TamtriSpacing.sm)
        .padding(.vertical, TamtriSpacing.xs)
    }
}

struct WorkspacePanelToggleButton: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        Button {
            store.toggleWorkspaceRail()
        } label: {
            Image(systemName: "sidebar.trailing")
                .font(.body)
                .imageScale(.medium)
        }
        .buttonStyle(.tamtriPlain)
        .foregroundStyle(store.showFilesPanel ? Color.accentColor : .secondary)
        .padding(4)
        .background(
            store.showFilesPanel
                ? Color.accentColor.opacity(0.12)
                : Color.primary.opacity(0.06),
            in: RoundedRectangle(cornerRadius: 6)
        )
        .help(store.showFilesPanel ? "Hide files panel" : "Show files panel")
        .accessibilityLabel(store.showFilesPanel ? "Hide files panel" : "Show files panel")
        .opacity(store.selectedConversationId == nil ? 0.4 : 1)
        .allowsHitTesting(store.selectedConversationId != nil)
    }
}
