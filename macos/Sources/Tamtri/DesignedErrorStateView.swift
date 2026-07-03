import SwiftUI

struct DesignedErrorStateView: View {
    let state: DesignedErrorState
    let onPrimary: () -> Void
    let onSecondary: (() -> Void)?

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: iconName)
                .font(.system(size: 36))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)

            VStack(spacing: 8) {
                Text(state.title)
                    .font(.title3.weight(.semibold))
                    .multilineTextAlignment(.center)
                Text(state.message)
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }

            if let detail = state.detail, !detail.isEmpty {
                DisclosureGroup("Details") {
                    Text(detail)
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .font(.caption)
            }

            HStack(spacing: 12) {
                Button(state.primaryAction.label, action: onPrimary)
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                if let secondary = state.secondaryAction, let onSecondary {
                    Button(secondary.label, action: onSecondary)
                        .buttonStyle(.bordered)
                }
            }
        }
        .padding(32)
        .frame(maxWidth: 420)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(state.accessibilityLabel)
    }

    private var iconName: String {
        switch state.kind {
        case .emptyVault: "tray"
        case .malformedConversation: "exclamationmark.triangle"
        case .busyConversation: "hourglass"
        case .missingBookmark: "folder.badge.questionmark"
        case .unsupportedSchema: "arrow.up.circle"
        case .unavailableHarness: "person.crop.circle.badge.exclamationmark"
        }
    }
}

struct DesignedErrorBannerView: View {
    let state: DesignedErrorState
    let onPrimary: () -> Void
    let onDismiss: () -> Void

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.orange)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 6) {
                Text(state.title)
                    .font(.subheadline.weight(.semibold))
                Text(state.message)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                HStack(spacing: 8) {
                    Button(state.primaryAction.label, action: onPrimary)
                        .controlSize(.small)
                    Button("Dismiss", action: onDismiss)
                        .controlSize(.small)
                }
            }
            Spacer()
        }
        .padding(12)
        .background(.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(state.accessibilityLabel)
    }
}

struct FileActionButtons: View {
    let actions: FileRowActionsPresentation
    let onPreview: () -> Void
    let onOpen: () -> Void
    let onReveal: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            if let preview = actions.preview {
                Button(action: onPreview) {
                    Label(preview.label, systemImage: preview.systemImage)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
                .disabled(!preview.isEnabled)
            }
            if let open = actions.open {
                Button(action: onOpen) {
                    Label(open.label, systemImage: open.systemImage)
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .disabled(!open.isEnabled)
            }
            if let reveal = actions.reveal {
                Button(action: onReveal) {
                    Label(reveal.label, systemImage: reveal.systemImage)
                }
                .labelStyle(.iconOnly)
                .controlSize(.small)
                .help(reveal.label)
                .disabled(!reveal.isEnabled)
            }
        }
    }
}
