import Foundation
import Network
import Security

final class OAuthLoopbackListener: @unchecked Sendable {
    private let port: UInt16
    private var listener: NWListener?
    private var onCallback: ((String) -> Void)?

    init(port: UInt16 = 3847) {
        self.port = port
    }

    func start(onCallback: @escaping (String) -> Void) throws {
        self.onCallback = onCallback
        let parameters = NWParameters.tcp
        guard let nwPort = NWEndpoint.Port(rawValue: port) else {
            throw NSError(domain: "tamtri.oauth", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "Invalid OAuth callback port."
            ])
        }
        let listener = try NWListener(using: parameters, on: nwPort)
        self.listener = listener
        listener.newConnectionHandler = { [weak self] connection in
            self?.handle(connection: connection)
        }
        listener.start(queue: .global(qos: .userInitiated))
    }

    func stop() {
        listener?.cancel()
        listener = nil
        onCallback = nil
    }

    private func handle(connection: NWConnection) {
        connection.start(queue: .global(qos: .userInitiated))
        connection.receive(minimumIncompleteLength: 1, maximumLength: 16_384) { [weak self] data, _, _, _ in
            defer { connection.cancel() }
            guard let self, let data, let request = String(data: data, encoding: .utf8) else {
                return
            }
            let firstLine = request.split(separator: "\r\n", maxSplits: 1).first.map(String.init) ?? ""
            let parts = firstLine.split(separator: " ")
            guard parts.count >= 2 else { return }
            let target = String(parts[1])
            guard target.hasPrefix("/callback") else { return }
            let host = "127.0.0.1"
            let callbackURL = "http://\(host):\(self.port)\(target)"
            let body = "<html><body><p>Authorization complete. You can close this tab.</p></body></html>"
            let response = """
            HTTP/1.1 200 OK\r
            Content-Type: text/html\r
            Content-Length: \(body.utf8.count)\r
            Connection: close\r
            \r
            \(body)
            """
            connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in
                DispatchQueue.main.async {
                    self.onCallback?(callbackURL)
                    self.stop()
                }
            })
        }
    }
}

struct GatewayOAuthPresentation: Equatable {
    enum IconTone: Equatable {
        case connected
        case warning
        case neutral
    }

    let iconSystemName: String
    let iconTone: IconTone
    let showsConnectButton: Bool
    let statusLabel: String

    static func forStatus(_ status: String) -> GatewayOAuthPresentation {
        let iconSystemName: String
        let iconTone: IconTone
        switch status {
        case "connected":
            iconSystemName = "checkmark.seal.fill"
            iconTone = .connected
        case "reauth_required", "expired":
            iconSystemName = "exclamationmark.triangle.fill"
            iconTone = .warning
        case "missing":
            iconSystemName = "key.slash"
            iconTone = .neutral
        case "not_configured":
            iconSystemName = "key"
            iconTone = .neutral
        default:
            iconSystemName = "key"
            iconTone = .neutral
        }
        let statusLabel: String
        switch status {
        case "connected":
            statusLabel = "Connected"
        case "missing":
            statusLabel = "Not connected"
        case "not_configured":
            statusLabel = "Not configured"
        case "expired":
            statusLabel = "Expired — reconnect"
        case "reauth_required":
            statusLabel = "Reconnect required"
        default:
            statusLabel = status.replacingOccurrences(of: "_", with: " ").capitalized
        }
        return GatewayOAuthPresentation(
            iconSystemName: iconSystemName,
            iconTone: iconTone,
            showsConnectButton: status != "connected",
            statusLabel: statusLabel
        )
    }
}

enum OAuthTokenStore {
    static func save(bundleJSON: String, for tokenRef: String) throws {
        try KeychainCredentialStore.save(value: bundleJSON, for: tokenRef)
    }

    static func load(for tokenRef: String) -> String? {
        KeychainCredentialStore.load(for: tokenRef)
    }
}

enum KeychainCredentialStore {
    #if DEBUG
    /// When set, `save` throws with this status instead of calling SecItemAdd (tests only).
    nonisolated(unsafe) static var testSaveFailureStatus: OSStatus?
    #endif

    static func save(value: String, for credentialRef: String) throws {
        #if DEBUG
        if let status = testSaveFailureStatus {
            throw NSError(domain: NSOSStatusErrorDomain, code: Int(status))
        }
        #endif
        let data = Data(value.utf8)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "tamtri.gateway",
            kSecAttrAccount as String: credentialRef
        ]
        SecItemDelete(query as CFDictionary)
        var item = query
        item[kSecValueData as String] = data
        let status = SecItemAdd(item as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw NSError(domain: NSOSStatusErrorDomain, code: Int(status))
        }
    }

    static func load(for credentialRef: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "tamtri.gateway",
            kSecAttrAccount as String: credentialRef,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let data = item as? Data else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }
}
