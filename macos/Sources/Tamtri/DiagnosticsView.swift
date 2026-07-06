import AppKit
import SwiftUI

enum DiagnosticsSystemInfo {
    static func currentJSON() -> String {
        let info: [String: Any] = [
            "app_version": Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.1.0",
            "app_build": Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "dev",
            "macos_version": ProcessInfo.processInfo.operatingSystemVersionString,
            "renderer_available": TranscriptRendererBundle.isAvailable,
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: info),
              let json = String(data: data, encoding: .utf8)
        else {
            return "{}"
        }
        return json
    }
}

struct DiagnosticsView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @State private var bundlePath: String?
    @State private var isGenerating = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Report an Issue")
                .font(.title2.bold())
            Text("tamtri assembles a local diagnostics zip with app, harness, gateway, and recent audit excerpts. Secrets are redacted. Nothing uploads automatically.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            if isGenerating {
                ProgressView("Building diagnostics bundle…")
            }

            if let bundlePath {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Bundle ready")
                        .font(.headline)
                    Text(bundlePath)
                        .font(.caption.monospaced())
                        .textSelection(.enabled)
                    HStack {
                        Button("Reveal in Finder") {
                            NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: bundlePath)])
                        }
                        Button("Copy Path") {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(bundlePath, forType: .string)
                        }
                    }
                }
            }

            if let errorMessage {
                Text(errorMessage)
                    .font(.caption)
                    .foregroundStyle(.red)
            }

            HStack {
                Button("Generate Bundle") {
                    Task { await generate() }
                }
                .disabled(isGenerating)
                Spacer()
                Button("Done") { dismiss() }
            }
        }
        .padding()
        .frame(width: 520)
    }

    private func generate() async {
        isGenerating = true
        errorMessage = nil
        defer { isGenerating = false }
        do {
            bundlePath = try await store.generateDiagnosticsBundle()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
