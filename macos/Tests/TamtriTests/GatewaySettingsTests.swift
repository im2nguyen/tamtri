import XCTest
@testable import Tamtri

final class GatewaySettingsTests: XCTestCase {
    func test_settings_gateway_tools_snapshot() async throws {
        let client = MockCoreClient()
        let tools = try await client.listGatewayTools()
        XCTAssertFalse(tools.isEmpty)
        XCTAssertTrue(tools.allSatisfy { !$0.exposedName.isEmpty && !$0.serverId.isEmpty })
        XCTAssertEqual(tools[0].exposedName, "mock__echo")
        XCTAssertEqual(tools[0].serverId, "mock")
        XCTAssertEqual(tools[0].originalName, "echo")
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
}
