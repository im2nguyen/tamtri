import SwiftUI

struct CommandPaletteAction: Identifiable, Equatable {
    let id: String
    let title: String
    let subtitle: String
    let systemImage: String
    let shortcut: String?
    let perform: () -> Void

    static func == (lhs: CommandPaletteAction, rhs: CommandPaletteAction) -> Bool {
        lhs.id == rhs.id
    }
}

struct CommandPaletteView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @FocusState private var queryFocused: Bool

    private var actions: [CommandPaletteAction] {
        [
            CommandPaletteAction(
                id: "new",
                title: "New Conversation",
                subtitle: "Start a fresh thread",
                systemImage: "plus.bubble",
                shortcut: "⌘N"
            ) {
                dismiss()
                store.showNewConversation = true
            },
            CommandPaletteAction(
                id: "fork",
                title: "Fork Into…",
                subtitle: "Branch with another harness or model",
                systemImage: "arrow.triangle.branch",
                shortcut: nil
            ) {
                dismiss()
                store.showForkConversation = true
            },
            CommandPaletteAction(
                id: "search",
                title: "Search Conversations",
                subtitle: "Titles, text, and thinking only",
                systemImage: "magnifyingglass",
                shortcut: "⌘F"
            ) {
                dismiss()
                store.showSearch = true
            },
            CommandPaletteAction(
                id: "settings",
                title: "Settings",
                subtitle: "Gateway servers and preferences",
                systemImage: "gearshape",
                shortcut: nil
            ) {
                dismiss()
                store.showSettings = true
            },
            CommandPaletteAction(
                id: "harness-health",
                title: "Harness Health",
                subtitle: "Installed agents and setup checklist",
                systemImage: "heart.text.square",
                shortcut: nil
            ) {
                dismiss()
                store.showHarnessHealth = true
            },
            CommandPaletteAction(
                id: "reveal-vault",
                title: "Reveal Vault in Finder",
                subtitle: store.vaultPath,
                systemImage: "folder",
                shortcut: nil
            ) {
                dismiss()
                store.revealVaultInFinder()
            },
            CommandPaletteAction(
                id: "diagnostics",
                title: "Report an Issue…",
                subtitle: "Build a diagnostics bundle to review before sharing",
                systemImage: "ladybug",
                shortcut: nil
            ) {
                dismiss()
                store.showDiagnostics = true
            },
        ]
    }

    private var filteredActions: [CommandPaletteAction] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return actions }
        return actions.filter {
            $0.title.localizedCaseInsensitiveContains(trimmed)
                || $0.subtitle.localizedCaseInsensitiveContains(trimmed)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            TextField("Search actions", text: $query)
                .textFieldStyle(.roundedBorder)
                .focused($queryFocused)
                .accessibilityLabel("Command palette search")
            List(filteredActions) { action in
                Button {
                    action.perform()
                } label: {
                    HStack {
                        Label(action.title, systemImage: action.systemImage)
                        Spacer()
                        if let shortcut = action.shortcut {
                            Text(shortcut)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
                .buttonStyle(.tamtriPlain)
                .accessibilityLabel(action.title)
                .accessibilityHint(action.subtitle)
            }
            .listStyle(.plain)
            .frame(minHeight: 220)
        }
        .padding()
        .frame(width: 460)
        .onAppear { queryFocused = true }
    }
}
