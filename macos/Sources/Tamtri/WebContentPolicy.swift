import Foundation

/// Declared network origin for MCP App sandbox policy (PR3 will enforce fetch/connect).
struct WebOrigin: Equatable, Hashable {
    let scheme: String
    let host: String
    let port: Int?

    init(scheme: String, host: String, port: Int? = nil) {
        self.scheme = scheme.lowercased()
        self.host = host.lowercased()
        self.port = port
    }
}

/// Sandbox policy for WKWebView renderer islands.
enum WebContentPolicy: Equatable {
    /// Harness artifacts: no network, no host bridge, strict CSP.
    case artifactNoNetwork
    /// MCP Apps (PR3): declared origins only, consent-gated JSON-RPC bridge.
    case app(allowedOrigins: [WebOrigin], appId: String, serverId: String)
}

enum WebNavigationPolicy {
    static func allows(_ url: URL?, policy: WebContentPolicy) -> Bool {
        switch policy {
        case .artifactNoNetwork:
            return artifactNavigationAllows(url)
        case .app(let allowedOrigins, _, _):
            guard let url else { return true }
            guard let scheme = url.scheme?.lowercased(), let host = url.host?.lowercased() else {
                return url.scheme == "about" || url.scheme == nil
            }
            let port = url.port
            let origin = WebOrigin(scheme: scheme, host: host, port: port)
            if allowedOrigins.contains(origin) {
                return true
            }
            return url.scheme == "about" || url.scheme == nil
        }
    }
}

func sandboxedHTML(for html: String, policy: WebContentPolicy) -> String {
    switch policy {
    case .artifactNoNetwork:
        return artifactSandboxedHTML(html)
    case .app:
        // PR3 will widen CSP connect-src to declared origins and enable the bridge.
        return artifactSandboxedHTML(html)
    }
}

private func artifactNavigationAllows(_ url: URL?) -> Bool {
    guard let url else { return true }
    return url.scheme == "about" || url.scheme == nil
}

func artifactSandboxedHTML(_ html: String) -> String {
    let csp = "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; base-uri 'none'; form-action 'none'\">"
    if let headRange = html.range(of: "<head", options: [.caseInsensitive]),
       let close = html[headRange.upperBound...].firstIndex(of: ">") {
        var copy = html
        copy.insert(contentsOf: csp, at: html.index(after: close))
        return copy
    }
    return "<!doctype html><html><head>\(csp)</head><body>\(html)</body></html>"
}

/// Back-compat alias used by existing tests.
enum ArtifactNavigationPolicy {
    static func allows(_ url: URL?) -> Bool {
        WebNavigationPolicy.allows(url, policy: .artifactNoNetwork)
    }
}
