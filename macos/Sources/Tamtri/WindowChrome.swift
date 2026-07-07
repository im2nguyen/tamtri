import AppKit
import SwiftUI

enum WindowChrome {
    /// macOS titlebar band height; traffic lights are vertically centered here.
    static let titlebarHeight: CGFloat = 38

    /// Legacy alias kept for drag-region helpers.
    static let titlebarInset: CGFloat = titlebarHeight

    @MainActor
    static func apply(to window: NSWindow) {
        window.title = "tamtri"
        window.subtitle = ""
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.titlebarSeparatorStyle = .none
        window.styleMask.insert(.fullSizeContentView)
        window.isMovableByWindowBackground = true
        window.isOpaque = true
        window.backgroundColor = NSColor.windowBackgroundColor
        window.toolbar = nil
    }

    @MainActor
    static func configureApplicationWindows() {
        for window in NSApp.windows where window.canBecomeMain || window.canBecomeKey {
            apply(to: window)
        }
    }
}

/// Native translucent surface (Finder/Mail sidebar look). Traffic lights sit on it cleanly.
struct VisualEffectView: NSViewRepresentable {
    var material: NSVisualEffectView.Material = .sidebar
    /// `.withinWindow` keeps the sidebar material consistent under the traffic lights;
    /// `.behindWindow` blurs the desktop through any unpainted titlebar gap.
    var blendingMode: NSVisualEffectView.BlendingMode = .withinWindow

    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = material
        view.blendingMode = blendingMode
        view.state = .followsWindowActiveState
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {
        nsView.material = material
        nsView.blendingMode = blendingMode
        nsView.state = .followsWindowActiveState
    }
}

/// Transparent drag handle; must not paint opaque pixels over traffic lights.
final class WindowDragView: NSView {
    override var isOpaque: Bool { false }

    override func draw(_ dirtyRect: NSRect) {}

    override func mouseDown(with event: NSEvent) {
        window?.performDrag(with: event)
    }
}

private struct WindowDragRegion: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        WindowDragView(frame: .zero)
    }

    func updateNSView(_ nsView: NSView, context: Context) {}
}

/// Fills whatever container it is placed in with a transparent drag region.
struct TitlebarBand: View {
    var body: some View {
        WindowDragRegion()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .contentShape(Rectangle())
            .accessibilityHidden(true)
    }
}

private struct WindowChromeConfigurator: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        Task { @MainActor in
            if let window = view.window {
                WindowChrome.apply(to: window)
            }
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        Task { @MainActor in
            if let window = nsView.window {
                WindowChrome.apply(to: window)
            }
        }
    }
}

extension View {
    func tamtriWindowChrome() -> some View {
        background(WindowChromeConfigurator())
    }
}
