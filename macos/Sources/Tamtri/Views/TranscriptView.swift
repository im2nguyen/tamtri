import SwiftUI

struct TranscriptView: View {
    @EnvironmentObject private var store: AppStore

    var body: some View {
        let conversation = store.displayedConversation
        let switching = store.isSwitchingConversation

        VStack(spacing: 0) {
            ZStack(alignment: .topTrailing) {
                Group {
                    if let conversation {
                        VStack(alignment: .leading, spacing: 0) {
                            if let bookmarkState = store.missingBookmarkState {
                                DesignedErrorBannerView(
                                    state: bookmarkState,
                                    onPrimary: {
                                        store.performDesignedErrorRecovery(.repickFolder)
                                    },
                                    onDismiss: {
                                        store.dismissMissingBookmarkState()
                                    }
                                )
                                .conversationContentColumn()
                                .padding(.top, 12)
                            }
                            ScrollViewReader { proxy in
                                ScrollView {
                                    VStack(alignment: .leading, spacing: 0) {
                                        if conversation.parsedMessages.isEmpty, liveEvents.isEmpty {
                                            EmptyStateView(
                                                systemImage: "text.bubble",
                                                title: "Start the conversation",
                                                message: composerEmptyMessage,
                                                primaryActionTitle: nil,
                                                onPrimary: nil
                                            )
                                            .frame(maxWidth: .infinity)
                                            .padding(.top, TamtriSpacing.xl)
                                        } else {
                                            TranscriptRendererSection(
                                                conversationId: conversation.id,
                                                messages: conversation.parsedMessages
                                            )
                                            if !liveEvents.isEmpty {
                                                LiveTranscriptSection(events: liveEvents, conversationId: conversation.id)
                                            }
                                        }
                                    }
                                    .conversationContentColumn()
                                    .padding(.vertical, TamtriSpacing.lg)
                                    .id("transcript-bottom")
                                }
                                .defaultScrollAnchor(.bottom)
                                .accessibilityIdentifier(KeyboardHeroFlowIdentifiers.transcriptIdentifier)
                                .onChange(of: liveEvents.count) { _, _ in
                                    proxy.scrollTo("transcript-bottom", anchor: .bottom)
                                }
                            }
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                    } else if let errorState = store.designedErrorState {
                        DesignedErrorStateView(
                            state: errorState,
                            onPrimary: {
                                store.performDesignedErrorRecovery(errorState.primaryAction.recovery)
                            },
                            onSecondary: errorState.secondaryAction.map { action in
                                { store.performDesignedErrorRecovery(action.recovery) }
                            }
                        )
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    } else if switching {
                        VStack(spacing: 12) {
                            ProgressView()
                                .controlSize(.large)
                            Text("Loading conversation…")
                                .foregroundStyle(.secondary)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)

                if switching, conversation != nil {
                    ProgressView()
                        .padding(10)
                        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 8))
                        .padding()
                        .allowsHitTesting(false)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private var liveEvents: [IdentifiedCoreEvent] {
        guard let selectedId = store.selectedConversationId else {
            return []
        }
        return store.liveEvents
            .enumerated()
            .filter { $0.element.conversationId == selectedId }
            .map { IdentifiedCoreEvent(id: $0.offset, event: $0.element) }
    }

    private var composerEmptyMessage: String {
        if store.workdirFiles.contains(where: { $0.relativePath.lowercased().hasSuffix(".csv") }) {
            return "Drop a CSV here or ask your harness to turn it into a report."
        }
        return "Send a message to begin. Attach files by dropping them on the composer."
    }
}

struct LiveTranscriptSection: View {
    let events: [IdentifiedCoreEvent]
    let conversationId: String

    var body: some View {
        VStack(alignment: .leading, spacing: TamtriLayout.transcriptBlockSpacing) {
            ForEach(LiveActivityGrouping.build(from: events)) { segment in
                switch segment {
                case .activity(let cluster):
                    ActivityClusterView(cluster: cluster)
                case .toolGroup(let group):
                    if let toolEvent = group.toolEvent {
                        VStack(alignment: .leading, spacing: TamtriSpacing.sm) {
                            ToolCard(event: toolEvent)
                            ForEach(Array(group.nested.enumerated()), id: \.offset) { _, nested in
                                EventRow(event: nested)
                                    .padding(.leading, TamtriSpacing.lg)
                            }
                        }
                    }
                case .event(let event):
                    EventRow(event: event)
                }
            }
        }
        .padding(.horizontal, TamtriLayout.composerInset)
        .padding(.top, TamtriLayout.transcriptUserToAssistantGap)
        .padding(.bottom, TamtriLayout.transcriptTurnSpacing)
        .accessibilityLabel("Live transcript updates")
    }
}
