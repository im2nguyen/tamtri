# Manual demo: Milestone 7 (Apps, Tasks, Roots)

Hermetic fixtures ship with `cargo build`. Paths below assume `target/debug/`.

## 1. Build fixtures

```bash
cargo build -p tamtri-core
```

Binaries: `m7-app-mcp`, `m7-task-mcp`, `m7-roots-mcp`, `m7-rc-mcp`.

## 2. Gateway config

Edit `<vault>/config.json` and add stdio servers (merge into existing `servers` array):

```json
{
  "id": "m7-app",
  "display_name": "M7 App fixture",
  "enabled": true,
  "scope": "user",
  "transport": {
    "type": "stdio",
    "command": "/absolute/path/to/tamtri/target/debug/m7-app-mcp",
    "args": [],
    "env": []
  }
}
```

Repeat for `m7-task-mcp` (`id`: `tasks`) and `m7-roots-mcp` (`id`: `m7roots`).

## 3. Probe capabilities

1. Launch tamtri (`cd macos && swift run Tamtri` or Xcode).
2. Open **Settings** → **Probe capabilities**.
3. Confirm badges: **Apps**, **Tasks**, **Roots** show **supported** (green) for fixtures that advertise them.
4. Confirm **Sampling** shows **declined** regardless of server (tamtri never samples).

## 4. Apps demo

1. Start a conversation with a harness that can call gateway tools.
2. Invoke gateway tool `m7-app__show_app`.
3. Expect an inline **MCP App** panel with native title/server chrome (VoiceOver reads these outside the webview).
4. If the App requests a bridge action, a consent card appears; deny blocks execution, allow routes through the gateway.
5. Reload the conversation: `app_resource` block replays; offline state shows without crashing the transcript.

## 5. Tasks demo

1. Call `tasks__progress_task` — live task card with status updates.
2. Call `tasks__cancelable_task` and press **Cancel task** on the card.
3. Call `tasks__input_task` — mid-task elicitation form; completing it finishes the task.
4. After completion, transcript contains a `task_ref` block.

## 6. Roots demo

1. Open conversation header → **Roots**.
2. **Add Folder** — pick a directory; bookmark saved under Application Support (not in vault).
3. Call `m7roots__probe_roots` — structured content lists attached roots.
4. Remove bookmark file manually and reopen Roots — missing bookmark warning with **Re-pick Folder**.

## 7. Automated checks

```bash
cargo test -p tamtri-core
cargo clippy -p tamtri-core --all-targets -- -D warnings
cd macos && swift test && swift build
```

All sixteen enumerated tests in `docs/milestone-7.md` are covered by `core/tests/gateway_app.rs`, `gateway_tasks.rs`, `m7_roots.rs`, `mcp_capabilities.rs`, `app.rs`, and `app_bridge.rs`, plus Swift policy tests in `RendererPolicyTests`.
