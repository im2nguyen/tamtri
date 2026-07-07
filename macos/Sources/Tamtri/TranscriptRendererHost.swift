import SwiftUI
import WebKit
import AppKit

enum TranscriptRendererBundle {
    static func indexURL() -> URL? {
        let candidates: [URL?] = [
            Bundle.main.resourceURL?.appendingPathComponent("renderer/dist/index.html"),
            Bundle.main.executableURL?
                .deletingLastPathComponent()
                .appendingPathComponent("../../../../renderer/dist/index.html"),
            URL(fileURLWithPath: "../renderer/dist/index.html", relativeTo: URL(fileURLWithPath: FileManager.default.currentDirectoryPath)),
            URL(fileURLWithPath: "renderer/dist/index.html", relativeTo: URL(fileURLWithPath: FileManager.default.currentDirectoryPath)),
        ]
        for candidate in candidates {
            guard let url = candidate?.standardizedFileURL,
                  FileManager.default.fileExists(atPath: url.path)
            else { continue }
            return url
        }
        return nil
    }

    static var isAvailable: Bool { indexURL() != nil }
}

struct TranscriptRendererHost: NSViewRepresentable {
    let messagesJSON: String
    @Binding var contentHeight: CGFloat
    var onUnavailable: (() -> Void)?

    func makeCoordinator() -> Coordinator {
        Coordinator(contentHeight: $contentHeight, onUnavailable: onUnavailable)
    }

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.defaultWebpagePreferences.allowsContentJavaScript = true
        config.userContentController.add(context.coordinator, name: "tamtriHeight")
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.setValue(false, forKey: "drawsBackground")
        webView.focusRingType = .none
        webView.navigationDelegate = context.coordinator
        for subview in webView.subviews {
            if let scrollView = subview as? NSScrollView {
                scrollView.hasVerticalScroller = false
            }
        }
        context.coordinator.webView = webView
        load(into: webView)
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        context.coordinator.onUnavailable = onUnavailable
        context.coordinator.render(messagesJSON: messagesJSON, in: webView)
    }

    private func load(into webView: WKWebView) {
        guard let url = TranscriptRendererBundle.indexURL() else {
            onUnavailable?()
            return
        }
        webView.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
    }

    final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        @Binding var contentHeight: CGFloat
        var onUnavailable: (() -> Void)?
        weak var webView: WKWebView?
        private var lastPayload = ""
        private var isPageLoaded = false
        private var reportedUnavailable = false

        init(contentHeight: Binding<CGFloat>, onUnavailable: (() -> Void)?) {
            _contentHeight = contentHeight
            self.onUnavailable = onUnavailable
        }

        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            guard message.name == "tamtriHeight", let height = message.body as? Double else { return }
            Task { @MainActor in
                contentHeight = max(48, CGFloat(height))
            }
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            isPageLoaded = true
            verifyBridge(in: webView)
            if !lastPayload.isEmpty {
                pushRender(lastPayload, in: webView)
            }
        }

        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
            markUnavailable()
        }

        func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
            markUnavailable()
        }

        func render(messagesJSON: String, in webView: WKWebView) {
            guard messagesJSON != lastPayload || !isPageLoaded else { return }
            lastPayload = messagesJSON
            guard isPageLoaded else { return }
            pushRender(messagesJSON, in: webView)
        }

        private func verifyBridge(in webView: WKWebView) {
            webView.evaluateJavaScript(
                "typeof window.tamtriRenderBase64 === 'function' || typeof window.tamtriRender === 'function'"
            ) { result, _ in
                guard (result as? Bool) != true else { return }
                Task { @MainActor in
                    self.markUnavailable()
                }
            }
        }

        private func pushRender(_ messagesJSON: String, in webView: WKWebView) {
            let payload = Data(messagesJSON.utf8).base64EncodedString()
            let script = """
            (function() {
              if (window.tamtriRenderBase64) {
                window.tamtriRenderBase64('\(payload)');
              } else if (window.tamtriRender) {
                window.tamtriRender(atob('\(payload)'));
              }
              if (window.tamtriReportHeight && window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.tamtriHeight) {
                window.webkit.messageHandlers.tamtriHeight.postMessage(window.tamtriReportHeight());
              }
            })();
            """
            Task { @MainActor in
                webView.evaluateJavaScript(script) { _, error in
                    if error != nil {
                        self.markUnavailable()
                    }
                }
            }
        }

        private func markUnavailable() {
            guard !reportedUnavailable else { return }
            reportedUnavailable = true
            onUnavailable?()
        }
    }
}

struct TranscriptRendererSection: View {
    let conversationId: String
    let messages: [ParsedTranscriptMessage]

    var body: some View {
        ForEach(Array(segments.enumerated()), id: \.offset) { _, segment in
            switch segment {
            case .native(let message):
                MessageRow(
                    conversationId: conversationId,
                    message: message,
                    previousRole: previousRole(forMessageID: message.id),
                    allMessages: messages
                )
            case .react(let batch):
                TranscriptRendererBatchHost(
                    conversationId: conversationId,
                    messages: batch,
                    firstMessagePreviousRole: batch.first.flatMap { previousRole(forMessageID: $0.id) }
                )
            }
        }
    }

    private func previousRole(forMessageID id: String) -> String? {
        guard let index = messages.firstIndex(where: { $0.id == id }), index > 0 else {
            return nil
        }
        return messages[index - 1].role
    }

    private enum Segment {
        case native(ParsedTranscriptMessage)
        case react([ParsedTranscriptMessage])
    }

    private var segments: [Segment] {
        var result: [Segment] = []
        var reactBatch: [ParsedTranscriptMessage] = []

        func flushReact() {
            guard !reactBatch.isEmpty else { return }
            result.append(.react(reactBatch))
            reactBatch = []
        }

        for message in messages {
            if shouldUseRenderer(for: message) {
                reactBatch.append(message)
            } else {
                flushReact()
                result.append(.native(message))
            }
        }
        flushReact()
        return result
    }

    private func shouldUseRenderer(for message: ParsedTranscriptMessage) -> Bool {
        guard message.rawJSON == nil else { return false }
        guard TranscriptRendererBundle.isAvailable else { return false }
        if message.content.contains(where: requiresNativePermissionRendering) {
            return false
        }
        return message.content.allSatisfy { block in
            switch block.type {
            case "text", "thinking", "tool_call", "tool_result":
                true
            default:
                false
            }
        }
    }

    private func requiresNativePermissionRendering(_ block: TranscriptContentBlock) -> Bool {
        PermissionCardPresentationBuilder.fromCommittedPermissionBlock(block) != nil
            || PermissionResolvedReceiptBuilder.fromCommittedBlock(block) != nil
    }
}

private struct TranscriptRendererBatchHost: View {
    let conversationId: String
    let messages: [ParsedTranscriptMessage]
    var firstMessagePreviousRole: String?
    @State private var contentHeight: CGFloat = 120
    @State private var useNativeFallback = false

    var body: some View {
        Group {
            if useNativeFallback || !TranscriptRendererBundle.isAvailable {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(messages.enumerated()), id: \.element.id) { index, message in
                        MessageRow(
                            conversationId: conversationId,
                            message: message,
                            previousRole: index > 0
                                ? messages[index - 1].role
                                : firstMessagePreviousRole,
                            allMessages: messages
                        )
                    }
                }
            } else {
                TranscriptRendererHost(
                    messagesJSON: rendererJSON,
                    contentHeight: $contentHeight,
                    onUnavailable: {
                        useNativeFallback = true
                    }
                )
                .frame(minHeight: 48, idealHeight: contentHeight, maxHeight: contentHeight)
                .focusEffectDisabled()
                .accessibilityLabel("Transcript messages")
            }
        }
    }

    private var rendererJSON: String {
        TranscriptRendererPayload.encode(messages: messages)
    }
}

enum TranscriptRendererPayload {
    static func encode(messages: [ParsedTranscriptMessage]) -> String {
        let payload = messages.map { message -> [String: Any] in
            [
                "id": message.id,
                "role": message.role,
                "harness_id": message.harnessId as Any,
                "content": renderBlocks(from: message.content),
            ]
        }
        guard let data = try? JSONSerialization.data(withJSONObject: payload),
              let json = String(data: data, encoding: .utf8)
        else {
            return "[]"
        }
        return json
    }

    private static func renderBlocks(from content: [TranscriptContentBlock]) -> [[String: Any]] {
        var resultIds = Set<String>()
        for block in content where block.type == "tool_result" {
            if let callId = block.callId {
                resultIds.insert(callId)
            }
        }

        return content.compactMap { block -> [String: Any]? in
            if block.type == "tool_call" {
                if !block.hasToolCallBody, let callId = block.callId, resultIds.contains(callId) {
                    return nil
                }
                if !block.hasToolCallBody {
                    return nil
                }
            }

            var dict: [String: Any] = [
                "type": block.type,
                "text": normalizedRendererText(for: block) as Any,
                "name": block.name as Any,
                "call_id": block.callId as Any,
            ]

            if let input = block.input, block.hasToolCallBody {
                dict["input"] = input.toJSONString() ?? input.description
            }

            if block.type == "tool_result" {
                if let title = block.toolResultDisplayTitle {
                    let status = block.output?.string(at: "status") ?? "completed"
                    dict["name"] = "\(title) \(status.replacingOccurrences(of: "_", with: " "))"
                }
                if let preview = block.toolResultPreviewText {
                    dict["text"] = preview
                } else if let output = block.output {
                    dict["text"] = output.truncatedDescription
                }
                dict["status"] = block.output?.string(at: "status") ?? "completed"
            }

            if block.type == "tool_call" {
                dict["status"] = "started"
            }

            return dict
        }
    }

    private static func normalizedRendererText(for block: TranscriptContentBlock) -> String? {
        switch block.type {
        case "text", "thinking":
            guard let text = block.text else { return nil }
            return normalizedTranscriptMarkdown(text)
        default:
            return block.text
        }
    }
}
