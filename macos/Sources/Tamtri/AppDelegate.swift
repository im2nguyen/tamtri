import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate {
    var onTerminate: (() -> Void)?

    func applicationDidFinishLaunching(_ notification: Notification) {
        Task { @MainActor in
            WindowChrome.configureApplicationWindows()
        }
    }

    func applicationDidBecomeActive(_ notification: Notification) {
        Task { @MainActor in
            WindowChrome.configureApplicationWindows()
        }
    }

    func windowDidBecomeKey(_ notification: Notification) {
        guard let window = notification.object as? NSWindow else { return }
        Task { @MainActor in
            WindowChrome.apply(to: window)
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        onTerminate?()
    }
}
