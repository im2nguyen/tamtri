import Foundation

enum AppPanelViewModel {
    static let offlineMessage = "App offline — template unavailable without an active gateway run."

    static func accessibilityValue(templateLoaded: Bool) -> String {
        templateLoaded ? "loaded" : "offline"
    }

    static func accessibilityLabel(title: String) -> String {
        "MCP App \(title)"
    }
}

enum AppBridgeConsentViewModel {
    static let headline = "App action needs consent"

    static func serverAttribution(serverId: String, appId: String) -> String {
        "Server: \(serverId) · App: \(appId)"
    }
}

enum TaskLiveCardViewModel {
    static func statusIcon(for status: String) -> String {
        switch status {
        case "completed": "checkmark.circle"
        case "failed": "xmark.circle"
        default: "clock"
        }
    }

    static func accessibilityStatus(for state: LiveTaskState) -> String {
        [state.status, state.progressMessage].compactMap { $0 }.joined(separator: ", ")
    }

    static func showsCancelButton(for state: LiveTaskState) -> Bool {
        !state.isTerminal
    }
}

enum TaskRefCardViewModel {
    static func title(block: TranscriptContentBlock) -> String {
        block.taskTitle ?? block.taskId ?? "Task"
    }

    static func accessibilityValue(status: String?, resultSummary: String?) -> String {
        [status, resultSummary].compactMap { $0 }.joined(separator: ", ")
    }
}

enum RootRowViewModel {
    static func showsRepickButton(bookmarkMissing: Bool) -> Bool {
        bookmarkMissing
    }

    static func warningMessage(bookmarkMissing: Bool) -> String? {
        bookmarkMissing ? RootBookmarkStatus.missingBookmarkWarning : nil
    }

    static func accessibilityLabel(rootName: String, bookmarkMissing: Bool) -> String {
        bookmarkMissing ? "Missing bookmark for root \(rootName)" : rootName
    }
}

enum CapabilityBadgeViewModel {
    static func effectiveStatus(title: String, status: String) -> String {
        title == "Sampling" ? "declined" : status
    }

    static func accessibilityText(title: String, status: String) -> String {
        if title == "Sampling" {
            "Sampling declined by design. The model lives in the harness."
        } else {
            "\(title) \(effectiveStatus(title: title, status: status).replacingOccurrences(of: "_", with: " "))"
        }
    }
}
