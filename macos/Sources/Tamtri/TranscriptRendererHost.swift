import SwiftUI
import WebKit

enum TranscriptRendererBundle {
    /// Cold-start budget target lives in UserPreferences.coldStartBudgetMs.
    static func indexURL() -> URL? {
        let candidates: [URL?] = [
            Bundle.main.resourceURL?.appendingPathComponent("renderer/dist/index.html"),
            URL(fileURLWithPath: "../renderer/dist/index.html", relativeTo: URL(fileURLWithPath: FileManager.default.currentDirectoryPath)),
            URL(fileURLWithPath: "renderer/dist/index.html", relativeTo: URL(fileURLWithPath: FileManager.default.currentDirectoryPath)),
        ]
        for candidate in candidates {
            guard let url = candidate, FileManager.default.fileExists(atPath: url.path) else { continue }
            return url
        }
        return nil
    }

    static var isAvailable: Bool { indexURL() != nil }
}

struct TranscriptRendererHost: NSViewRepresentable {
    let messagesJSON: String

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.defaultWebpagePreferences.allowsContentJavaScript = true
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.setValue(false, forKey: "drawsBackground")
        context.coordinator.webView = webView
        load(into: webView)
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        context.coordinator.render(messagesJSON: messagesJSON, in: webView)
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    private func load(into webView: WKWebView) {
        guard let url = TranscriptRendererBundle.indexURL() else { return }
        webView.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
    }

    final class Coordinator {
        weak var webView: WKWebView?
        private var lastPayload = ""

        func render(messagesJSON: String, in webView: WKWebView) {
            guard messagesJSON != lastPayload else { return }
            lastPayload = messagesJSON
            let escaped = messagesJSON
                .replacingOccurrences(of: "\\", with: "\\\\")
                .replacingOccurrences(of: "'", with: "\\'")
                .replacingOccurrences(of: "\n", with: "\\n")
            let script = "window.tamtriRender && window.tamtriRender('\(escaped)');"
            Task { @MainActor in
                webView.evaluateJavaScript(script, completionHandler: nil)
            }
        }
    }
}

struct TranscriptRendererSection: View {
    let conversationId: String
    let messages: [ParsedTranscriptMessage]

    var body: some View {
        ForEach(messages) { message in
            if shouldUseRenderer(for: message), TranscriptRendererBundle.isAvailable {
                TranscriptRendererHost(messagesJSON: rendererJSON(for: [message]))
                    .frame(minHeight: 48)
                    .accessibilityLabel("\(message.role) message")
            } else {
                MessageRow(conversationId: conversationId, message: message)
            }
        }
    }

    private func shouldUseRenderer(for message: ParsedTranscriptMessage) -> Bool {
        guard message.rawJSON == nil else { return false }
        return message.content.allSatisfy { block in
            switch block.type {
            case "text", "thinking", "tool_call", "tool_result":
                true
            default:
                false
            }
        }
    }

    private func rendererJSON(for messages: [ParsedTranscriptMessage]) -> String {
        let payload = messages.map { message -> [String: Any] in
            [
                "id": message.id,
                "role": message.role,
                "harness_id": message.harnessId as Any,
                "content": message.content.map { block in
                    [
                        "type": block.type,
                        "text": block.text as Any,
                        "name": block.name as Any,
                        "call_id": block.callId as Any,
                    ]
                },
            ]
        }
        guard let data = try? JSONSerialization.data(withJSONObject: payload),
              let json = String(data: data, encoding: .utf8)
        else {
            return "[]"
        }
        return json
    }
}
