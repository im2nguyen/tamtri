import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct ConversationRootsSettingsView: View {
    @EnvironmentObject private var store: AppStore
    let conversationId: String

    @State private var roots: [RootRecord] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Roots")
                    .font(.headline)
                Spacer()
                Button("Add Folder") {
                    pickFolder()
                }
                .keyboardShortcut("n", modifiers: [.command, .shift])
            }

            if isLoading {
                ProgressView("Loading roots…")
            } else if roots.isEmpty {
                Text("Attach a filesystem folder so downstream MCP servers can read approved paths through the gateway.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(roots) { root in
                    RootRow(root: root, onRemove: { removeRoot(root) }, onRepick: { repickRoot(root) })
                }
            }

            if let errorMessage {
                Label(errorMessage, systemImage: "exclamationmark.triangle.fill")
                    .font(.callout)
                    .foregroundStyle(.orange)
            }
        }
        .padding(.vertical, 4)
        .task(id: conversationId) {
            await refresh()
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Conversation roots")
    }

    private func refresh() async {
        isLoading = true
        defer { isLoading = false }
        do {
            roots = try await store.listRoots(conversationId: conversationId)
            errorMessage = nil
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func pickFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Attach Root"
        panel.message = "Choose a folder to expose as a conversation root."
        guard panel.runModal() == .OK, let url = panel.url else { return }
        attachFolder(url: url)
    }

    private func attachFolder(url: URL) {
        Task {
            do {
                let bookmark = try url.bookmarkData(
                    options: [.withSecurityScope],
                    includingResourceValuesForKeys: nil,
                    relativeTo: nil
                )
                let name = url.lastPathComponent
                let root = try await store.attachRoot(
                    conversationId: conversationId,
                    name: name,
                    uri: url.path,
                    kind: "filesystem",
                    scope: "conversation"
                )
                try RootBookmarkStore.saveBookmark(
                    data: bookmark,
                    conversationId: conversationId,
                    rootId: root.id
                )
                await refresh()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func removeRoot(_ root: RootRecord) {
        Task {
            do {
                try await store.removeRoot(conversationId: conversationId, rootId: root.id)
                try RootBookmarkStore.deleteBookmark(conversationId: conversationId, rootId: root.id)
                await refresh()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func repickRoot(_ root: RootRecord) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Re-pick Folder"
        panel.message = "Restore access to \"\(root.name)\"."
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task {
            do {
                let bookmark = try url.bookmarkData(
                    options: [.withSecurityScope],
                    includingResourceValuesForKeys: nil,
                    relativeTo: nil
                )
                try RootBookmarkStore.saveBookmark(
                    data: bookmark,
                    conversationId: conversationId,
                    rootId: root.id
                )
                await refresh()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}

private struct RootRow: View {
    let root: RootRecord
    let onRemove: () -> Void
    let onRepick: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(root.name)
                        .font(.body.bold())
                    Text(root.uri)
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                    Text("\(root.kind) · \(root.scope)")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                if RootRowViewModel.showsRepickButton(bookmarkMissing: root.bookmarkMissing) {
                    Button("Re-pick Folder", action: onRepick)
                        .help("Restore security-scoped access to this root")
                }
                Button(role: .destructive, action: onRemove) {
                    Label("Remove", systemImage: "trash")
                }
                .labelStyle(.iconOnly)
                .help("Remove root")
            }
            if let warning = RootRowViewModel.warningMessage(bookmarkMissing: root.bookmarkMissing) {
                Label(
                    warning,
                    systemImage: "exclamationmark.triangle.fill"
                )
                .font(.callout)
                .foregroundStyle(.orange)
                .accessibilityLabel(RootRowViewModel.accessibilityLabel(rootName: root.name, bookmarkMissing: true))
            }
        }
        .padding(10)
        .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 8))
        .focusable()
        .accessibilityElement(children: .combine)
        .accessibilityLabel(RootRowViewModel.accessibilityLabel(
            rootName: root.name,
            bookmarkMissing: root.bookmarkMissing
        ))
        .accessibilityAction(named: "Remove root") {
            onRemove()
        }
        .accessibilityAction(named: "Re-pick Folder") {
            guard RootRowViewModel.showsRepickButton(bookmarkMissing: root.bookmarkMissing) else { return }
            onRepick()
        }
    }
}
