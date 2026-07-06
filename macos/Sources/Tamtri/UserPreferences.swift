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
}
