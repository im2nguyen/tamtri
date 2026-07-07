import SwiftUI
import UniformTypeIdentifiers
import AppKit

struct ComposerView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.colorScheme) private var colorScheme
    @State private var isDropTargeted = false

    var body: some View {
        VStack(alignment: .leading, spacing: TamtriSpacing.sm) {
                if !store.composerAttachedFiles.isEmpty {
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: TamtriSpacing.sm) {
                            ForEach(store.composerAttachedFiles, id: \.self) { name in
                                StatusChip(label: name, tone: .artifact)
                            }
                        }
                    }
                }
                TextField(composerPlaceholder, text: $store.composerText, axis: .vertical)
                    .textFieldStyle(.plain)
                    .lineLimit(1...6)
                    .accessibilityIdentifier(KeyboardHeroFlowIdentifiers.composerIdentifier)
                    .onSubmit {
                        if !store.composerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                            store.send()
                        }
                    }
                    .onDrop(of: [.fileURL], isTargeted: nil, perform: handleFileDrop)
                HStack(spacing: TamtriSpacing.sm) {
                    ComposerAddMenu()
                    Spacer()
                    composerIdentityChips
                    composerSendButton
                }
        }
        .padding(TamtriSpacing.md)
            .background(
                RoundedRectangle(cornerRadius: TamtriRadius.bar)
                    .fill(TamtriTheme.surfaceComposer(colorScheme))
                    .shadow(color: TamtriTheme.composerShadow(colorScheme), radius: 12, y: 4)
            )
            .overlay(
                RoundedRectangle(cornerRadius: TamtriRadius.bar)
                    .strokeBorder(
                        isDropTargeted ? Color.accentColor.opacity(0.5) : TamtriTheme.hairline(colorScheme),
                        lineWidth: isDropTargeted ? 1.5 : 0.5
                    )
            )
        .conversationContentColumn()
        .padding(.vertical, TamtriSpacing.md)
        .onDrop(of: [.fileURL], isTargeted: $isDropTargeted, perform: handleFileDrop)
    }

    @ViewBuilder
    private var composerIdentityChips: some View {
        if let conversation = store.displayedConversation {
            HStack(spacing: TamtriSpacing.xs) {
                StatusChip(
                    label: HarnessDisplayNames.harness(conversation.harnessId, agents: store.harnessAgents)
                        ?? conversation.harnessId
                        ?? "Harness",
                    tone: .info
                )
                StatusChip(
                    label: HarnessDisplayNames.model(conversation.modelId) ?? conversation.modelId ?? "Model",
                    tone: .info
                )
            }
        }
    }

    @ViewBuilder
    private var composerSendButton: some View {
        if store.isRunActive {
            Button {
                store.cancelRun()
            } label: {
                Image(systemName: "stop.fill")
                    .font(.body.weight(.semibold))
                    .foregroundStyle(.white)
                    .frame(width: 32, height: 32)
                    .background(Circle().fill(Color.red.opacity(0.85)))
            }
            .buttonStyle(.tamtriPlain)
            .keyboardShortcut(.escape)
            .help("Cancel run")
        } else {
            Button {
                store.send()
            } label: {
                Image(systemName: "arrow.up")
                    .font(.body.weight(.bold))
                    .foregroundStyle(.white)
                    .frame(width: 32, height: 32)
                    .background(Circle().fill(Color.accentColor))
            }
            .buttonStyle(.tamtriPlain)
            .keyboardShortcut(.return, modifiers: [.command])
            .disabled(store.selectedConversation == nil)
            .help("Send message")
        }
    }

    private var composerPlaceholder: String {
        if store.workdirFiles.contains(where: { $0.relativePath.lowercased().hasSuffix(".csv") }) {
            return "Ask your harness to turn this CSV into a report…"
        }
        return "Message"
    }

    private func handleFileDrop(_ providers: [NSItemProvider]) -> Bool {
        let group = DispatchGroup()
        let collector = DropFileCollector()
        for provider in providers {
            group.enter()
            provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, _ in
                defer { group.leave() }
                let path: String?
                if let data = item as? Data,
                   let url = URL(dataRepresentation: data, relativeTo: nil),
                   url.isFileURL {
                    path = url.path
                } else if let url = item as? URL, url.isFileURL {
                    path = url.path
                } else {
                    path = nil
                }
                if let path {
                    collector.append(path)
                }
            }
        }
        group.notify(queue: .main) {
            store.attachFiles(paths: collector.paths())
        }
        return true
    }
}

final class DropFileCollector: @unchecked Sendable {
    private let lock = NSLock()
    private var collected: [String] = []

    func append(_ path: String) {
        lock.lock()
        collected.append(path)
        lock.unlock()
    }

    func paths() -> [String] {
        lock.lock()
        let copy = collected
        lock.unlock()
        return copy
    }
}
