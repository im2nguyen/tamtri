import XCTest
@testable import Tamtri

final class TranscriptMarkdownTests: XCTestCase {
    private let reportSample = """
    Done — I created `report.html` in the working directory and verified it renders as a self-contained HTML report.

    Includes:
    - Executive summary
    - Data quality notes
    - Top 5 findings
    - Two inline SVG charts:
      - Revenue by product line
      - Weekly revenue trend

    Verified summary from the CSV:
    - 30 rows
    - $355,690 total revenue
    - 3,158 units sold
    - No missing values or exact duplicates found

    If you want, I can also generate...
    """

    func testNormalizedTranscriptMarkdownPreservesParagraphsAndLists() {
        let normalized = normalizedTranscriptMarkdown(reportSample)

        XCTAssertTrue(normalized.contains("Done — I created `report.html`"))
        XCTAssertTrue(normalized.contains("\n\nIncludes:"))
        XCTAssertTrue(normalized.contains("\n\n- Executive summary"))
        XCTAssertTrue(normalized.contains("- Two inline SVG charts:"))
        XCTAssertTrue(normalized.contains("  - Revenue by product line"))
        XCTAssertTrue(normalized.contains("\n\nVerified summary from the CSV:"))
        XCTAssertFalse(normalized.contains("report.Includes"))
    }

    func testNormalizedTranscriptMarkdownInsertsBlankLineBeforeTopLevelListOnly() {
        let input = "Includes:\n- Executive summary\n  - Nested item"
        let normalized = normalizedTranscriptMarkdown(input)

        XCTAssertTrue(normalized.contains("Includes:\n\n- Executive summary"))
        XCTAssertTrue(normalized.contains("- Executive summary\n  - Nested item"))
        XCTAssertFalse(normalized.contains("- Executive summary\n\n  - Nested item"))
    }

    func testTranscriptMarkdownBlocksSplitParagraphsAndLists() {
        let blocks = transcriptMarkdownBlocks(reportSample)

        XCTAssertFalse(blocks.isEmpty)
        XCTAssertTrue(blocks.contains { block in
            if case .paragraph(let text) = block {
                return text.contains("Done — I created `report.html`")
            }
            return false
        })

        let listItems = blocks.flatMap { block -> [TranscriptMarkdownListItem] in
            if case .list(let items) = block { return items }
            return []
        }
        XCTAssertTrue(listItems.contains { $0.text == "Executive summary" && $0.indent == 0 })
        XCTAssertTrue(listItems.contains { $0.text == "Revenue by product line" && $0.indent == 1 })
    }

    func testAttributedTranscriptMarkdownPreservesInlineCode() throws {
        let attributed = try XCTUnwrap(attributedTranscriptMarkdown("Use `report.html` here."))
        let plain = String(attributed.characters)
        XCTAssertTrue(plain.contains("report.html"))
    }
}
