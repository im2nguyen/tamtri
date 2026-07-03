import Foundation
import SwiftUI
import WebKit

struct WebTranscriptView: NSViewRepresentable {
    let transcriptJSON: String

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.setValue(false, forKey: "drawsBackground")
        webView.loadHTMLString(Self.shellHTML, baseURL: nil)
        context.coordinator.webView = webView
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        guard context.coordinator.lastTranscriptJSON != transcriptJSON else { return }
        context.coordinator.lastTranscriptJSON = transcriptJSON
        let payload = Self.javascriptStringLiteral(transcriptJSON)
        webView.evaluateJavaScript("window.tamtriSetTranscript(\(payload))", completionHandler: nil)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    final class Coordinator {
        weak var webView: WKWebView?
        var lastTranscriptJSON = ""
    }

    private static func javascriptStringLiteral(_ value: String) -> String {
        let data = try? JSONSerialization.data(withJSONObject: value, options: [])
        if let data, let encoded = String(data: data, encoding: .utf8) {
            return encoded
        }
        return "\"\""
    }

    private static let shellHTML = """
    <!doctype html>
    <html>
    <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data:;">
    <style>
    :root { color-scheme: light dark; font: -apple-system-body; }
    body { margin: 0; padding: 12px 16px 24px; background: transparent; color: CanvasText; }
    .message { margin: 0 0 14px; }
    .meta { font-size: 11px; font-weight: 600; text-transform: capitalize; opacity: 0.65; margin-bottom: 6px; }
    .text { white-space: pre-wrap; line-height: 1.45; }
    details { margin: 6px 0; border-radius: 8px; background: color-mix(in srgb, CanvasText 6%, transparent); }
    summary { cursor: pointer; padding: 8px 10px; font-weight: 600; }
    details pre, .tool pre { margin: 0; padding: 0 10px 10px; white-space: pre-wrap; word-break: break-word; font: 12px ui-monospace, Menlo, monospace; opacity: 0.9; }
    .tool { margin: 6px 0; padding: 8px 10px; border-radius: 8px; background: color-mix(in srgb, CanvasText 6%, transparent); }
    .tool-title { font-weight: 600; margin-bottom: 4px; }
    .muted { opacity: 0.55; font-size: 12px; }
    </style>
    </head>
    <body><div id="root"></div>
    <script>
    const MAX_PREVIEW = 600;
    function esc(s) {
      return String(s ?? '').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
    }
    function truncate(value) {
      const s = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
      if (!s) return '';
      return s.length > MAX_PREVIEW ? s.slice(0, MAX_PREVIEW) + '…' : s;
    }
    function renderBlock(block) {
      switch (block.type) {
        case 'text':
          return '<div class="text">' + esc(block.text) + '</div>';
        case 'thinking':
          return '<details><summary>Thinking</summary><pre>' + esc(block.text) + '</pre></details>';
        case 'tool_call':
          return '<div class="tool"><div class="tool-title">Tool: ' + esc(block.name || 'call') + '</div><pre>' + esc(truncate(block.input)) + '</pre></div>';
        case 'tool_result':
          return '<div class="tool"><div class="tool-title">Tool result</div><pre>' + esc(truncate(block.output)) + '</pre></div>';
        default:
          return '';
      }
    }
    function renderMessage(msg) {
      const role = esc(msg.role || 'message');
      const harness = msg.harness_id ? '<span class="muted"> · ' + esc(msg.harness_id) + '</span>' : '';
      const blocks = Array.isArray(msg.content) ? msg.content.map(renderBlock).join('') : '';
      return '<article class="message"><div class="meta">' + role + harness + '</div>' + blocks + '</article>';
    }
    window.tamtriSetTranscript = function(jsonText) {
      const root = document.getElementById('root');
      try {
        const messages = JSON.parse(jsonText || '[]');
        root.innerHTML = messages.map(renderMessage).join('');
      } catch (err) {
        root.innerHTML = '<div class="muted">Unable to render transcript.</div>';
      }
    };
    </script>
    </body>
    </html>
    """
}
