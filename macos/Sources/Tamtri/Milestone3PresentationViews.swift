import SwiftUI

struct ThinkingDisclosureView: View {
    let presentation: ThinkingDisclosurePresentation

    var body: some View {
        DisclosureGroup(presentation.title, isExpanded: .constant(presentation.initiallyExpanded)) {
            Text(presentation.text)
                .textSelection(.enabled)
        }
        .padding(.vertical, 4)
    }
}

struct ToolCardView: View {
    let presentation: ToolCardPresentation

    var body: some View {
        CompactCard(title: presentation.title, systemImage: "wrench.and.screwdriver") {
            if !presentation.subtitle.isEmpty {
                Text(presentation.subtitle)
                    .foregroundStyle(.secondary)
            }
            if let diff = presentation.diff {
                FileDiffView(diff: diff)
            } else if !presentation.detail.isEmpty {
                Text(presentation.detail)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            }
        }
        .accessibilityLabel(presentation.accessibilityLabel)
    }
}

struct PermissionCardView: View {
    let presentation: PermissionCardPresentation
    var onSelectOption: ((String) -> Void)?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label("Permission requested", systemImage: "hand.raised")
                .font(.headline)
            if let harnessName = presentation.harnessDisplayName, !harnessName.isEmpty {
                Text("From \(harnessName)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            if let command = presentation.command, !command.isEmpty {
                Text(command)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            } else if let diff = presentation.diff {
                FileDiffView(diff: diff)
            } else if !presentation.summary.isEmpty {
                Text(presentation.summary)
                    .textSelection(.enabled)
            }
            if let onSelectOption {
                HStack {
                    ForEach(presentation.rejectOptions) { option in
                        Button(option.label, role: .destructive) {
                            onSelectOption(option.id)
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    ForEach(presentation.allowOptions) { option in
                        Button(option.label) {
                            onSelectOption(option.id)
                        }
                        .buttonStyle(.bordered)
                    }
                    .keyboardShortcut(.defaultAction)
                }
            }
        }
        .padding(10)
        .background(.yellow.opacity(0.18), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityLabel(presentation.accessibilityLabel)
    }
}

struct FileDiffView: View {
    let diff: DiffPayload

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let path = diff.path, !path.isEmpty {
                Text(path)
                    .font(.subheadline.bold())
            }
            if let change = diff.change, !change.isEmpty {
                Text(change.capitalized)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if let oldText = diff.oldText, !oldText.isEmpty {
                Text("Before")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(oldText)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            }
            if let newText = diff.newText, !newText.isEmpty {
                Text("After")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(newText)
                    .font(.body.monospaced())
                    .textSelection(.enabled)
            }
        }
    }
}
