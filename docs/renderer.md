# Renderer

Milestone 5 introduces the first artifact renderer. Swift still owns the app, vault, permissions, keychain, and harness lifecycle; renderer surfaces receive already-reduced transcript view models.

## Artifact Snapshots

The renderer never reads from `workdir/`. Core snapshots renderable `FileChanged` outputs into `attachments/`, records size and SHA-256 in the transcript `Artifact` block, and emits an `artifact_snapshotted` audit receipt.

## HTML Sandbox

HTML and SVG artifacts render from the frozen inline snapshot when the artifact is small enough to inline. Larger file-backed HTML/SVG artifacts are read through core's verified attachment API, which rejects bad paths, missing files, size mismatches, and SHA-256 mismatches before Swift receives bytes. The `WKWebView` uses a non-persistent data store and no host bridge. Swift wraps the document with a strict Content Security Policy:

```text
default-src 'none';
img-src data:;
style-src 'unsafe-inline';
script-src 'none';
base-uri 'none';
form-action 'none'
```

The navigation delegate allows only the initial `about:` document and cancels all other top-level navigation. Blocked navigations are logged to `events.jsonl` as `artifact_navigation_blocked`. No artifact webview gets network access, cookies, persistent storage, popups, downloads, geolocation, camera, microphone, clipboard writes, or a host-call bridge.

## Non-HTML Previews

Markdown and plain text render as selectable native text. CSV/TSV render as a capped native grid preview. PNG, JPEG, GIF, and WebP render natively after integrity verification. Unknown artifacts render as typed file cards with metadata rather than active content.

## Artifact Actions

Open and Reveal actions are available from artifact cards. They first resolve the artifact through core's verified attachment API; AppKit only receives a local file URL after the path is confirmed to be under `attachments/` and the file's size and SHA-256 match the transcript.
