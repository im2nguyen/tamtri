import SwiftUI

enum TamtriSpacing {
    static let xs: CGFloat = 4
    static let sm: CGFloat = 8
    static let md: CGFloat = 12
    static let lg: CGFloat = 16
    static let xl: CGFloat = 24
}

enum TamtriRadius {
    static let card: CGFloat = 10
    static let bar: CGFloat = 12
    static let chip: CGFloat = 999
    static let inlineCode: CGFloat = 6
}

enum TamtriLayout {
    static let contentMaxWidth: CGFloat = 680
    static let composerInset: CGFloat = TamtriSpacing.lg
    static let railMinWidth: CGFloat = 280
    static let railIdealWidth: CGFloat = 360
    static let railMaxWidth: CGFloat = 520
    static let filesBrowseRailIdealWidth: CGFloat = 360
    static let filesPreviewRailIdealWidth: CGFloat = 520
    static let filesPreviewRailMinWidth: CGFloat = 400
    static let filesPreviewRailMaxWidth: CGFloat = 720
    /// Shared height of the window-spanning top bar (same band as the traffic lights).
    static let topBarHeight: CGFloat = WindowChrome.titlebarHeight
    static let workspaceHeaderHeight: CGFloat = WindowChrome.titlebarHeight
    /// Horizontal room reserved for the macOS traffic lights before controls begin.
    static let trafficLightInset: CGFloat = 78
    /// Left edge where sidebar row content begins (nav icons, chat titles, footer).
    static let sidebarContentInset: CGFloat = 16
    static let sidebarIconWidth: CGFloat = 18
    static let sidebarRowVerticalPadding: CGFloat = 6
    /// Margin from the sidebar edge to row highlight pills.
    static let sidebarRowHighlightInset: CGFloat = 8
    /// Padding inside a row highlight pill (text ↔ pill edge).
    static let sidebarRowInnerInset: CGFloat = 20
    /// Section headers (e.g. "Chats") align to the highlight margin, not row inner padding.
    static let sidebarSectionHeaderInset: CGFloat = 12
    static let sidebarSectionHeaderBottomSpacing: CGFloat = 8
    static let sidebarMinWidth: CGFloat = 220
    static let sidebarIdealWidth: CGFloat = 260
    static let sidebarMaxWidth: CGFloat = 360

    /// Vertical gap between content blocks inside one message.
    static let transcriptBlockSpacing: CGFloat = TamtriSpacing.md
    /// Bottom padding after an assistant turn / before the next message.
    static let transcriptTurnSpacing: CGFloat = TamtriSpacing.sm
    /// Extra line spacing for assistant prose (added to body line height).
    static let transcriptProseLineSpacing: CGFloat = 5
    /// Extra space below rich cards (permission, artifact, elicitation, app).
    static let transcriptRichBlockBottomSpacing: CGFloat = TamtriSpacing.lg
    /// Margin above and below collapsed activity clusters vs prose.
    static let transcriptActivityClusterMargin: CGFloat = TamtriSpacing.sm
    /// Gap between a user bubble and the following assistant reply.
    static let transcriptUserToAssistantGap: CGFloat = TamtriSpacing.sm
    /// Horizontal inset for inline `code` pills in transcript prose.
    static let inlineCodeHorizontalPadding: CGFloat = 5
    /// Vertical inset for inline `code` pills in transcript prose.
    static let inlineCodeVerticalPadding: CGFloat = 2

    /// Shared slide for left sidebar and right files rail.
    static let panelSlideAnimation: Animation = .easeInOut(duration: 0.22)
}

enum TamtriTheme {
    static func proseFont() -> Font {
        .body
    }

    static func uiTitleFont() -> Font {
        .title3.weight(.semibold)
    }

    static func uiMetaFont() -> Font {
        .caption
    }

    static func rowTitleFont() -> Font {
        .system(size: 13, weight: .semibold)
    }

    static func sidebarNavFont() -> Font {
        .system(size: 13, weight: .regular)
    }

    static func sidebarRowFont() -> Font {
        .system(size: 13, weight: .regular)
    }

    static func sidebarTimestampFont() -> Font {
        .system(size: 12, weight: .regular)
    }

    static func sidebarSectionFont() -> Font {
        .system(size: 11, weight: .medium)
    }

    static func metadataFont() -> Font {
        .caption
    }

    static func monoDetailFont() -> Font {
        .system(size: 12, design: .monospaced)
    }

    static func inlineCodeFont() -> Font {
        .system(size: 13, design: .monospaced)
    }

    static func inlineCodeBackground(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.white.opacity(0.10)
            : Color.black.opacity(0.06)
    }

    static func artifactProseFont() -> Font {
        .system(.body, design: .serif)
    }

    static func surfaceBase(_ scheme: ColorScheme) -> Color {
        Color(nsColor: .windowBackgroundColor)
    }

    static func surfaceRaised(_ scheme: ColorScheme) -> Color {
        cardBackground(scheme)
    }

    static func surfaceComposer(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.white.opacity(0.08)
            : Color.black.opacity(0.04)
    }

    static func cardBackground(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.white.opacity(0.06)
            : Color.black.opacity(0.04)
    }

    static func elevatedBarBackground(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color(nsColor: .windowBackgroundColor).opacity(0.85)
            : Color(nsColor: .windowBackgroundColor).opacity(0.92)
    }

    static func sidebarSelection(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.white.opacity(0.08)
            : Color.black.opacity(0.06)
    }

    static func sidebarHover(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.white.opacity(0.05)
            : Color.black.opacity(0.04)
    }

    static func userMessageBackground(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.white.opacity(0.06)
            : Color.black.opacity(0.04)
    }

    static func semanticBackground(_ tone: SemanticTone, scheme: ColorScheme) -> Color {
        let base: Color
        switch tone {
        case .consent: base = .yellow
        case .thinking: base = .purple
        case .tool: base = .blue
        case .artifact: base = .teal
        case .elicitation: base = .teal
        case .error: base = .red
        case .info: base = .secondary
        }
        return base.opacity(scheme == .dark ? 0.16 : 0.12)
    }

    static func mutedActionFont() -> Font {
        .caption
    }

    static func activityPrimaryLabel() -> Color {
        Color.secondary
    }

    static func activityMutedDuration() -> Color {
        Color(nsColor: .tertiaryLabelColor)
    }

    static func hairline(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.white.opacity(0.08)
            : Color.black.opacity(0.08)
    }

    static func composerShadow(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color.black.opacity(0.35)
            : Color.black.opacity(0.12)
    }

    enum SemanticTone {
        case consent, thinking, tool, artifact, elicitation, error, info
    }
}

enum TamtriFormatting {
    static func relativeTimestamp(from isoString: String) -> String {
        guard let date = parseDate(isoString) else {
            return isoString
        }
        let interval = date.timeIntervalSinceNow
        if abs(interval) < 60 {
            return "Just now"
        }
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: date, relativeTo: Date())
    }

    /// Codex-style compact stamp: `5m`, `2h`, `3d`, `1w`.
    /// Bubble hover stamp: relative for recent messages, short date otherwise.
    static func messageBubbleTimestamp(from isoString: String?) -> String? {
        guard let isoString, let date = parseDate(isoString) else {
            return nil
        }
        let seconds = abs(date.timeIntervalSinceNow)
        if seconds < 86_400 {
            return relativeTimestamp(from: isoString)
        }
        let formatter = DateFormatter()
        formatter.dateFormat = "MMM d"
        return formatter.string(from: date)
    }

    /// Action bar stamp: weekday and time, e.g. "Thursday 10:50 PM".
    static func messageActionBarTimestamp(from isoString: String?) -> String? {
        guard let isoString, let date = parseDate(isoString) else {
            return nil
        }
        let formatter = DateFormatter()
        formatter.dateFormat = "EEEE h:mm a"
        return formatter.string(from: date)
    }

    static func compactRelativeTimestamp(from isoString: String) -> String {
        guard let date = parseDate(isoString) else {
            return isoString
        }
        let seconds = abs(date.timeIntervalSinceNow)
        if seconds < 60 { return "now" }
        if seconds < 3_600 { return "\(Int(seconds / 60))m" }
        if seconds < 86_400 { return "\(Int(seconds / 3_600))h" }
        if seconds < 604_800 { return "\(Int(seconds / 86_400))d" }
        if seconds < 2_592_000 { return "\(Int(seconds / 604_800))w" }
        if seconds < 31_536_000 { return "\(Int(seconds / 2_592_000))mo" }
        return "\(Int(seconds / 31_536_000))y"
    }

    static func sidebarSectionTitle(for date: Date) -> String {
        let calendar = Calendar.current
        if calendar.isDateInToday(date) {
            return "Today"
        }
        if calendar.isDateInYesterday(date) {
            return "Yesterday"
        }
        return "Earlier"
    }

    static func parseDate(_ isoString: String) -> Date? {
        let withFraction = ISO8601DateFormatter()
        withFraction.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = withFraction.date(from: isoString) {
            return date
        }
        let plain = ISO8601DateFormatter()
        plain.formatOptions = [.withInternetDateTime]
        return plain.date(from: isoString)
    }
}
