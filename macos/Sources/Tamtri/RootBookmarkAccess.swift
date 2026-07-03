import Foundation

/// Holds security-scoped access for conversation roots for the duration of a harness run.
@MainActor
enum RootBookmarkAccess {
    private struct Session {
        var urls: [URL] = []
    }

    private static var sessions: [String: Session] = [:]

    /// Resolves bookmarks, starts security-scoped access, and returns roots with accessible URIs for the core gateway.
    static func beginAccess(conversationId: String, roots: [RootRecord]) throws -> [RootDto] {
        endAccess(conversationId: conversationId)
        var session = Session()
        var dtos: [RootDto] = []

        for root in roots {
            guard root.kind == "filesystem" else {
                dtos.append(root.dto)
                continue
            }
            guard !root.bookmarkMissing else {
                continue
            }
            let url = try RootBookmarkStore.resolveURL(conversationId: conversationId, rootId: root.id)
            guard url.startAccessingSecurityScopedResource() else {
                throw RootBookmarkAccessError.accessDenied(root.name)
            }
            session.urls.append(url)
            dtos.append(
                RootDto(
                    id: root.id,
                    name: root.name,
                    uri: url.isFileURL ? url.absoluteString : url.path,
                    kind: root.kind,
                    scope: root.scope
                )
            )
        }

        sessions[conversationId] = session
        return dtos
    }

    static func endAccess(conversationId: String) {
        guard let session = sessions.removeValue(forKey: conversationId) else { return }
        for url in session.urls {
            url.stopAccessingSecurityScopedResource()
        }
    }
}

enum RootBookmarkAccessError: LocalizedError {
    case accessDenied(String)

    var errorDescription: String? {
        switch self {
        case .accessDenied(let name):
            "Could not access root folder \"\(name)\". Re-pick the folder in conversation settings."
        }
    }
}

private extension RootRecord {
    var dto: RootDto {
        RootDto(id: id, name: name, uri: uri, kind: kind, scope: scope)
    }
}
