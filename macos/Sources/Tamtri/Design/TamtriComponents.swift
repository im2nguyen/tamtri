import AppKit
import SwiftUI

struct StatusChip: View {
    let label: String
    var tone: TamtriTheme.SemanticTone = .info
    var prominent: Bool = false

    var body: some View {
        Text(label)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, TamtriSpacing.sm)
            .padding(.vertical, 3)
            .background(background, in: Capsule())
            .foregroundStyle(foreground)
            .accessibilityLabel(label)
    }

    @Environment(\.colorScheme) private var colorScheme

    private var background: Color {
        if prominent {
            return Color.accentColor.opacity(colorScheme == .dark ? 0.28 : 0.18)
        }
        return TamtriTheme.semanticBackground(tone, scheme: colorScheme)
    }

    private var foreground: Color {
        if prominent {
            return Color.accentColor
        }
        switch tone {
        case .consent: return .yellow
        case .thinking: return .purple
        case .tool: return .blue
        case .artifact: return .teal
        case .elicitation: return .teal
        case .error: return .red
        case .info: return .secondary
        }
    }
}

struct TamtriCard<Content: View>: View {
    let title: String
    let systemImage: String
    var tone: TamtriTheme.SemanticTone = .tool
    var accentBar: Bool = false
    @ViewBuilder let content: Content

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            if accentBar {
                RoundedRectangle(cornerRadius: 1.5)
                    .fill(Color.yellow.opacity(colorScheme == .dark ? 0.75 : 0.85))
                    .frame(width: 3)
                    .padding(.vertical, TamtriSpacing.xs)
            }
            VStack(alignment: .leading, spacing: TamtriSpacing.sm) {
                Label(title, systemImage: systemImage)
                    .font(.subheadline.weight(.semibold))
                content
            }
            .padding(TamtriSpacing.md)
        }
        .background(background, in: RoundedRectangle(cornerRadius: TamtriRadius.card))
        .overlay(
            RoundedRectangle(cornerRadius: TamtriRadius.card)
                .strokeBorder(TamtriTheme.hairline(colorScheme), lineWidth: 0.5)
        )
    }

    @Environment(\.colorScheme) private var colorScheme

    private var background: Color {
        if accentBar {
            return TamtriTheme.cardBackground(colorScheme)
        }
        return TamtriTheme.semanticBackground(tone, scheme: colorScheme)
    }
}

struct MetadataRow: View {
    let harnessLabel: String?
    let modelLabel: String?
    var isRunning: Bool = false

    var body: some View {
        HStack(spacing: TamtriSpacing.sm) {
            if let harnessLabel {
                StatusChip(label: harnessLabel, tone: .info)
            }
            if let modelLabel {
                StatusChip(label: modelLabel, tone: .info)
            }
            if isRunning {
                StatusChip(label: "Running…", tone: .tool, prominent: true)
            }
        }
    }
}

struct EmptyStateView: View {
    let systemImage: String
    let title: String
    let message: String
    var primaryActionTitle: String?
    var onPrimary: (() -> Void)?

    var body: some View {
        VStack(spacing: TamtriSpacing.lg) {
            Image(systemName: systemImage)
                .font(.system(size: 36))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)

            VStack(spacing: TamtriSpacing.sm) {
                Text(title)
                    .font(.title3.weight(.semibold))
                    .multilineTextAlignment(.center)
                Text(message)
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }

            if let primaryActionTitle, let onPrimary {
                Button(primaryActionTitle, action: onPrimary)
                    .buttonStyle(.borderedProminent)
            }
        }
        .padding(TamtriSpacing.xl)
        .frame(maxWidth: 420)
        .accessibilityElement(children: .contain)
    }
}

struct HarnessDisplayNames {
    static func harness(_ id: String?, agents: [HarnessAgentRecord]) -> String? {
        guard let id else { return nil }
        return agents.first(where: { $0.id == id })?.displayName ?? id.replacingOccurrences(of: "-acp", with: "").capitalized
    }

    static func model(_ id: String?) -> String? {
        guard let id, !id.isEmpty else { return nil }
        if id == "default" { return "Default" }
        return id.replacingOccurrences(of: "_", with: " ").capitalized
    }
}

extension View {
    /// Readable centered column for transcript and composer.
    func conversationContentColumn(alignment: Alignment = .center) -> some View {
        frame(maxWidth: TamtriLayout.contentMaxWidth, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: alignment)
            .padding(.horizontal, TamtriLayout.composerInset)
    }

    /// Pointer hand on hover for custom tap targets (macOS 14+).
    func tamtriClickableCursor(enabled: Bool = true) -> some View {
        modifier(TamtriClickableCursorModifier(enabled: enabled))
    }
}

private struct TamtriClickableCursorModifier: ViewModifier {
    var enabled: Bool
    @Environment(\.isEnabled) private var isEnabled
    @State private var cursorPushed = false

    func body(content: Content) -> some View {
        content
            .onHover { hovering in
                if hovering {
                    guard enabled, isEnabled, !cursorPushed else { return }
                    NSCursor.pointingHand.push()
                    cursorPushed = true
                } else {
                    clearCursor()
                }
            }
            .onDisappear {
                clearCursor()
            }
    }

    private func clearCursor() {
        guard cursorPushed else { return }
        NSCursor.pop()
        cursorPushed = false
    }
}

/// Plain button style with pointing-hand cursor for custom shell controls.
struct TamtriPlainButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .tamtriClickableCursor()
    }
}

extension ButtonStyle where Self == TamtriPlainButtonStyle {
    static var tamtriPlain: TamtriPlainButtonStyle { TamtriPlainButtonStyle() }
}

/// Drag handle between resizable workspace panels.
struct PanelResizeDivider: View {
    static let thickness: CGFloat = 1

    enum Edge {
        /// Divider sits on the panel's leading edge (e.g. files rail).
        case panelLeading
        /// Divider sits on the panel's trailing edge (e.g. left sidebar).
        case panelTrailing
    }

    @Binding var width: CGFloat
    let minWidth: CGFloat
    let maxWidth: CGFloat
    var edge: Edge = .panelLeading
    var accessibilityLabel: String = "Resize panel"

    @State private var dragStartWidth: CGFloat?

    var body: some View {
        Rectangle()
            .fill(Color.primary.opacity(0.12))
            .frame(width: Self.thickness)
            .overlay {
                Rectangle()
                    .fill(Color.clear)
                    .frame(width: 6)
                    .contentShape(Rectangle())
                    .onHover { hovering in
                        if hovering {
                            NSCursor.resizeLeftRight.push()
                        } else {
                            NSCursor.pop()
                        }
                    }
                    .gesture(
                        DragGesture(minimumDistance: 1)
                            .onChanged { value in
                                if dragStartWidth == nil {
                                    dragStartWidth = width
                                }
                                let start = dragStartWidth ?? width
                                let delta = value.translation.width
                                let next = switch edge {
                                case .panelLeading:
                                    start - delta
                                case .panelTrailing:
                                    start + delta
                                }
                                width = min(maxWidth, max(minWidth, next))
                            }
                            .onEnded { _ in
                                dragStartWidth = nil
                            }
                    )
            }
            .accessibilityLabel(accessibilityLabel)
    }
}
