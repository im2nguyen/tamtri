# UI surfaces

tamtri's product UI lives in **`packages/app`** (Expo + React Native Web, SDK 54). **Electron** (`packages/desktop`) hosts the exported web bundle and bridges the daemon wire protocol over IPC.

## Surfaces

| Surface | How it connects | Notes |
|---------|-----------------|-------|
| **Desktop (Electron)** | IPC → main process → localhost WebSocket | Daemon spawned automatically; token never in renderer |
| **Web** | Direct WebSocket + bearer token | Use `pnpm run dev:web` so auth env is injected |
| **Mobile (Expo Go)** | LAN WebSocket to Mac daemon | `pnpm run dev:ios`; relay remote access in progress |

## Renderer boundary

Surfaces are dumb clients:

- They talk to `tamtri-daemon` via `@tamtri/client` only.
- They render transcript view models from `conversation.load` and wire `event` notifications.
- They never read the vault directly, spawn harnesses, hold gateway credentials, or bypass consent.

Every UI-initiated action goes through the same daemon RPC path as a direct tool call.

## Project shell

The normal shell is project-first:

- The left sidebar renders projects with nested conversation threads. It has no separate top-level recent-conversation list.
- Real projects stay visible when empty. The immutable `Unfiled` project appears only when it contains legacy or orphaned conversations.
- Creating or selecting a project prepares the centered new-thread composer. Project rows own rename, shared-root, and empty-project delete actions.
- Desktop content sits in a raised card beside a resizable sidebar. Compact layouts use a full-screen sidebar overlay.
- Conversation and dock toolbars are 46 px high.

Project and conversation mutations go through `@tamtri/client`; the sidebar never edits vault files.

## Transcript rendering

- Messages project to an AI-SDK-shaped view model for incremental streaming (`packages/app/src/lib/ai-sdk-bridge.ts`).
- User and assistant rows support copy, timestamp, and fork actions on hover (web).
- Thinking, tool calls/results, tasks, and App resources render as compact, expandable receipt rows. Their persisted input, output, status, and provenance remain inspectable without interrupting the narrative.
- Elicitation and permission requests render as typed action cards. Unresolved requests stay immediately above the composer and block sending.
- The transcript and composer share one centered narrative column, approximately 820 px at desktop widths.
- The transcript follows new content only while the reader remains near the bottom. A keyboard-accessible control returns to the latest message.

## Composer stack

The composer is a rounded, raised stack in the same narrative column as the transcript:

1. Attached file pills, when present.
2. A multiline prompt field. Enter sends; Shift+Enter or Alt+Enter inserts a newline.
3. An attachment menu plus harness, model, and mode controls on the left; voice placeholder and send/stop control on the right.

Dropped or selected files are copied into the conversation workdir through daemon RPC. Root attachment is capability-gated. Existing threads keep their harness and model identity; choosing another starts the fork flow unless that harness explicitly supports a runtime model switch.

## Artifacts

The UI never reads mutable files from `workdir/` for preview. Core snapshots renderable harness output into `attachments/` with SHA-256 verification, then references them from `Artifact` blocks in the transcript.

**Right dock:**

- Transcript shows compact artifact cards.
- The optional right dock (`right-dock.tsx`) derives Artifacts, Apps, and Tasks tabs from transcript content and shows only non-empty tabs.
- Artifact selection opens full preview: sandboxed HTML iframe (web), text/markdown, CSV, or images.
- App and task tabs summarize transcript content. Interactive controls stay in transcript order.
- Dock width persists through the versioned Zustand UI store. Narrow layouts use a modal dock.

**HTML sandbox (web):** verified bytes only, no scripts, deny-by-default CSP, and no external URL-bearing attributes. Artifact HTML has no network access. Blocked link attempts are audited.

## Non-HTML previews

Markdown, plain text, CSV grids, and images render in the sidebar. Unknown types show metadata and open-in-Finder where the desktop shell provides it.

## Settings navigation

Settings reuse the project shell with a dedicated grouped sidebar and search:

- The settings sidebar replaces the project tree while preserving the same compact row rhythm and shell proportions.
- **General**: density and app-wide defaults.
- **Appearance**: system/light/dark theme, fonts, and syntax preview.
- **Providers** and **Usage**: agent roster, readiness, authentication guidance, and quotas.
- **Connect host**: mobile and remote daemon connection.
- **Import bundle** and **Import sessions**: portable conversations and native agent history.
- **Diagnostics & vault**: daemon-reported vault state.

Canonical routes are `/settings/<section>`. `/settings` and invalid section slugs redirect to General; legacy `/settings/agents` redirects to Providers. Search ranks canonical setting rows, opens the owning section, and links to stable row anchors when available.

## Related

- [daemon-protocol.md](./daemon-protocol.md) — wire contract
- [vault-format.md](./vault-format.md) — attachments vs workdir
- [packages/app/README.md](../packages/app/README.md) — dev loop
