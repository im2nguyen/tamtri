import Foundation
@testable import Tamtri
import XCTest

final class RendererPolicyTests: XCTestCase {
    func testSandboxedHTMLInjectsNetworkBlockingCSP() {
        let html = artifactSandboxedHTML("<!doctype html><html><head><title>x</title></head><body></body></html>")

        XCTAssertTrue(html.contains("Content-Security-Policy"))
        XCTAssertTrue(html.contains("default-src 'none'"))
        XCTAssertTrue(html.contains("script-src 'none'"))
        XCTAssertTrue(html.contains("form-action 'none'"))
    }

    func testArtifactNavigationPolicyBlocksExternalNavigation() {
        XCTAssertTrue(ArtifactNavigationPolicy.allows(URL(string: "about:blank")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "https://example.com")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(fileURLWithPath: "/tmp/report.html")))
    }

    func testArtifactMimeRouting() {
        XCTAssertTrue(artifactIsTextLikeMime("text/html"))
        XCTAssertTrue(artifactIsTextLikeMime("image/svg+xml"))
        XCTAssertFalse(artifactIsTextLikeMime("image/png"))

        XCTAssertTrue(artifactIsImageMime("image/png"))
        XCTAssertTrue(artifactIsImageMime("image/webp"))
        XCTAssertFalse(artifactIsImageMime("image/svg+xml"))
    }
}
