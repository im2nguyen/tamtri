import Foundation

/// Declared network origin for MCP App sandbox policy.
struct WebOrigin: Equatable, Hashable {
    let scheme: String
    let host: String
    let port: Int?

    init(scheme: String, host: String, port: Int? = nil) {
        self.scheme = scheme.lowercased()
        self.host = host.lowercased()
        self.port = port
    }

    init(mcpOrigin: String) {
        if let url = URL(string: mcpOrigin), let host = url.host {
            self.init(scheme: url.scheme ?? "https", host: host, port: url.port)
        } else {
            self.init(scheme: "https", host: mcpOrigin.lowercased(), port: nil)
        }
    }

    var mcpOriginString: String {
        if let port {
            return "\(scheme)://\(host):\(port)"
        }
        return "\(scheme)://\(host)"
    }
}

/// Sandbox policy for WKWebView renderer islands.
enum WebContentPolicy: Equatable {
    /// Harness artifacts: no network, no host bridge, strict CSP.
    case artifactNoNetwork
    /// MCP Apps: declared origins only, consent-gated JSON-RPC bridge.
    case app(allowedOrigins: [WebOrigin], appId: String, serverId: String, templateRef: String)
}

enum WebNavigationPolicy {
    static func allows(_ url: URL?, policy: WebContentPolicy) -> Bool {
        switch policy {
        case .artifactNoNetwork:
            return artifactNavigationAllows(url)
        case .app(let allowedOrigins, _, _, _):
            guard let url else { return true }
            if url.scheme == "about" || url.scheme == nil {
                return true
            }
            let origin = url.originString
            return allowedOrigins.contains { $0.matches(url: url) || $0.mcpOriginString == origin }
        }
    }
}

extension WebOrigin {
    func matches(url: URL) -> Bool {
        guard let scheme = url.scheme?.lowercased(), let host = url.host?.lowercased() else {
            return false
        }
        return self.scheme == scheme && self.host == host && self.port == url.port
    }
}

private extension URL {
    var originString: String {
        guard let host else { return absoluteString }
        if let port {
            return "\(scheme ?? "https")://\(host):\(port)"
        }
        return "\(scheme ?? "https")://\(host)"
    }
}

func sandboxedHTML(for html: String, policy: WebContentPolicy, bridgeScript: String? = nil, persistedStateJSON: String? = nil) -> String {
    switch policy {
    case .artifactNoNetwork:
        return artifactSandboxedHTML(html)
    case .app(let allowedOrigins, _, _, _):
        return appSandboxedHTML(
            html: html,
            allowedOrigins: allowedOrigins,
            bridgeScript: bridgeScript ?? "",
            persistedStateJSON: persistedStateJSON
        )
    }
}

private func artifactNavigationAllows(_ url: URL?) -> Bool {
    guard let url else { return true }
    return url.scheme == "about" || url.scheme == nil
}

func artifactSandboxedHTML(_ html: String) -> String {
    let csp = "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; base-uri 'none'; form-action 'none'\">"
    return wrapHTML(html, headInjection: csp)
}

func appSandboxedHTML(html: String, allowedOrigins: [WebOrigin], bridgeScript: String, persistedStateJSON: String? = nil) -> String {
    let connect = allowedOrigins.map(\.mcpOriginString).joined(separator: " ")
    let csp = "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline' 'self'; connect-src \(connect) about:; img-src data:; base-uri 'none'; form-action 'none'\">"
    let bridge = bridgeScript.isEmpty ? "" : "<script>\(bridgeScript)</script>"
    let rehydrate = appStateRehydrationScript(persistedStateJSON: persistedStateJSON)
    return wrapHTML(html, headInjection: csp + bridge + rehydrate)
}

func appStateRehydrationScript(persistedStateJSON: String?) -> String {
    guard let persistedStateJSON,
          !persistedStateJSON.isEmpty,
          persistedStateJSON != "{}",
          persistedStateJSON != "null"
    else {
        return ""
    }
    return """
    <script>window.__tamtriAppPersistedState=\(persistedStateJSON);\
    if(typeof window.__tamtriAppRehydrate==='function'){window.__tamtriAppRehydrate(window.__tamtriAppPersistedState);}</script>
    """
}

private func wrapHTML(_ html: String, headInjection: String) -> String {
    if let headRange = html.range(of: "<head", options: [.caseInsensitive]),
       let close = html[headRange.upperBound...].firstIndex(of: ">") {
        var copy = html
        copy.insert(contentsOf: headInjection, at: html.index(after: close))
        return copy
    }
    return "<!doctype html><html><head>\(headInjection)</head><body>\(html)</body></html>"
}

/// Artifacts must never expose the MCP App bridge bootstrap.
func artifactHTMLHasBridge(_ html: String) -> Bool {
    html.contains("__tamtriAppBridgeInstalled") || html.contains("tamtriAppBridge")
}

/// Back-compat alias used by existing tests.
enum ArtifactNavigationPolicy {
    static func allows(_ url: URL?) -> Bool {
        WebNavigationPolicy.allows(url, policy: .artifactNoNetwork)
    }
}

/// Strip raw HTML and dangerous blocks before markdown preview rendering.
func sanitizedMarkdownForPreview(_ content: String) -> String {
    var sanitized = content
    let dangerousBlockPatterns = [
        "(?is)<script\\b[^>]*>.*?</script>",
        "(?is)<style\\b[^>]*>.*?</style>",
        "(?is)<iframe\\b[^>]*>.*?</iframe>",
        "(?is)<object\\b[^>]*>.*?</object>",
        "(?is)<embed\\b[^>]*>",
    ]
    for pattern in dangerousBlockPatterns {
        sanitized = sanitized.replacingOccurrences(
            of: pattern,
            with: "",
            options: .regularExpression
        )
    }
    sanitized = sanitized.replacingOccurrences(
        of: "<[^>]+>",
        with: "",
        options: .regularExpression
    )
    return sanitized
}

func attributedMarkdownPreview(_ content: String) -> AttributedString? {
    let safe = sanitizedMarkdownForPreview(content)
    var options = AttributedString.MarkdownParsingOptions()
    options.interpretedSyntax = .inlineOnlyPreservingWhitespace
    return try? AttributedString(markdown: safe, options: options)
}
