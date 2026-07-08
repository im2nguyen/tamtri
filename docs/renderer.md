# UI surfaces

tamtri's product UI lives in **`packages/app`** (Expo + React Native Web). Electron (`packages/desktop`) hosts the exported web bundle and bridges the daemon wire protocol over IPC.

## Renderer boundary

Surfaces are dumb clients:

- They talk to `tamtri-daemon` via `@tamtri/client` only.
- They render transcript view models from `conversation.load` and wire events.
- They never read the vault directly, spawn harnesses, hold gateway credentials, or bypass consent.

Every UI-initiated action goes through the same daemon RPC path as a direct tool call.

## Artifact snapshots (core)

The UI never reads from `workdir/`. Core snapshots renderable `FileChanged` outputs into `attachments/`, records size and SHA-256 in the transcript `Artifact` block, and emits an `artifact_snapshotted` audit receipt.

## HTML sandbox (future in Expo)

HTML and SVG artifacts will render in a sandboxed webview with no network access, strict CSP, and verified attachment bytes before display. Milestone 5 rules still apply; the host moves from Swift `WKWebView` to a contained webview inside `@tamtri/app`.

## Non-HTML previews

Markdown, text, CSV grids, and images render as native/RN components where possible. Unknown artifacts show typed file cards with metadata.

## Related

- `docs/daemon-protocol.md` — wire contract
- `packages/app/README.md` — dev loop
