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

    func testWebContentPolicyArtifactMatchesLegacyNavigation() {
        XCTAssertTrue(WebNavigationPolicy.allows(URL(string: "about:blank"), policy: .artifactNoNetwork))
        XCTAssertFalse(WebNavigationPolicy.allows(URL(string: "https://example.com"), policy: .artifactNoNetwork))
        XCTAssertTrue(ArtifactNavigationPolicy.allows(URL(string: "about:blank")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "https://example.com")))
    }

    func testArtifactPolicyBlocksFileNavigation() {
        XCTAssertFalse(WebNavigationPolicy.allows(URL(string: "file:///etc/passwd"), policy: .artifactNoNetwork))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "file:///tmp/report.html")))
    }

    func testAppPolicyBlocksUndeclaredOrigin() {
        let policy = WebContentPolicy.app(
            allowedOrigins: [WebOrigin(mcpOrigin: "https://cdn.example.com")],
            appId: "demo",
            serverId: "fixture",
            templateRef: "ui://fixture/demo"
        )
        XCTAssertFalse(WebNavigationPolicy.allows(URL(string: "https://evil.example"), policy: policy))
        XCTAssertTrue(WebNavigationPolicy.allows(URL(string: "https://cdn.example.com/app.js"), policy: policy))
    }

    func testAppPolicyBlocksRedirectToUndeclaredOrigin() {
        let policy = WebContentPolicy.app(
            allowedOrigins: [WebOrigin(mcpOrigin: "https://cdn.example.com")],
            appId: "demo",
            serverId: "fixture",
            templateRef: "ui://fixture/demo"
        )
        XCTAssertFalse(
            WebNavigationPolicy.allows(URL(string: "https://evil.example/redirected"), policy: policy)
        )
        XCTAssertFalse(
            WebNavigationPolicy.allows(URL(string: "https://cdn.evil.com/login"), policy: policy)
        )
    }

    func testAppStateRehydrationInjectsPersistedJSON() {
        let html = appSandboxedHTML(
            html: "<!doctype html><html><head></head><body>App</body></html>",
            allowedOrigins: [WebOrigin(mcpOrigin: "https://api.example.com")],
            bridgeScript: "",
            persistedStateJSON: #"{"title":"Demo App","value":42}"#
        )
        XCTAssertTrue(html.contains("window.__tamtriAppPersistedState="))
        XCTAssertTrue(html.contains(#"{"title":"Demo App","value":42}"#))
        XCTAssertTrue(html.contains("__tamtriAppRehydrate"))
    }

    func testAppTemplateDeclaredOriginLoads() {
        let html = appSandboxedHTML(
            html: "<!doctype html><html><head></head><body>App</body></html>",
            allowedOrigins: [WebOrigin(mcpOrigin: "https://api.example.com")],
            bridgeScript: "(function(){window.__tamtriAppBridgeInstalled=true;})();"
        )
        XCTAssertTrue(html.contains("connect-src https://api.example.com"))
        XCTAssertTrue(html.contains("__tamtriAppBridgeInstalled"))
        XCTAssertTrue(html.contains("script-src 'unsafe-inline'"))
    }

    func testArtifactWebviewStillHasNoBridge() {
        let html = artifactSandboxedHTML("<html><body><h1>Report</h1></body></html>")
        XCTAssertFalse(artifactHTMLHasBridge(html))
        XCTAssertTrue(html.contains("script-src 'none'"))
    }

    func testArtifactMimeRouting() {
        XCTAssertTrue(artifactIsTextLikeMime("text/html"))
        XCTAssertTrue(artifactIsTextLikeMime("image/svg+xml"))
        XCTAssertFalse(artifactIsTextLikeMime("image/png"))

        XCTAssertTrue(artifactIsImageMime("image/png"))
        XCTAssertTrue(artifactIsImageMime("image/webp"))
        XCTAssertFalse(artifactIsImageMime("image/svg+xml"))

        XCTAssertEqual(artifactMimeLabel("application/pdf"), "PDF")
        XCTAssertEqual(artifactMimeLabel("application/octet-stream"), "application/octet-stream")
        XCTAssertEqual(artifactFileIcon("application/pdf"), "doc.fill")
        XCTAssertEqual(artifactFileIcon(nil), "doc")
    }

    func testMarkdownPreviewSanitizationStripsRawHTML() {
        let input = "# Title\n<script>alert(1)</script>\n**bold** <img onerror=\"x()\">"
        let safe = sanitizedMarkdownForPreview(input)
        XCTAssertFalse(safe.contains("<script"))
        XCTAssertFalse(safe.contains("<img"))
        XCTAssertTrue(safe.contains("# Title"))
        XCTAssertTrue(safe.contains("**bold**"))
    }
}
