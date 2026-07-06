import Foundation
@testable import Tamtri
import XCTest

final class Milestone8AccessibilityTests: XCTestCase {
    func testKeyboardOnlyHeroFlowIdentifiers() {
        XCTAssertEqual(KeyboardHeroFlowCopy.sidebarIdentifier, "tamtri.sidebar")
        XCTAssertEqual(KeyboardHeroFlowCopy.composerIdentifier, "tamtri.composer")
        XCTAssertEqual(KeyboardHeroFlowCopy.transcriptIdentifier, "tamtri.transcript")
    }

    func testVoiceoverCardLabels() {
        let issue = VaultIssueRecord(
            kind: "duplicate_id",
            conversationId: "abc",
            path: nil,
            reason: nil,
            winnerPath: "/winner",
            loserPaths: ["/loser"],
            detail: "Duplicate conversation id abc"
        )
        XCTAssertFalse(issue.detail.isEmpty)

        let oauth = GatewayOAuthPresentation.forStatus("missing")
        XCTAssertEqual(oauth.statusLabel, "Not connected")

        let artifactActions = FileRowActionsPresentation.artifact(canPreviewInline: true)
        XCTAssertEqual(artifactActions.primaryAction?.label, "Preview")
    }

    @MainActor
    func testCommandPaletteActionsRoute() async {
        let store = AppStore(core: MockCoreClient())
        store.showCommandPalette = true
        store.performDesignedErrorRecovery(.openHarnessHealth)
        XCTAssertTrue(store.showHarnessHealth)
        store.showHarnessHealth = false
        store.performDesignedErrorRecovery(.forkConversation)
        XCTAssertTrue(store.showForkConversation)
    }

    func testColdStartBudgetSmoke() {
        XCTAssertEqual(UserPreferences.coldStartBudgetMs, 2500)
    }

    func testOAuthHumanReadableLabels() {
        XCTAssertEqual(GatewayOAuthPresentation.forStatus("not_configured").statusLabel, "Not configured")
        XCTAssertEqual(GatewayOAuthPresentation.forStatus("reauth_required").statusLabel, "Reconnect required")
    }

    func testVaultIssueRecordIdentity() {
        let issue = VaultIssueRecord(
            kind: "unreadable_folder",
            conversationId: nil,
            path: "/tmp/bad",
            reason: "bad meta",
            winnerPath: nil,
            loserPaths: [],
            detail: "Unreadable"
        )
        XCTAssertEqual(issue.id, "unreadable_folder-/tmp/bad")
    }
}
