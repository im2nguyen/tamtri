import XCTest
@testable import Tamtri

private final class NoOpConversationObserver: ConversationObserver, @unchecked Sendable {
    func onEvent(event: UiEvent) {}
}

final class GatewaySettingsTests: XCTestCase {
    func test_settings_gateway_tools_snapshot() async throws {
        let temp = FileManager.default.temporaryDirectory
            .appendingPathComponent("tamtri-gateway-tools-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: temp, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: temp) }

        guard let mockMcp = Self.mockMcpServerPath() else {
            throw XCTSkip("mock-mcp-server not built; run cargo build first")
        }

        let core = try TamtriCore(vaultPath: temp.path, observer: NoOpConversationObserver())
        try core.saveGatewayServers(servers: [
            GatewayServerDto(
                id: "mock",
                displayName: "Mock MCP",
                enabled: true,
                scope: "user",
                transport: "stdio",
                stdioCommand: mockMcp,
                stdioArgs: [],
                stdioEnv: [],
                httpEndpoint: "",
                credentialRefs: [],
                missingCredentialRefs: [],
                oauthStatus: "none",
                oauthTokenRef: "",
                oauthClientId: "",
                oauthAuthorizationEndpoint: "",
                oauthTokenEndpoint: "",
                oauthScopes: [],
                capTools: "unknown",
                capResources: "unknown",
                capPrompts: "unknown",
                capElicitation: "unknown",
                capApps: "unknown",
                capTasks: "unknown",
                capRoots: "unknown",
                capSampling: "declined",
                connectionStatus: "unknown",
                lastError: "",
                timeoutSecs: nil
            ),
            GatewayServerDto(
                id: "disabled-mock",
                displayName: "Disabled MCP",
                enabled: false,
                scope: "user",
                transport: "stdio",
                stdioCommand: mockMcp,
                stdioArgs: [],
                stdioEnv: [],
                httpEndpoint: "",
                credentialRefs: [],
                missingCredentialRefs: [],
                oauthStatus: "none",
                oauthTokenRef: "",
                oauthClientId: "",
                oauthAuthorizationEndpoint: "",
                oauthTokenEndpoint: "",
                oauthScopes: [],
                capTools: "unknown",
                capResources: "unknown",
                capPrompts: "unknown",
                capElicitation: "unknown",
                capApps: "unknown",
                capTasks: "unknown",
                capRoots: "unknown",
                capSampling: "declined",
                connectionStatus: "unknown",
                lastError: "",
                timeoutSecs: nil
            )
        ])

        let servers = try core.listGatewayServers()
        XCTAssertEqual(servers.count, 2)
        XCTAssertTrue(servers.contains(where: { $0.id == "mock" && $0.enabled }))
        XCTAssertTrue(servers.contains(where: { $0.id == "disabled-mock" && !$0.enabled }))

        let tools = try core.listGatewayTools()
        XCTAssertFalse(tools.isEmpty)
        XCTAssertTrue(tools.allSatisfy { $0.serverId == "mock" })
        XCTAssertFalse(tools.contains(where: { $0.serverId == "disabled-mock" }))
        XCTAssertTrue(tools.allSatisfy { !$0.exposedName.isEmpty && !$0.serverId.isEmpty })
        XCTAssertTrue(tools.contains(where: { $0.exposedName == "mock__echo" && $0.serverId == "mock" && $0.originalName == "echo" }))

        // Agent-native tools are not surfaced through listGatewayTools; the UI keeps them separate.
        XCTAssertTrue(
            GatewaySettingsCopy.agentNativeToolsDisclaimer.contains("not exposed by this harness yet")
        )
        XCTAssertEqual(GatewaySettingsCopy.tamtriGatewayToolsHeading, "Tamtri gateway tools")
    }

    func test_settings_agent_native_tools_disclaimer() {
        XCTAssertFalse(GatewaySettingsCopy.agentNativeToolsDisclaimer.isEmpty)
        XCTAssertTrue(
            GatewaySettingsCopy.agentNativeToolsDisclaimer.contains("Agent-native tools")
        )
        XCTAssertTrue(
            GatewaySettingsCopy.agentNativeToolsDisclaimer.contains("not exposed by this harness yet")
        )
    }

    func test_settings_tamtri_gateway_tools_heading() {
        XCTAssertEqual(GatewaySettingsCopy.tamtriGatewayToolsHeading, "Tamtri gateway tools")
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
}
