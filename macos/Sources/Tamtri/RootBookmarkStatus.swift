import Foundation

/// Shell-side bookmark presence for conversation roots (bookmarks never live in the vault).
enum RootBookmarkStatus {
    static let missingBookmarkWarning =
        "Missing access bookmark. Re-pick the folder to restore gateway access."

    static func isBookmarkMissing(kind: String, conversationId: String, rootId: String) -> Bool {
        guard kind == "filesystem" else { return false }
        return !RootBookmarkStore.hasBookmark(conversationId: conversationId, rootId: rootId)
    }

    static func missingBookmarkError(rootName: String) -> String {
        "Missing access bookmark for root \"\(rootName)\". Re-pick the folder in conversation settings."
    }
}
