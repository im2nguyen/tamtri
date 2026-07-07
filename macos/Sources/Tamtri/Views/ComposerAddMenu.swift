import SwiftUI

struct ComposerAddMenu: View {
    @EnvironmentObject private var store: AppStore
    @State private var isPresented = false

    var body: some View {
        Button {
            isPresented.toggle()
        } label: {
            Image(systemName: "plus.circle")
                .frame(width: 20, height: 20)
        }
        .buttonStyle(.tamtriPlain)
        .help("Add files, folders, and more")
        .disabled(store.selectedConversation == nil)
        .popover(isPresented: $isPresented, arrowEdge: .top) {
            VStack(alignment: .leading, spacing: TamtriSpacing.xs) {
                TranscriptSectionHeader(title: "Add to conversation")
                menuRow(
                    title: "Add files…",
                    detail: "Copy files into this conversation's workdir",
                    systemImage: "doc.badge.plus"
                ) {
                    isPresented = false
                    store.presentAddFilesPanel()
                }
                menuRow(
                    title: "Add folder…",
                    detail: "Attach a folder as a conversation root",
                    systemImage: "folder.badge.plus"
                ) {
                    isPresented = false
                    store.presentAddFolderAsRootPanel()
                }

                TranscriptSectionHeader(title: "Manage")
                menuRow(
                    title: "Conversation roots…",
                    detail: "View and manage attached folder roots",
                    systemImage: "folder"
                ) {
                    isPresented = false
                    store.presentConversationRoots()
                }
                disabledMenuRow(
                    title: "MCP servers",
                    detail: "Coming soon",
                    systemImage: "server.rack"
                )
                disabledMenuRow(
                    title: "Skills",
                    detail: "Coming soon",
                    systemImage: "sparkles"
                )
            }
            .padding(TamtriSpacing.sm)
            .frame(minWidth: 280)
        }
    }

    private func menuRow(
        title: String,
        detail: String,
        systemImage: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            menuRowContent(title: title, detail: detail, systemImage: systemImage)
        }
        .buttonStyle(.tamtriPlain)
        .padding(.horizontal, TamtriSpacing.sm)
        .padding(.vertical, TamtriSpacing.xs)
    }

    private func disabledMenuRow(
        title: String,
        detail: String,
        systemImage: String
    ) -> some View {
        menuRowContent(title: title, detail: detail, systemImage: systemImage, isEnabled: false)
            .padding(.horizontal, TamtriSpacing.sm)
            .padding(.vertical, TamtriSpacing.xs)
            .allowsHitTesting(false)
    }

    private func menuRowContent(
        title: String,
        detail: String,
        systemImage: String,
        isEnabled: Bool = true
    ) -> some View {
        HStack(alignment: .top, spacing: TamtriSpacing.sm) {
            Image(systemName: systemImage)
                .font(.body)
                .frame(width: 20, alignment: .center)
                .padding(.top, 1)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(TamtriTheme.sidebarNavFont())
                    .foregroundStyle(isEnabled ? .primary : .tertiary)
                Text(detail)
                    .font(TamtriTheme.sidebarTimestampFont())
                    .foregroundStyle(isEnabled ? .secondary : .tertiary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }
}
