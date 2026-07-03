# Testing MCP Apps

Verify inline MCP App rendering, declared-origin network policy, consent-gated bridge actions, and transcript replay.

## Prerequisites

- Built `m7-app-mcp` fixture (`cargo build -p tamtri-core`).
- Vault gateway config with the app fixture registered.
- An ACP harness that calls tamtri gateway tools.

## Build

```bash
cargo build -p tamtri-core
```

Binary: `target/debug/m7-app-mcp`

## Config example

Add to `<vault>/config.json` under `gateway.servers`:

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

Gateway tool: `m7-app__show_app`

The fixture declares an App template at `ui://m7-app/demo` with `text/html;profile=mcp-app` MIME type and advertises the `io.modelcontextprotocol/apps` RC extension.

## Manual verification

- [ ] Register the server and probe capabilities (see [mcp-capabilities.md](mcp-capabilities.md)); **Apps** badge is green.
- [ ] Start a conversation with a gateway-connected harness.
- [ ] Invoke `m7-app__show_app`.
- [ ] Expect an inline **MCP App** panel with native title/server chrome (VoiceOver reads these outside the webview).
- [ ] If the App requests a bridge action, a consent card appears; deny blocks execution, allow routes through the gateway.
- [ ] Reload the conversation; `app_resource` block replays; offline state shows without crashing the transcript.

## Automated tests

```bash
cargo test -p tamtri-core gateway_app
cargo test -p tamtri-core app_bridge
```

Coverage lives in `core/tests/gateway_app.rs` and `core/tests/app_webview_bridge.rs`, plus Swift policy tests in `RendererPolicyTests`.

## Known limitations

- Apps get declared origins only; artifacts stay no-network (see `WebContentPolicy` in milestone docs).
- UI-initiated App actions use the same consent/audit path as direct tool calls; there is no silent bridge.
- MCP App code cannot call the trusted renderer bridge; two bridges are intentionally separate.
- Sampling is always declined; Apps do not depend on it.
