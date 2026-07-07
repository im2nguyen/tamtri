import XCTest
@testable import Tamtri

final class ThoughtDurationFormattingTests: XCTestCase {
    func testFormatSeconds() {
        XCTAssertEqual(ThoughtDurationFormatting.format(seconds: 0.4), "1s")
        XCTAssertEqual(ThoughtDurationFormatting.format(seconds: 5), "5s")
        XCTAssertEqual(ThoughtDurationFormatting.format(seconds: 12), "12s")
    }

    func testFormatMinutes() {
        XCTAssertEqual(ThoughtDurationFormatting.format(seconds: 60), "1m")
        XCTAssertEqual(ThoughtDurationFormatting.format(seconds: 63), "1m 3s")
    }

    func testDurationFromMeasuredInterval() {
        let started = Date(timeIntervalSince1970: 0)
        let ended = Date(timeIntervalSince1970: 5)
        let presentation = ThinkingDisclosurePresentation(
            title: "Thinking",
            text: "plan",
            initiallyExpanded: false,
            startedAt: started,
            endedAt: ended
        )
        XCTAssertEqual(ThoughtDurationFormatting.duration(for: presentation), 5)
        XCTAssertEqual(
            ThoughtDurationFormatting.thoughtHeaderLabel(for: presentation),
            "Thought for 5s"
        )
    }

    func testDurationFromCommittedEstimate() {
        let presentation = ThinkingDisclosurePresentation(
            title: "Thinking",
            text: String(repeating: "a", count: 120),
            initiallyExpanded: false,
            estimatedDurationSeconds: 2
        )
        XCTAssertEqual(ThoughtDurationFormatting.duration(for: presentation), 2)
    }
}
