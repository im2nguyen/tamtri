# Milestone 5: Rendering Plane + Artifact Hero

Fifth build session. This is the first "show, don't tell" milestone: a harness creates a report file and tamtri renders it inline. The same sandboxed rendering infrastructure later hosts MCP Apps, but this milestone is about artifacts produced by the harness, especially the report-from-data hero. This is also the natural milestone to introduce the React/TypeScript renderer island if the transcript/cards are moving to WebKit.

Security is the shape of the milestone. Harness-produced HTML is model-generated code, so it never gets network access and never talks to the host except through audited, consent-gated paths added later for Apps. The transcript renders a frozen snapshot from `attachments/`, not a mutable file in `workdir/`.

## Definition of done

- A `FileChanged` event for a renderable file snapshots bytes from the conversation working directory into `attachments/`, records size and SHA-256, and appends an `Artifact` content block.
- `messages.jsonl` plus `attachments/` can redraw rendered artifacts after quit and relaunch. The live `workdir/` file is not needed for replay.
- HTML artifacts render inline in a sandboxed `WKWebView` with zero network access. Any attempted `http`, `https`, or external `file` navigation is blocked and logged.
- Markdown, CSV, and images render with native or sandboxed previews. Unknown files show a typed file card with open-in-Finder/reveal actions.
- If a React renderer island is used, it renders only from Swift-provided view models and cannot read attachment files directly except through explicit, sandboxed URLs Swift grants for that artifact.
- Artifact integrity is enforced: an artifact block with a path outside `attachments/`, size mismatch, or hash mismatch never renders active content.
- Drag a CSV into the composer, ask an ACP agent to create `report.html`, and see the report inline in the conversation. This is the launch clip.
- Accessible fallbacks exist for every artifact card: title, type, size, integrity status, and reveal/open actions are keyboard and VoiceOver reachable.
- `/docs/vault-format.md` and the renderer docs describe the snapshot boundary, sandbox policy, MIME handling, and integrity behavior.
- Hermetic core tests and a Swift/UI smoke path cover artifact snapshotting, reload, and network blocking. `cargo test` and `cargo clippy` are clean for core work.

## Implementation checkpoints and gaps

Current repo status:

- `core/src/artifact.rs` snapshots renderable `FileChanged` paths from the conversation workdir into hash-prefixed files under `attachments/`, validates the resulting `Artifact` block, and emits `artifact_snapshotted` receipts (including `tool_call_id` when the snapshot came from an explicit `FileChanged`).
- `verify_inline_artifact` rejects inline transcript bytes whose size or SHA-256 do not match the `Artifact` block before any renderer uses them.
- `TamtriCore` appends snapshot artifact blocks to the committed assistant message before writing `messages.jsonl`, so reload redraws from transcript state instead of the mutable workdir file. It snapshots explicit `FileChanged` paths first, then any remaining `referenced_paths` collected by the turn reducer (paths from `FileChanged`, tool-result diffs, and write/edit tool inputs). There is no full `workdir/` directory scan at end-of-turn.
- The turn reducer tracks `referenced_paths` from harness events so incidental files in `workdir/` (for example a dropped input CSV) are not snapshotted unless the harness actually touched them.
- The mock ACP agent writes deterministic `workdir/report.html`; the facade integration test verifies an artifact block appears, remains stable after the workdir file changes, and that `artifact_snapshotted` includes `tool_call_id`.
- Swift renders inline transcript `Artifact` blocks via `ArtifactCard`: HTML/SVG in a no-network `WKWebView` with strict CSP, blocked navigation logged to `events.jsonl` as `artifact_navigation_blocked`. CSV/TSV, markdown, images, and plain text preview inline after integrity verification. The Files inspector (right panel) still lists live `workdir/` files for browsing and reveal/open actions.
- Drag-and-drop into the composer copies files into the selected conversation workdir and inserts `Attached: <filename>` references into the draft prompt.
- `list_workdir_files` returns `modified_at` (Unix mtime); the Files panel auto-selects the newest file when a turn ends.
- Swift renderer policy tests cover CSP injection, external navigation blocking, MIME routing, and markdown preview sanitization.
- Core hermetic test verifies `log_artifact_navigation_blocked` appends an `artifact_navigation_blocked` receipt to `events.jsonl`.
- `/docs/vault-format.md`, `/docs/events-format.md`, and `/docs/renderer.md` describe the snapshot boundary, audit receipts, MIME handling, and sandbox policy.

Remaining M5 gaps (defer to M8 unless noted):

- Full accessibility pass on artifact/file surfaces (M8).
- React renderer island not built (optional per M5; native SwiftUI suffices for V1).
- Deeper renderer UI automation can still be added later, but the V1 policy surface is covered by tests and the app build.

## Architecture note: snapshot first, render second

There are two file zones:

```text
workdir/       harness working files, mutable, local
attachments/   curated snapshots, content-hashed, portable
```

Never render directly from `workdir/`. When the reducer sees a completed `FileChanged` that points to a renderable artifact, the core snapshots bytes into `attachments/`, validates the resulting `ContentBlock::Artifact`, and only then tells the UI to render it. This keeps old conversations stable even if the harness later overwrites the same filename.

## Task 1: Artifact snapshot service

Fill the M3 reducer hook for `FileChanged`.

Suggested shape:

```rust
pub struct ArtifactSnapshotter {
    conversation_paths: ConversationPaths,
    detector: ArtifactDetector,
}

impl ArtifactSnapshotter {
    pub async fn snapshot_file_changed(&self, event: &FileChanged) -> Result<Option<ArtifactSnapshot>>;
}

pub struct ArtifactSnapshot {
    pub block: ContentBlock,
    pub original_path: PathBuf,
    pub attachment_path: String,
    pub mime_type: String,
    pub size: u64,
    pub sha256: String,
}
```

Rules:

- Resolve `FileChanged.path` relative to the conversation working directory unless it is already a permitted external working-dir path.
- Reject paths outside the consent-scoped working directory.
- Wait for the file write to finish before reading. For ACP tool events this should happen after the write/edit tool completes, not on every diff hunk.
- Name attachment files predictably: `attachments/<sha256-prefix>-<safe-basename>`. Collisions are harmless if the full hash and bytes match.
- Inline small UTF-8 text artifacts only when they are at or below `ARTIFACT_INLINE_MAX_BYTES` and the MIME type is text-like.
- Record an audit event containing original path, attachment path, MIME type, size, hash, and tool call id. Do not record file contents.

Tests: snapshot from `VaultLocal`, snapshot from an allowed external working dir, reject traversal, reject missing file, stable hash/name, inline small text, do not inline large text, and one assistant message commits with an artifact block.

## Task 2: Artifact detection and MIME policy

Add a small detector that decides whether a file gets an inline preview.

Preview classes:

- HTML: `text/html`, `.html`, `.htm`
- Markdown: `text/markdown`, `.md`, `.markdown`
- CSV/TSV: `text/csv`, `.csv`, `.tsv`
- Images: PNG, JPEG, GIF, WebP, SVG
- Plain text / JSON: simple text card or preview
- Everything else: typed file card

Prefer extension plus lightweight content sniffing over trusting the model's declared type. If the file extension and bytes disagree, choose the safer renderer. For example, HTML-looking bytes in a `.txt` file may render as text, but a `.html` file with binary bytes should not enter the webview.

Tests: extension detection, byte sniffing, mismatched extension/content fallback, SVG treated as active-ish content and kept inside the sandbox, and unknown binary rendered as a file card.

## Task 3: Sandboxed artifact webview and renderer island

Build the first `WKWebView` host in `/macos/Render`. If the product is adopting a React/TypeScript renderer, build it as a contained app bundle loaded by this host. Swift still owns the app, bridge, permissions, and file access.

Artifact sandbox policy:

- Use an ephemeral data store.
- Disable or intercept all navigation to `http`, `https`, `ftp`, custom schemes, and external `file` URLs.
- Serve artifact bytes through a narrow local scheme or read-only file URL that cannot escape the attachment file.
- Disable popups, downloads, geolocation, camera, microphone, clipboard write, and window opening.
- Block JavaScript dialogs from becoming host UI. If JavaScript is left enabled for static reports, keep it inside the sandbox and document why.
- No cookies, no persistent storage, no shared process pool with authenticated content.
- Add an accessible fallback outside the webview.
- The React renderer, if present, receives artifact/transcript view models and sandboxed resource URLs from Swift. It emits typed intents back to Swift. It does not open arbitrary local paths, inspect the vault, or call the gateway directly.

Apps in M7 will use a related host with pre-declared origins and a JSON-RPC bridge. Keep the host factored so Apps can reuse the frame and loosen only the origin policy they need. Do not add the App bridge in M5.

Tests: local HTML renders, script cannot navigate to network, image tags with remote URLs do not load, external file navigation is blocked, reload works from `messages.jsonl`, and fallback metadata remains accessible if the webview fails.

## Task 4: Non-HTML artifact renderers

Add sensible previews:

- Markdown: render sanitized markdown in the React renderer, a native/text view, or the same no-network webview. Links open only after explicit user action.
- CSV/TSV: table preview with header detection, row/column caps, type-neutral formatting, and open-in-Finder for the full file.
- Images: inline preview, size/format metadata, no remote references.
- Text/JSON: monospaced preview with truncation for large files.
- Unknown: file card with MIME type, size, hash, and reveal/open actions.

Keep cards compact and work-focused. The report is allowed to be rich; the shell around it should stay calm.

Tests: markdown sanitization, CSV row/column caps, large file truncation, image metadata, and unknown file fallback.

## Task 5: Hero flow polish

Wire the whole path for the launch demo:

1. User drags a CSV into the composer.
2. The file lands in the conversation `workdir/`.
3. User asks for a report.
4. ACP agent writes `report.html`.
5. `FileChanged` snapshots the report into `attachments/`.
6. Transcript appends an `Artifact` block.
7. The report renders inline after reload.

Add a manual verification fixture or script with a tiny CSV and a mock ACP agent that writes deterministic `report.html`. This lets the hero path be tested without a real model.

Tests: drag/drop writes into `workdir/`, mock agent writes report, artifact appears, reload redraws, and modifying `workdir/report.html` after the run does not change the rendered attachment.

## Task 6: Integrity and import hardening

Artifact validation already exists at the model boundary. Extend the read path so renderers verify bytes before active rendering:

- Path must be vault-relative under `attachments/`.
- Attachment must exist.
- Size must match.
- SHA-256 must match.
- Inline content, if present, must match the same validation rules.

On failure, show a failed-integrity file card. Do not render HTML, SVG, or any active content when integrity fails. Full import UX lands in M8, but the renderer must be safe now.

Tests: hash mismatch blocks webview, missing attachment shows failed card, bad path rejects load, inline mismatch rejected, and failed cards are keyboard accessible.

## Task 7: Docs and verification

Update docs:

- `/docs/vault-format.md`: snapshot timing, hash naming, inline rules, and replay semantics.
- renderer docs: artifact webview sandbox policy and blocked capabilities.
- `/docs/events-format.md`: artifact snapshot audit event.

Verification:

- Core: `cargo test`, `cargo clippy`.
- Swift: build the macOS app and run a small artifact-render smoke test.
- Renderer, if present: run TypeScript typecheck/build and renderer smoke tests.
- Manual: record the CSV to `report.html` hero flow.

## Out of scope this milestone

Do not build elicitation, URL handoff, or OAuth (M6). Do not build MCP Apps, UI-to-host JSON-RPC, Tasks, or Roots (M7). Do not give artifacts network access. Do not add import/export UX beyond the renderer safety checks. Do not build a full document editor or artifact editing surface.

## Kickoff prompt for Claude Code

> Read CLAUDE.md, docs/milestone-4.md, docs/milestone-5.md, docs/tamtri-decisions.md sections 8, 9, 17, and 18, and docs/vault-format.md. Implement Milestone 5. Start with Task 1 (ArtifactSnapshotter and reducer hook) and Task 3 (sandboxed artifact webview), then stop and show me the snapshot boundary and webview sandbox policy before polishing the non-HTML renderers. The hero is report.html inline, but the security posture matters just as much.
