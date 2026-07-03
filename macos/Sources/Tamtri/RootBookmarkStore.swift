import Foundation

enum RootBookmarkStore {
    private static var baseDirectory: URL {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        return appSupport
            .appendingPathComponent("tamtri", isDirectory: true)
            .appendingPathComponent("root-bookmarks", isDirectory: true)
    }

    static func bookmarkURL(conversationId: String, rootId: String) -> URL {
        baseDirectory
            .appendingPathComponent(conversationId, isDirectory: true)
            .appendingPathComponent("\(rootId).bookmark")
    }

    static func hasBookmark(conversationId: String, rootId: String) -> Bool {
        FileManager.default.fileExists(atPath: bookmarkURL(conversationId: conversationId, rootId: rootId).path)
    }

    static func saveBookmark(data: Data, conversationId: String, rootId: String) throws {
        let url = bookmarkURL(conversationId: conversationId, rootId: rootId)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try data.write(to: url, options: .atomic)
    }

    static func deleteBookmark(conversationId: String, rootId: String) throws {
        let url = bookmarkURL(conversationId: conversationId, rootId: rootId)
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
    }

    static func resolveURL(conversationId: String, rootId: String) throws -> URL {
        let url = bookmarkURL(conversationId: conversationId, rootId: rootId)
        let data = try Data(contentsOf: url)
        var isStale = false
        return try URL(
            resolvingBookmarkData: data,
            options: [.withSecurityScope],
            relativeTo: nil,
            bookmarkDataIsStale: &isStale
        )
    }
}
