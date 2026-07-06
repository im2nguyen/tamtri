import AppKit
import Carbon.HIToolbox

@MainActor
final class GlobalHotkeyManager: @unchecked Sendable {
    private var hotKeyRef: EventHotKeyRef?
    private var eventHandler: EventHandlerRef?

    func register() {
        unregister()
        guard UserPreferences.globalHotkeyEnabled else { return }

        var eventType = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        InstallEventHandler(
            GetApplicationEventTarget(),
            { _, event, userData -> OSStatus in
                guard let userData else { return noErr }
                let manager = Unmanaged<GlobalHotkeyManager>.fromOpaque(userData).takeUnretainedValue()
                manager.focusApp()
                return noErr
            },
            1,
            &eventType,
            Unmanaged.passUnretained(self).toOpaque(),
            &eventHandler
        )

        let keyCode = UserPreferences.globalHotkeyKeyCode
        let modifiers = carbonModifiers(from: UserPreferences.globalHotkeyModifiers)
        var ref: EventHotKeyRef?
        RegisterEventHotKey(
            UInt32(keyCode),
            modifiers,
            EventHotKeyID(signature: OSType(0x544D5452), id: 1),
            GetApplicationEventTarget(),
            0,
            &ref
        )
        hotKeyRef = ref
    }

    func unregister() {
        if let hotKeyRef {
            UnregisterEventHotKey(hotKeyRef)
            self.hotKeyRef = nil
        }
        if let eventHandler {
            RemoveEventHandler(eventHandler)
            self.eventHandler = nil
        }
    }

    private func focusApp() {
        NSApp.activate(ignoringOtherApps: true)
        for window in NSApp.windows where window.canBecomeMain {
            window.makeKeyAndOrderFront(nil)
            break
        }
    }

    private func carbonModifiers(from flags: NSEvent.ModifierFlags) -> UInt32 {
        var carbon: UInt32 = 0
        if flags.contains(.command) { carbon |= UInt32(cmdKey) }
        if flags.contains(.shift) { carbon |= UInt32(shiftKey) }
        if flags.contains(.option) { carbon |= UInt32(optionKey) }
        if flags.contains(.control) { carbon |= UInt32(controlKey) }
        return carbon
    }
}
