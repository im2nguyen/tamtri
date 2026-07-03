import XCTest
@testable import Tamtri

final class GatewaySettingsTests: XCTestCase {
    func test_settings_gateway_tools_snapshot() async throws {
        let client = MockCoreClient()
        let tools = try await client.listGatewayTools()
        XCTAssertFalse(tools.isEmpty)
        XCTAssertTrue(tools.allSatisfy { !$0.exposedName.isEmpty && !$0.serverId.isEmpty })
        XCTAssertEqual(tools[0].exposedName, "mock__echo")
    }
}
