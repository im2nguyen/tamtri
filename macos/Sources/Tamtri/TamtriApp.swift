import AppKit
import SwiftUI

@main
struct TamtriApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var store = AppStore(core: makeDefaultCoreClient())

    init() {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(store)
                .task {
                    await store.refresh()
                    await store.refreshGatewayServers()
                    await store.refreshHarnessAgents()
                    await store.evaluateFirstRunHarnessHealth()
                }
                .onAppear {
                    appDelegate.onTerminate = {
                        store.prepareForAppQuitSync()
                    }
                }
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Conversation") {
                    store.showNewConversation = true
                }
                .keyboardShortcut("n", modifiers: [.command])
            }
            CommandGroup(after: .toolbar) {
                Button("Search Conversations") {
                    store.showSearch = true
                }
                .keyboardShortcut("f", modifiers: [.command])
                Button("Harness Health") {
                    store.showHarnessHealth = true
                }
            }
        }
    }
}
