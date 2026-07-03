import Foundation
import Security
@testable import Tamtri
import XCTest

final class KeychainCredentialStoreTests: XCTestCase {
    func testSaveAndLoadRoundTrip() throws {
        let ref = "tamtri-test-\(UUID().uuidString)"
        defer { delete(ref: ref) }

        try KeychainCredentialStore.save(value: "hello", for: ref)
        XCTAssertEqual(KeychainCredentialStore.load(for: ref), "hello")

        try KeychainCredentialStore.save(value: "goodbye", for: ref)
        XCTAssertEqual(KeychainCredentialStore.load(for: ref), "goodbye")
    }

    func testOAuthTokenStoreRoundTrip() throws {
        let ref = "tamtri-oauth-test-\(UUID().uuidString)"
        defer { delete(ref: ref) }

        try OAuthTokenStore.save(bundleJSON: #"{"access_token":"x"}"#, for: ref)
        XCTAssertEqual(OAuthTokenStore.load(for: ref), #"{"access_token":"x"}"#)
    }

    func testMissingCredentialLoadReturnsNil() {
        let ref = "tamtri-missing-\(UUID().uuidString)"
        XCTAssertNil(KeychainCredentialStore.load(for: ref))
        XCTAssertNil(OAuthTokenStore.load(for: ref))
    }

    func testDeletedCredentialMissingPath() throws {
        let ref = "tamtri-delete-\(UUID().uuidString)"
        try KeychainCredentialStore.save(value: "temporary", for: ref)
        XCTAssertEqual(KeychainCredentialStore.load(for: ref), "temporary")
        delete(ref: ref)
        XCTAssertNil(KeychainCredentialStore.load(for: ref))
    }

    /// Mirrors `AppStore.reloadGatewayServers` keychain preload into core memory.
    func testKeychainPreloadRoundTripMatchesReloadPath() throws {
        let ref = "tamtri-reload-test-\(UUID().uuidString)"
        defer { delete(ref: ref) }

        try KeychainCredentialStore.save(value: "gateway-token", for: ref)
        let loaded = KeychainCredentialStore.load(for: ref)
        XCTAssertEqual(loaded, "gateway-token")
        try KeychainCredentialStore.save(value: "rotated-token", for: ref)
        XCTAssertEqual(KeychainCredentialStore.load(for: ref), "rotated-token")
    }

    func testOAuthRefreshSuccessUpdatesKeychain() throws {
        let ref = "tamtri-oauth-refresh-\(UUID().uuidString)"
        defer { delete(ref: ref) }

        let stale = """
        {"access_token":"stale-access","refresh_token":"refresh-ok","expires_at":0,"reauth_required":false}
        """
        try OAuthTokenStore.save(bundleJSON: stale, for: ref)

        let refreshed = """
        {"access_token":"access-new","refresh_token":"refresh-ok","expires_at":9999999999,"reauth_required":false}
        """
        try OAuthTokenStore.save(bundleJSON: refreshed, for: ref)

        let loaded = try XCTUnwrap(OAuthTokenStore.load(for: ref))
        XCTAssertTrue(loaded.contains("access-new"))
        XCTAssertFalse(loaded.contains("stale-access"))
    }

    func testCredentialRefNotLeakedInGatewayServerDescription() {
        let server = GatewayServerRecord(
            id: "remote",
            displayName: "Remote",
            enabled: true,
            scope: "user",
            transport: "streamable_http",
            stdioCommand: "",
            stdioArgs: [],
            stdioEnv: [],
            httpEndpoint: "https://api.example.com/mcp",
            credentialRefs: ["keychain://api-key"],
            missingCredentialRefs: [],
            oauthStatus: "connected",
            oauthTokenRef: "keychain://remote-oauth",
            oauthClientId: "client-id",
            oauthAuthorizationEndpoint: "https://auth.example.com/authorize",
            oauthTokenEndpoint: "https://auth.example.com/token",
            oauthScopes: ["mcp"],
            capTools: "supported",
            capResources: "unknown",
            capPrompts: "unknown",
            capElicitation: "supported",
            capApps: "unknown",
            capTasks: "unknown",
            capRoots: "unknown",
            capSampling: "declined",
            connectionStatus: "connected",
            lastError: "",
            timeoutSecs: 30
        )
        let description = String(describing: server)
        XCTAssertTrue(description.contains("keychain://remote-oauth"))
        XCTAssertFalse(description.contains("super-secret-token"))
        XCTAssertFalse(description.contains("access-new"))
    }

    func testBindingClientCredentialPersistsToKeychainAndCore() async throws {
        let vaultURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("tamtri-kc-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: vaultURL, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: vaultURL) }

        guard let mockMcp = Self.mockMcpServerPath() else {
            throw XCTSkip("mock-mcp-server not built; run cargo build first")
        }

        let credentialRef = "keychain://swift-ffi-\(UUID().uuidString)"
        defer { delete(ref: credentialRef) }

        let configJSON = """
        {
          "gateway": {
            "default_call_timeout_secs": 300,
            "servers": [{
              "id": "mock",
              "display_name": "Mock MCP",
              "enabled": true,
              "scope": "user",
              "transport": {"type": "stdio", "command": "\(mockMcp)", "args": [], "env": []},
              "credentials": [{
                "credential_ref": "\(credentialRef)",
                "target": {"type": "env_var", "name": "MOCK_TOKEN"}
              }]
            }]
          }
        }
        """
        try configJSON.write(
            to: vaultURL.appendingPathComponent("config.json"),
            atomically: true,
            encoding: .utf8
        )

        let client = try TamtriBindingClient(vaultPath: vaultURL.path)
        var servers = try await client.listGatewayServers()
        XCTAssertEqual(servers[0].missingCredentialRefs, [credentialRef])

        try await client.setGatewayCredential(credentialRef: credentialRef, value: "ffi-secret-value")
        XCTAssertEqual(KeychainCredentialStore.load(for: credentialRef), "ffi-secret-value")
        let exported = try await client.exportGatewayCredential(credentialRef: credentialRef)
        XCTAssertEqual(exported, "ffi-secret-value")

        servers = try await client.listGatewayServers()
        XCTAssertTrue(servers[0].missingCredentialRefs.isEmpty)

        let relaunched = try TamtriBindingClient(vaultPath: vaultURL.path)
        var relaunchedServers = try await relaunched.listGatewayServers()
        XCTAssertEqual(relaunchedServers[0].missingCredentialRefs, [credentialRef])
        let missingAfterRelaunch = try await relaunched.exportGatewayCredential(credentialRef: credentialRef)
        XCTAssertNil(missingAfterRelaunch)

        if let stored = KeychainCredentialStore.load(for: credentialRef) {
            try await relaunched.setGatewayCredential(credentialRef: credentialRef, value: stored)
        } else {
            XCTFail("expected keychain value to survive relaunch")
        }

        relaunchedServers = try await relaunched.listGatewayServers()
        XCTAssertTrue(relaunchedServers[0].missingCredentialRefs.isEmpty)
        let reloadedExport = try await relaunched.exportGatewayCredential(credentialRef: credentialRef)
        XCTAssertEqual(reloadedExport, "ffi-secret-value")
    }

    private static func mockMcpServerPath() -> String? {
        let fileManager = FileManager.default
        let cwd = fileManager.currentDirectoryPath
        let home = fileManager.homeDirectoryForCurrentUser.path
        let candidates = [
            "\(cwd)/target/debug/mock-mcp-server",
            "\(cwd)/../target/debug/mock-mcp-server",
            "\(home)/Desktop/tamtri/target/debug/mock-mcp-server"
        ]
        return candidates.first { fileManager.isExecutableFile(atPath: $0) }
    }

    private func delete(ref: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "tamtri.gateway",
            kSecAttrAccount as String: ref
        ]
        SecItemDelete(query as CFDictionary)
    }
}
