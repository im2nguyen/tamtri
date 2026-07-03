import AppKit
import Foundation
import SwiftUI

struct URLConsentCard: View {
    @EnvironmentObject private var store: AppStore
    let request: URLConsentRequest

    private var destination: URLHandoffDestination? {
        guard let urlString = request.url else { return nil }
        return URLHandoffPolicy.destination(for: urlString)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Browser handoff requested", systemImage: "safari")
                .font(.headline)
            if let serverId = request.serverId, !serverId.isEmpty {
                Text("Requested by \(serverId)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if let origin = request.originToolCallId, !origin.isEmpty {
                Text("Related to tool call \(origin)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Text(request.message)
                .textSelection(.enabled)
            if let destination {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Destination")
                        .font(.caption.bold())
                    Text(destination.origin)
                        .font(.body.monospaced())
                        .textSelection(.enabled)
                    Text(destination.displayURL)
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }
                .padding(8)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
            } else {
                Text("This URL cannot be opened safely.")
                    .foregroundStyle(.red)
            }
            Text("tamtri opens your default browser only after you approve. Query parameters are not stored in the audit log.")
                .font(.caption)
                .foregroundStyle(.secondary)
            HStack {
                Button("Decline") {
                    respond(action: "decline")
                }
                Button("Cancel", role: .cancel) {
                    respond(action: "cancel")
                }
                Spacer()
                Button("Open in Browser") {
                    guard let destination else { return }
                    NSWorkspace.shared.open(destination.openURL)
                    respond(action: "accept")
                }
                .keyboardShortcut(.defaultAction)
                .disabled(destination == nil)
            }
        }
        .padding(10)
        .background(.blue.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Browser handoff requested")
    }

    private func respond(action: String) {
        store.respondElicitation(requestId: request.requestId, action: action, dataJSON: nil)
    }
}

struct URLConsentRequest {
    let requestId: String
    let serverId: String?
    let originToolCallId: String?
    let message: String
    let url: String?
}

struct URLHandoffDestination: Equatable {
    let origin: String
    let displayURL: String
    let openURL: URL
}

enum URLHandoffPolicy {
    static func destination(for raw: String) -> URLHandoffDestination? {
        guard let url = URL(string: raw.trimmingCharacters(in: .whitespacesAndNewlines)),
              let host = url.host
        else {
            return nil
        }
        if url.user != nil || url.password != nil {
            return nil
        }
        let scheme = url.scheme?.lowercased() ?? ""
        let isLoopback = ["localhost", "127.0.0.1", "::1"].contains(host.lowercased())
        guard scheme == "https" || (scheme == "http" && isLoopback) else {
            return nil
        }
        var origin = "\(scheme)://\(host)"
        if let port = url.port {
            origin += ":\(port)"
        }
        return URLHandoffDestination(
            origin: origin,
            displayURL: redactedDisplay(raw),
            openURL: url
        )
    }

    static func redactedDisplay(_ raw: String) -> String {
        guard var components = URLComponents(string: raw) else {
            return raw
        }
        components.query = nil
        components.fragment = nil
        return components.string ?? raw
    }
}
