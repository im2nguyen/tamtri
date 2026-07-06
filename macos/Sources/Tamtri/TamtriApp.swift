import AppKit
import SwiftUI

@main
struct TamtriApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var store = AppStore(core: makeDefaultCoreClient())
    @State private var hotkeyManager = GlobalHotkeyManager()

    init() {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(store)
                .task {
                    let started = ContinuousClock.now
                    await store.refresh()
                    await store.refreshGatewayServers()
                    await store.refreshHarnessAgents()
                    await store.evaluateFirstRunHarnessHealth()
                    let elapsed = started.duration(to: ContinuousClock.now)
                    let ms = Int(elapsed.components.seconds * 1000 + elapsed.components.attoseconds / 1_000_000_000_000_000)
                    store.recordColdStart(elapsedMs: ms)
                }
                .onAppear {
                    appDelegate.onTerminate = {
                        store.prepareForAppQuitSync()
                    }
                    hotkeyManager.register()
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
                Button("Command Palette") {
                    store.showCommandPalette = true
                }
                .keyboardShortcut("k", modifiers: [.command])
                Button("Search Conversations") {
                    store.showSearch = true
                }
                .keyboardShortcut("f", modifiers: [.command])
                Button("Harness Health") {
                    store.showHarnessHealth = true
                }
                Button("Report an Issue…") {
                    store.showDiagnostics = true
                }
            }
        }

        MenuBarExtra("tamtri", systemImage: "bubble.left.and.bubble.right") {
            Button("Show Tamtri") {
                NSApp.activate(ignoringOtherApps: true)
                NSApp.windows.first?.makeKeyAndOrderFront(nil)
            }
            Button("New Conversation") {
                store.showNewConversation = true
                NSApp.activate(ignoringOtherApps: true)
            }
            Divider()
            Button("Quit") {
                NSApp.terminate(nil)
            }
        }
    }
}
