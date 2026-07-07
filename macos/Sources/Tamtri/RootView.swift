import SwiftUI

struct RootView: View {
    @EnvironmentObject private var store: AppStore
    @State private var sidebarWidth = CGFloat(UserPreferences.sidebarWidth)

    var body: some View {
        HStack(spacing: 0) {
            HStack(spacing: 0) {
                SidebarView()
                    .frame(width: sidebarWidth)
                PanelResizeDivider(
                    width: $sidebarWidth,
                    minWidth: TamtriLayout.sidebarMinWidth,
                    maxWidth: TamtriLayout.sidebarMaxWidth,
                    edge: .panelTrailing,
                    accessibilityLabel: "Resize sidebar"
                )
            }
            .frame(width: store.sidebarCollapsed ? 0 : sidebarWidth + PanelResizeDivider.thickness)
            .clipped()
            .allowsHitTesting(!store.sidebarCollapsed)
            .accessibilityHidden(store.sidebarCollapsed)

            MainWorkspaceView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear {
            sidebarWidth = clampedSidebarWidth(CGFloat(UserPreferences.sidebarWidth))
        }
        .onChange(of: sidebarWidth) { _, width in
            guard !store.sidebarCollapsed else { return }
            UserPreferences.sidebarWidth = Double(clampedSidebarWidth(width))
        }
        .onChange(of: store.sidebarCollapsed) { _, collapsed in
            if !collapsed {
                sidebarWidth = clampedSidebarWidth(CGFloat(UserPreferences.sidebarWidth))
            }
        }
        // Draw under the system titlebar so traffic lights share a row with our top bar.
        .ignoresSafeArea(.container, edges: .top)
        .tamtriWindowChrome()
        .sheet(isPresented: $store.showNewConversation) {
            NewConversationView()
        }
        .sheet(isPresented: $store.showSettings) {
            SettingsView()
        }
        .sheet(isPresented: $store.showForkConversation) {
            ForkConversationView()
        }
        .sheet(isPresented: $store.showHarnessHealth) {
            HarnessHealthView()
        }
        .sheet(isPresented: $store.showSearch) {
            SearchView()
        }
        .sheet(isPresented: $store.showCommandPalette) {
            CommandPaletteView()
        }
        .sheet(isPresented: $store.showDiagnostics) {
            DiagnosticsView()
        }
        .sheet(isPresented: $store.showConversationRoots) {
            if let conversation = store.displayedConversation {
                NavigationStack {
                    ConversationRootsSettingsView(conversationId: conversation.id)
                        .padding()
                        .navigationTitle("Conversation Roots")
                        .toolbar {
                            Button("Done") {
                                store.showConversationRoots = false
                            }
                        }
                }
                .frame(minWidth: 480, minHeight: 320)
            }
        }
        .alert("Tamtri", isPresented: errorPresented) {
            Button("OK") {
                store.errorMessage = nil
            }
            if store.errorMessage?.localizedCaseInsensitiveContains("unknown harness") == true
                || store.errorMessage?.localizedCaseInsensitiveContains("unknown acp agent") == true {
                Button("Open Harness Health") {
                    store.errorMessage = nil
                    store.showHarnessHealth = true
                }
            }
        } message: {
            Text(store.errorMessage ?? "")
        }
        .alert("Import complete", isPresented: importSummaryPresented) {
            Button("OK") {
                store.importSummaryMessage = nil
            }
        } message: {
            Text(store.importSummaryMessage ?? "")
        }
    }

    private func clampedSidebarWidth(_ width: CGFloat) -> CGFloat {
        min(TamtriLayout.sidebarMaxWidth, max(TamtriLayout.sidebarMinWidth, width))
    }

    private var importSummaryPresented: Binding<Bool> {
        Binding(
            get: { store.importSummaryMessage != nil },
            set: { isPresented in
                if !isPresented {
                    store.importSummaryMessage = nil
                }
            }
        )
    }

    private var errorPresented: Binding<Bool> {
        Binding(
            get: { store.errorMessage != nil },
            set: { isPresented in
                if !isPresented {
                    store.errorMessage = nil
                }
            }
        )
    }
}
