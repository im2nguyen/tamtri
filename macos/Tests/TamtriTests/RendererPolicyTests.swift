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

    func testLargePlainTextArtifactPreviewUsesLineLimit() {
        XCTAssertEqual(artifactPlainTextPreviewLineLimit, 24)
        let largeText = (0..<100).map { "line \($0)" }.joined(separator: "\n")
        XCTAssertGreaterThan(largeText.components(separatedBy: "\n").count, artifactPlainTextPreviewLineLimit)
    }

    func testCSVPreviewCapsRowsAndColumns() {
        let header = (0..<10).map { "col\($0)" }.joined(separator: ",")
        let rows = (0..<30).map { row in
            (0..<10).map { col in "r\(row)c\(col)" }.joined(separator: ",")
        }
        let csv = ([header] + rows).joined(separator: "\n")
        let preview = csvPreviewRows(text: csv, separator: ",")

        XCTAssertEqual(preview.count, artifactCSVPreviewMaxRows)
        XCTAssertTrue(preview.allSatisfy { $0.count <= artifactCSVPreviewMaxColumns })
        XCTAssertEqual(preview[0], (0..<8).map { "col\($0)" })
        XCTAssertEqual(preview[19], (0..<8).map { "r18c\($0)" })
    }

    func testCSVPreviewStylesFirstRowAsHeader() {
        let csv = "name,value\nalpha,1\nbeta,2"
        let preview = csvPreviewRows(text: csv, separator: ",")
        XCTAssertEqual(preview.first, ["name", "value"])
        XCTAssertEqual(preview[1], ["alpha", "1"])
    }

    func testArtifactNavigationPolicyBlocksExternalSchemes() {
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "https://example.com")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "http://example.com")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "ftp://example.com/file")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "mailto:user@example.com")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "javascript:alert(1)")))
        XCTAssertFalse(ArtifactNavigationPolicy.allows(URL(string: "tamtri://open")))
    }

    func testArtifactSandboxCSPBlocksNetworkAndScripts() {
        let html = artifactSandboxedHTML(
            "<html><head></head><body><img src=\"https://evil.example/x.png\"><script>fetch('https://evil.example')</script></body></html>"
        )
        XCTAssertTrue(html.contains("default-src 'none'"))
        XCTAssertTrue(html.contains("script-src 'none'"))
        XCTAssertTrue(html.contains("form-action 'none'"))
        XCTAssertTrue(html.contains("base-uri 'none'"))
        XCTAssertTrue(html.contains("img-src data:"))
    }

    func testIntegrityFailureUsesTypedFileCardAccessibility() {
        let label = artifactCardAccessibilityLabel(
            path: "attachments/report.html",
            mimeType: "text/html",
            integrityFailed: true
        )
        XCTAssertEqual(label, "report.html, integrity check failed")

        let value = artifactCardAccessibilityValue(
            integrityFailed: true,
            size: 128,
            previewLoaded: false,
            imageLoaded: false,
            nonPreviewable: false,
            loading: false
        )
        XCTAssertEqual(value, "integrity check failed")
    }

    func testTamperedHashDoesNotUseWebViewPreview() {
        XCTAssertFalse(
            artifactShouldUseWebViewPreview(
                mimeType: "text/html",
                integrityFailed: true,
                hasVerifiedContent: true
            )
        )
        XCTAssertFalse(
            artifactShouldUseWebViewPreview(
                mimeType: "image/svg+xml",
                integrityFailed: true,
                hasVerifiedContent: true
            )
        )
        XCTAssertTrue(
            artifactShouldUseWebViewPreview(
                mimeType: "text/html",
                integrityFailed: false,
                hasVerifiedContent: true
            )
        )
        XCTAssertFalse(
            artifactShouldUseWebViewPreview(
                mimeType: "text/html",
                integrityFailed: false,
                hasVerifiedContent: false
            )
        )
    }
}
