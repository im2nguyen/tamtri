import SwiftUI
import AppKit

struct VaultIssueBadge: View {
    let issue: VaultIssueRecord

    var body: some View {
        Image(systemName: iconName)
            .font(.caption2)
            .foregroundStyle(tint)
            .help(issue.detail)
            .accessibilityLabel(issue.detail)
    }

    private var iconName: String {
        switch issue.kind {
        case "duplicate_id": "doc.on.doc.fill"
        case "torn_tail": "scissors"
        default: "exclamationmark.triangle.fill"
        }
    }

    private var tint: Color {
        switch issue.kind {
        case "duplicate_id": .orange
        case "torn_tail": .yellow
        default: .red
        }
    }
}

struct SidebarConversationRow: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.colorScheme) private var colorScheme
    let conversation: ConversationSummary

    @State private var isHovered = false

    private var issues: [VaultIssueRecord] {
        store.issuesForConversation(conversation.id)
    }

    private var compactTime: String {
        TamtriFormatting.compactRelativeTimestamp(from: conversation.updatedAt)
    }

    private var isRunningHere: Bool {
        store.selectedConversationId == conversation.id && store.isRunActive
    }

    private var isSelected: Bool {
        store.selectedConversationId == conversation.id
    }

    var body: some View {
        Button {
            store.selectConversation(conversation)
        } label: {
            HStack(spacing: TamtriSpacing.sm) {
                Text(conversation.title)
                    .font(TamtriTheme.sidebarRowFont())
                    .foregroundStyle(isSelected ? .primary : .secondary)
                    .lineLimit(1)
                Spacer(minLength: TamtriSpacing.sm)
                if isRunningHere {
                    Circle()
                        .fill(Color.accentColor)
                        .frame(width: 6, height: 6)
                        .accessibilityLabel("Running")
                }
                ForEach(issues) { issue in
                    VaultIssueBadge(issue: issue)
                }
                Text(compactTime)
                    .font(TamtriTheme.sidebarTimestampFont())
                    .foregroundStyle(.tertiary)
                    .monospacedDigit()
            }
            .sidebarRowLabel()
        }
        .buttonStyle(.tamtriPlain)
        .sidebarRowHighlight(
            isSelected: isSelected,
            isHovered: isHovered,
            colorScheme: colorScheme
        )
        .onHover { isHovered = $0 }
        .contextMenu {
            if let issue = issues.first {
                Button("Copy Issue Details") {
                    store.copyVaultIssueDetails(issue)
                }
                if let conversationId = issue.conversationId {
                    Button("Reveal in Finder") {
                        store.revealConversationFolder(conversationId: conversationId)
                    }
                } else if let path = issue.path {
                    Button("Reveal in Finder") {
                        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
                    }
                }
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(conversation.title), updated \(compactTime)")
        .accessibilityValue(issues.isEmpty ? (isRunningHere ? "Running" : "") : issues.map(\.kind).joined(separator: ", "))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

struct SidebarView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        ZStack(alignment: .topLeading) {
            VisualEffectView(material: .sidebar)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .ignoresSafeArea(.container, edges: .top)

            VStack(spacing: 0) {
                SidebarTopBar()

                SidebarNavSection()

                Group {
                    if store.conversations.isEmpty {
                        EmptyStateView(
                            systemImage: "bubble.left.and.bubble.right",
                            title: "No chats yet",
                            message: "Start a new chat to turn data into a report or explore your vault.",
                            primaryActionTitle: nil,
                            onPrimary: nil
                        )
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .padding(.horizontal, TamtriLayout.sidebarContentInset)
                    } else {
                        SidebarChatsList(conversations: store.conversations)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)

                SidebarFooter()
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .ignoresSafeArea(.container, edges: .top)
    }
}

private struct SidebarChatsList: View {
    let conversations: [ConversationSummary]

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                Text("Chats")
                    .font(TamtriTheme.sidebarSectionFont())
                    .foregroundStyle(.tertiary)
                    .padding(.leading, TamtriLayout.sidebarSectionHeaderInset)
                    .padding(.trailing, TamtriLayout.sidebarSectionHeaderInset)
                    .padding(.top, TamtriSpacing.md)
                    .padding(.bottom, TamtriLayout.sidebarSectionHeaderBottomSpacing)

                ForEach(conversations) { conversation in
                    SidebarConversationRow(conversation: conversation)
                }
            }
            .padding(.bottom, TamtriSpacing.sm)
        }
        .scrollContentBackground(.hidden)
        .accessibilityIdentifier(KeyboardHeroFlowIdentifiers.sidebarIdentifier)
    }
}

/// Sidebar half of the window-spanning top bar: room for traffic lights, then the collapse toggle.
struct SidebarTopBar: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        HStack(spacing: TamtriSpacing.sm) {
            Color.clear
                .frame(width: TamtriLayout.trafficLightInset)
                .accessibilityHidden(true)
            Button {
                store.toggleSidebar()
            } label: {
                Image(systemName: "sidebar.leading")
            }
            .buttonStyle(.tamtriPlain)
            .foregroundStyle(.secondary)
            .help("Collapse sidebar")
            TitlebarBand()
        }
        .padding(.trailing, TamtriSpacing.sm)
        .frame(height: TamtriLayout.topBarHeight, alignment: .center)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(VisualEffectView(material: .sidebar))
        .zIndex(1)
    }
}

struct SidebarFooter: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.colorScheme) private var colorScheme

    @State private var isHovered = false

    var body: some View {
        Button {
            store.showSettings = true
            Task { await store.refreshGatewayServers() }
        } label: {
            HStack(spacing: TamtriSpacing.sm) {
                Image(systemName: "gearshape")
                    .font(.body)
                    .frame(width: TamtriLayout.sidebarIconWidth, alignment: .center)
                Text("Settings")
                    .font(TamtriTheme.sidebarNavFont())
            }
            .sidebarRowLabel()
        }
        .buttonStyle(.tamtriPlain)
        .help("Settings")
        .foregroundStyle(.secondary)
        .sidebarRowHighlight(isSelected: false, isHovered: isHovered, colorScheme: colorScheme)
        .onHover { isHovered = $0 }
        .padding(.top, TamtriSpacing.sm)
        .padding(.bottom, TamtriSpacing.lg)
        .accessibilityLabel("Settings")
    }
}

struct SidebarNavSection: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        VStack(spacing: 0) {
            SidebarNavRow(title: "New chat", systemImage: "square.and.pencil", shortcut: "⌘N") {
                store.showNewConversation = true
            }
            SidebarNavRow(title: "Search", systemImage: "magnifyingglass", shortcut: "⌘F") {
                store.showSearch = true
            }
            SidebarNavRow(title: "Harness health", systemImage: "heart.text.square") {
                store.showHarnessHealth = true
            }
        }
        .padding(.top, TamtriSpacing.xs)
        .padding(.bottom, TamtriSpacing.xs)
    }
}

struct SidebarNavRow: View {
    @Environment(\.colorScheme) private var colorScheme
    let title: String
    let systemImage: String
    var shortcut: String?
    let action: () -> Void

    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: TamtriSpacing.sm) {
                Image(systemName: systemImage)
                    .font(.body)
                    .frame(width: TamtriLayout.sidebarIconWidth, alignment: .center)
                Text(title)
                    .font(TamtriTheme.sidebarNavFont())
                Spacer(minLength: 0)
                if let shortcut {
                    Text(shortcut)
                        .font(TamtriTheme.sidebarTimestampFont())
                        .foregroundStyle(.tertiary)
                }
            }
            .sidebarRowLabel()
        }
        .buttonStyle(.tamtriPlain)
        .foregroundStyle(.secondary)
        .sidebarRowHighlight(isSelected: false, isHovered: isHovered, colorScheme: colorScheme)
        .onHover { isHovered = $0 }
        .accessibilityLabel(title)
    }
}

private extension View {
    func sidebarRowLabel() -> some View {
        padding(.horizontal, TamtriLayout.sidebarRowInnerInset)
            .padding(.vertical, TamtriLayout.sidebarRowVerticalPadding)
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
    }

    func sidebarRowHighlight(isSelected: Bool, isHovered: Bool, colorScheme: ColorScheme) -> some View {
        background {
            RoundedRectangle(cornerRadius: 6)
                .fill(
                    isSelected
                        ? TamtriTheme.sidebarSelection(colorScheme)
                        : isHovered
                            ? TamtriTheme.sidebarHover(colorScheme)
                            : Color.clear
                )
                .padding(.horizontal, TamtriLayout.sidebarRowHighlightInset)
        }
    }
}
