import SwiftUI

struct TranscriptMarkdownListItem: Equatable {
    let indent: Int
    let text: String
}

enum TranscriptMarkdownBlock: Equatable {
    case paragraph(String)
    case list([TranscriptMarkdownListItem])
}

func normalizedTranscriptMarkdown(_ content: String) -> String {
    var text = sanitizedMarkdownForPreview(content)
    text = text.replacingOccurrences(of: "\r\n", with: "\n")

    // Top-level list markers need a blank line before them when preceded by prose.
    text = text.replacingOccurrences(
        of: "(?<![\\n\\r])(\\n)(?=[\\-*+]\\s|\\d+\\.\\s)",
        with: "\n\n",
        options: .regularExpression
    )

    // Preserve single line breaks in prose as hard breaks (two trailing spaces).
    let paragraphs = text.components(separatedBy: "\n\n")
    let normalized = paragraphs.map { paragraph -> String in
        let lines = paragraph.components(separatedBy: "\n")
        guard lines.count > 1 else { return paragraph }
        let isListBlock = lines.contains { line in
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty else { return false }
            return trimmed.hasPrefix("- ")
                || trimmed.hasPrefix("* ")
                || trimmed.hasPrefix("+ ")
                || trimmed.range(of: "^\\d+\\.\\s", options: .regularExpression) != nil
        }
        if isListBlock { return paragraph }
        return lines.joined(separator: "  \n")
    }
    return normalized.joined(separator: "\n\n")
}

func transcriptMarkdownBlocks(_ content: String) -> [TranscriptMarkdownBlock] {
    let normalized = normalizedTranscriptMarkdown(content)
    guard !normalized.isEmpty else { return [] }

    var blocks: [TranscriptMarkdownBlock] = []
    var paragraphLines: [String] = []
    var listItems: [TranscriptMarkdownListItem] = []

    func flushParagraph() {
        guard !paragraphLines.isEmpty else { return }
        let text = paragraphLines.joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
        paragraphLines.removeAll()
        guard !text.isEmpty else { return }
        blocks.append(.paragraph(text))
    }

    func flushList() {
        guard !listItems.isEmpty else { return }
        blocks.append(.list(listItems))
        listItems.removeAll()
    }

    for line in normalized.components(separatedBy: "\n") {
        if line.isEmpty {
            flushList()
            flushParagraph()
            continue
        }

        if let item = parseTranscriptMarkdownListLine(line) {
            flushParagraph()
            listItems.append(item)
            continue
        }

        flushList()
        paragraphLines.append(line)
    }

    flushList()
    flushParagraph()
    return blocks
}

private func parseTranscriptMarkdownListLine(_ line: String) -> TranscriptMarkdownListItem? {
    let leadingSpaces = line.prefix(while: { $0 == " " }).count
    let trimmed = line.trimmingCharacters(in: .whitespaces)
    let markerPattern = "^(?:[-*+]|\\d+\\.)\\s+"
    guard let markerRange = trimmed.range(of: markerPattern, options: .regularExpression) else {
        return nil
    }
    let text = String(trimmed[markerRange.upperBound...])
    guard !text.isEmpty else { return nil }
    let indent = max(0, leadingSpaces / 2)
    return TranscriptMarkdownListItem(indent: indent, text: text)
}

func attributedTranscriptMarkdown(_ content: String) -> AttributedString? {
    attributedTranscriptInlineMarkdown(content)
}

func attributedTranscriptInlineMarkdown(_ content: String) -> AttributedString? {
    let safe = content.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !safe.isEmpty else { return nil }
    var options = AttributedString.MarkdownParsingOptions()
    options.allowsExtendedAttributes = true
    options.interpretedSyntax = .inlineOnlyPreservingWhitespace
    return try? AttributedString(markdown: safe, options: options)
}

func styledTranscriptInlineMarkdownText(
    _ content: String,
    colorScheme: ColorScheme,
    muted: Bool
) -> Text? {
    guard let attributed = attributedTranscriptInlineMarkdown(content) else { return nil }

    let proseFont = muted ? TamtriTheme.mutedActionFont() : TamtriTheme.proseFont()
    let foreground: Color = muted ? Color.secondary : Color.primary
    let codeForeground: Color = muted ? Color.secondary : Color.primary

    var result: Text?
    for run in attributed.runs {
        let slice = String(attributed[run.range].characters)
        let runText: Text
        if run.inlinePresentationIntent?.contains(.code) == true {
            if #available(macOS 15, *) {
                runText = Text(slice)
                    .font(TamtriTheme.inlineCodeFont())
                    .foregroundStyle(codeForeground)
                    .customAttribute(InlineCodeAttribute())
            } else {
                runText = Text(slice)
                    .font(TamtriTheme.inlineCodeFont())
                    .foregroundStyle(codeForeground)
            }
        } else {
            runText = Text(slice)
                .font(proseFont)
                .foregroundStyle(foreground)
        }

        if let existing = result {
            result = existing + runText
        } else {
            result = runText
        }
    }

    return result
}

func legacyStyledTranscriptInlineMarkdownText(
    _ content: String,
    colorScheme: ColorScheme,
    muted: Bool
) -> Text? {
    guard let attributed = attributedTranscriptInlineMarkdown(content) else { return nil }

    let proseFont = muted ? TamtriTheme.mutedActionFont() : TamtriTheme.proseFont()
    let foreground: Color = muted ? Color.secondary : Color.primary
    let codeForeground: Color = muted ? Color.secondary : Color.primary
    let codeBackground = TamtriTheme.inlineCodeBackground(colorScheme)
    let horizontalPadding = String(repeating: "\u{2009}", count: 3)

    var result = AttributedString()
    for run in attributed.runs {
        let slice = String(attributed[run.range].characters)
        if run.inlinePresentationIntent?.contains(.code) == true {
            var code = AttributedString("\(horizontalPadding)\(slice)\(horizontalPadding)")
            code.font = TamtriTheme.inlineCodeFont()
            code.foregroundColor = codeForeground
            code.backgroundColor = codeBackground
            result.append(code)
        } else {
            var prose = AttributedString(slice)
            prose.font = proseFont
            prose.foregroundColor = foreground
            result.append(prose)
        }
    }

    return Text(result)
}

@available(macOS 15, *)
private struct InlineCodeAttribute: TextAttribute {}

@available(macOS 15, *)
struct InlineCodePillRenderer: TextRenderer {
    var background: Color
    var cornerRadius: CGFloat
    var horizontalPadding: CGFloat
    var verticalPadding: CGFloat

    func draw(layout: Text.Layout, in context: inout GraphicsContext) {
        for line in layout {
            for run in line {
                if run[InlineCodeAttribute.self] != nil {
                    let rect = run.typographicBounds.rect.insetBy(
                        dx: -horizontalPadding,
                        dy: -verticalPadding
                    )
                    let shape = RoundedRectangle(cornerRadius: cornerRadius).path(in: rect)
                    context.fill(shape, with: .color(background))
                }
                context.draw(run)
            }
        }
    }
}

struct TranscriptMarkdownText: View {
    let content: String
    var muted: Bool = true

    @Environment(\.colorScheme) private var colorScheme

    private var blocks: [TranscriptMarkdownBlock] {
        transcriptMarkdownBlocks(content)
    }

    var body: some View {
        if blocks.isEmpty {
            EmptyView()
        } else if blocks.count == 1, case .paragraph(let text) = blocks[0] {
            inlineMarkdownText(text)
        } else {
            VStack(alignment: .leading, spacing: muted ? TamtriSpacing.xs : TamtriSpacing.sm) {
                ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in
                    blockView(block)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder
    private func blockView(_ block: TranscriptMarkdownBlock) -> some View {
        switch block {
        case .paragraph(let text):
            inlineMarkdownText(text)
        case .list(let items):
            listView(items)
        }
    }

    @ViewBuilder
    private func inlineMarkdownText(_ text: String) -> some View {
        if let styled = styledTranscriptInlineMarkdownText(text, colorScheme: colorScheme, muted: muted) {
            styled
                .lineSpacing(muted ? 0 : TamtriLayout.transcriptProseLineSpacing)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            Text(text)
                .font(muted ? TamtriTheme.mutedActionFont() : TamtriTheme.proseFont())
                .foregroundStyle(muted ? .tertiary : .primary)
                .lineSpacing(muted ? 0 : TamtriLayout.transcriptProseLineSpacing)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder
    private func listView(_ items: [TranscriptMarkdownListItem]) -> some View {
        VStack(alignment: .leading, spacing: TamtriSpacing.xs) {
            ForEach(Array(items.enumerated()), id: \.offset) { _, item in
                HStack(alignment: .firstTextBaseline, spacing: TamtriSpacing.sm) {
                    Text("•")
                        .font(muted ? TamtriTheme.mutedActionFont() : TamtriTheme.proseFont())
                        .foregroundStyle(muted ? .tertiary : .secondary)
                        .padding(.leading, CGFloat(item.indent) * TamtriSpacing.lg)
                    inlineMarkdownText(item.text)
                }
            }
        }
    }
}

struct ToolOutputText: View {
    let content: String

    var body: some View {
        Text(content)
            .font(TamtriTheme.monoDetailFont())
            .foregroundStyle(.secondary)
            .textSelection(.enabled)
            .multilineTextAlignment(.leading)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct TranscriptSectionHeader: View {
    let title: String

    var body: some View {
        Text(title.uppercased())
            .font(TamtriTheme.sidebarSectionFont())
            .foregroundStyle(.tertiary)
            .padding(.top, TamtriSpacing.sm)
            .padding(.bottom, TamtriSpacing.xs)
            .accessibilityAddTraits(.isHeader)
    }
}
