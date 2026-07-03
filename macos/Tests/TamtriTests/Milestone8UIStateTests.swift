import Foundation
@testable import Tamtri
import XCTest

final class Milestone8UIStateTests: XCTestCase {
    func testSearchScopeMessageNamesTitlesTextAndThinking() async {
        let client = MockCoreClient()
        let message = await client.searchScopeMessage()
        XCTAssertTrue(message.localizedCaseInsensitiveContains("title"))
        XCTAssertTrue(message.localizedCaseInsensitiveContains("Text"))
        XCTAssertTrue(message.localizedCaseInsensitiveContains("Thinking"))
    }

    func testHarnessHealthMockReportsReadyAgent() async throws {
        let client = MockCoreClient()
        let entries = try await client.listHarnessHealth()
        XCTAssertFalse(entries.isEmpty)
        XCTAssertTrue(entries.contains { $0.status == "ready" })
    }

    func testHarnessHealthChecklistIsCopyableText() async throws {
        let client = MockCoreClient()
        let checklist = try await client.harnessHealthChecklist()
        XCTAssertTrue(checklist.contains("tamtri harness setup checklist"))
    }

    func testArtifactPreviewPrimaryOpenSecondary() {
        let actions = FileRowActionsPresentation.artifact(canPreviewInline: true)
        XCTAssertEqual(actions.primaryAction?.label, "Preview")
        XCTAssertEqual(actions.primaryAction?.role, .previewPrimary)
        XCTAssertEqual(actions.secondaryAction?.label, "Open")
        XCTAssertEqual(actions.secondaryAction?.role, .openSecondary)

        let loaded = FileRowActionsPresentation.artifact(canPreviewInline: true, previewLoaded: true)
        XCTAssertEqual(loaded.primaryAction?.label, "Previewing")
        XCTAssertFalse(loaded.primaryAction?.isEnabled ?? true)
        XCTAssertEqual(loaded.secondaryAction?.label, "Open")
    }

    func testWorkdirPreviewActionsPreferPreviewWhenSupported() {
        let actions = FileRowActionsPresentation.workdir(canPreviewInline: true)
        XCTAssertEqual(actions.primaryAction?.label, "Preview")
        XCTAssertEqual(actions.secondaryAction?.label, "Open")

        let binaryOnly = FileRowActionsPresentation.workdir(canPreviewInline: false)
        XCTAssertNil(binaryOnly.preview)
        XCTAssertEqual(binaryOnly.primaryAction?.label, "Open")
    }

    func testFilesPanelCopyDistinguishesZones() {
        XCTAssertTrue(FilesPanelCopy.artifactsSectionTitle.localizedCaseInsensitiveContains("frozen"))
        XCTAssertTrue(FilesPanelCopy.workdirSectionSubtitle.localizedCaseInsensitiveContains("live"))
        XCTAssertNotEqual(FilesPanelCopy.liveWorkingFileBadge, FilesPanelCopy.frozenAttachmentBadge)
    }

    func testEmptyVaultState() {
        let state = TamtriErrorClassifier.emptyVaultState()
        XCTAssertEqual(state.kind, .emptyVault)
        XCTAssertEqual(state.primaryAction.label, "New Conversation")
        XCTAssertEqual(state.primaryAction.recovery, .newConversation)
        XCTAssertFalse(state.accessibilityLabel.isEmpty)
    }

    func testMalformedConversationState() {
        let state = TamtriErrorClassifier.malformedConversation(
            message: "malformed vault: bad line",
            conversationId: "abc"
        )
        XCTAssertEqual(state.kind, .malformedConversation)
        XCTAssertEqual(state.primaryAction.label, "Reveal in Finder")
        XCTAssertEqual(state.primaryAction.recovery, .revealInFinder(conversationId: "abc"))
        XCTAssertEqual(state.detail, "malformed vault: bad line")
    }

    func testBusyConversationState() {
        let state = TamtriErrorClassifier.busyConversation(conversationId: "busy-1")
        XCTAssertEqual(state.kind, .busyConversation)
        XCTAssertEqual(state.primaryAction.label, "Cancel Run")
        XCTAssertEqual(state.secondaryAction?.label, "Wait")
    }

    func testMissingBookmarkState() {
        let state = TamtriErrorClassifier.missingBookmark(rootName: "Reports")
        XCTAssertEqual(state.kind, .missingBookmark)
        XCTAssertEqual(state.primaryAction.label, "Re-pick Folder")
        XCTAssertTrue(state.message.contains("Re-pick"))
    }

    func testUnsupportedSchemaState() {
        let state = TamtriErrorClassifier.unsupportedSchema(
            message: "unsupported schema version: 999",
            conversationId: "future"
        )
        XCTAssertEqual(state.kind, .unsupportedSchema)
        XCTAssertEqual(state.primaryAction.recovery, .revealInFinder(conversationId: "future"))
    }

    func testUnavailableHarnessState() {
        let state = TamtriErrorClassifier.unavailableHarness(
            message: "unknown harness: missing-agent",
            harnessId: "missing-agent"
        )
        XCTAssertEqual(state.kind, .unavailableHarness)
        XCTAssertEqual(state.primaryAction.label, "Open Harness Health")
        XCTAssertEqual(state.secondaryAction?.label, "Fork Into…")
    }

    func testTamtriErrorClassifierParsesCoreMessages() {
        let classified = TamtriErrorClassifier.classify(
            message: "conversation is being written by another process: abc",
            conversationId: "abc"
        )
        XCTAssertEqual(classified?.kind, .busyConversation)
    }

    @MainActor
    func testAppStoreRoutesDesignedRecoveryActions() async {
        let store = AppStore(core: MockCoreClient())
        store.designedErrorState = TamtriErrorClassifier.emptyVaultState()
        store.performDesignedErrorRecovery(.newConversation)
        XCTAssertNil(store.designedErrorState)
        XCTAssertTrue(store.showNewConversation)

        store.performDesignedErrorRecovery(.openHarnessHealth)
        XCTAssertTrue(store.showHarnessHealth)
    }
}
