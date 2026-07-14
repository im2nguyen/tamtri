# External working directory brokering

## Default: VaultLocal

When a conversation uses `WorkingDir::VaultLocal`, the harness cwd is
`workdir/` inside the conversation's vault folder. The daemon has full access;
no brokering is required.

## External(path)

When the user points a conversation at an external folder (e.g. a git repo),
`meta.json` stores the logical path for legibility. **Filesystem access** is a
platform concern:

| Surface | Responsibility |
|---------|----------------|
| **Shell (Electron)** | Native folder picker, security-scoped bookmark / sandbox grant |
| **Daemon** | Operates on paths the user has consented to for that conversation |
| **Wire** | `roots.sync_runtime` carries resolved root URIs from shell → daemon |

The old SwiftUI shell held `NSURL` bookmark data keyed by conversation id. The
Electron shell will hold the equivalent and call `roots.sync_runtime` after the
user picks a folder.

## Rules

- Never symlink an external tree into the vault.
- On share/export, external working trees do not travel in `.tamtri` bundles unless the user explicitly opts in.
- `External(path)` bookmarks are platform-specific and never go in portable `meta.json`.

## Daemon sandbox posture

The daemon runs as a user process with the invoking user's POSIX permissions.
It is not macOS-sandboxed in V1. Electron may be sandboxed; brokering ensures
the renderer never receives raw filesystem access to paths the user did not grant.
