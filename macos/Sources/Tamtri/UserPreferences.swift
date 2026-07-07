import AppKit
import Carbon.HIToolbox
import Foundation

enum UserPreferences {
    private nonisolated(unsafe) static let defaults = UserDefaults.standard

    static var globalHotkeyEnabled: Bool {
        get { defaults.object(forKey: "tamtri.globalHotkeyEnabled") as? Bool ?? true }
        set { defaults.set(newValue, forKey: "tamtri.globalHotkeyEnabled") }
    }

    static var globalHotkeyKeyCode: UInt16 {
        get {
            let stored = defaults.integer(forKey: "tamtri.globalHotkeyKeyCode")
            return stored == 0 ? UInt16(kVK_Space) : UInt16(stored)
        }
        set { defaults.set(Int(newValue), forKey: "tamtri.globalHotkeyKeyCode") }
    }

    static var globalHotkeyModifiers: NSEvent.ModifierFlags {
        get {
            let raw = defaults.integer(forKey: "tamtri.globalHotkeyModifiers")
            if raw == 0 {
                return [.command, .shift]
            }
            return NSEvent.ModifierFlags(rawValue: UInt(raw))
        }
        set { defaults.set(Int(newValue.rawValue), forKey: "tamtri.globalHotkeyModifiers") }
    }

    static var coldStartBudgetMs: Int { 2500 }

    static var workspaceRailCollapsed: Bool {
        get { defaults.object(forKey: "tamtri.workspaceRailCollapsed") as? Bool ?? true }
        set { defaults.set(newValue, forKey: "tamtri.workspaceRailCollapsed") }
    }

    static var workspaceRailWidth: Double {
        get {
            let stored = defaults.double(forKey: "tamtri.workspaceRailWidth")
            return stored > 0 ? stored : Double(TamtriLayout.filesBrowseRailIdealWidth)
        }
        set { defaults.set(newValue, forKey: "tamtri.workspaceRailWidth") }
    }

    static var workspacePreviewRailWidth: Double {
        get {
            let stored = defaults.double(forKey: "tamtri.workspacePreviewRailWidth")
            return stored > 0 ? stored : Double(TamtriLayout.filesPreviewRailIdealWidth)
        }
        set { defaults.set(newValue, forKey: "tamtri.workspacePreviewRailWidth") }
    }

    static var sidebarCollapsed: Bool {
        get { defaults.object(forKey: "tamtri.sidebarCollapsed") as? Bool ?? false }
        set { defaults.set(newValue, forKey: "tamtri.sidebarCollapsed") }
    }

    static var sidebarWidth: Double {
        get {
            let stored = defaults.double(forKey: "tamtri.sidebarWidth")
            return stored > 0 ? stored : Double(TamtriLayout.sidebarIdealWidth)
        }
        set { defaults.set(newValue, forKey: "tamtri.sidebarWidth") }
    }
}
