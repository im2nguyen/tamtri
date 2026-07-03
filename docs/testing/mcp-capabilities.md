# Testing MCP Capabilities

Verify capability negotiation, RC extension parsing, per-server badges in settings, and the sampling-declined posture.

## Prerequisites

- Built M7 fixtures (`cargo build -p tamtri-core`).
- Vault gateway config with one or more fixtures registered (see [apps.md](apps.md), [tasks.md](tasks.md), [roots.md](roots.md)).
- tamtri macOS shell for **Probe capabilities** in Settings.

## Fixtures

| Binary | Advertises | Purpose |
|--------|------------|---------|
| `m7-app-mcp` | Apps extension | App template + `show_app` tool |
| `m7-task-mcp` | Tasks extension | Progress, cancel, mid-task input |
| `m7-roots-mcp` | Roots | `probe_roots` tool |
| `m7-rc-mcp` | Apps, Tasks, unknown extension, sampling | RC negotiation + sampling probe |

## Build

```bash
cargo build -p tamtri-core
```

All four binaries land in `target/debug/`.

## Config example (RC probe)

Add `m7-rc-mcp` to exercise extension parsing and sampling decline:

```json
{
  "id": "m7-rc",
  "display_name": "RC capabilities fixture",
  "enabled": true,
  "scope": "user",
  "transport": {
    "type": "stdio",
    "command": "/absolute/path/to/tamtri/target/debug/m7-rc-mcp",
    "args": [],
    "env": []
  }
}
```

Gateway tool: `m7-rc__probe_sampling` (requests sampling; tamtri declines).

## Manual verification

- [ ] Launch tamtri (`cd macos && swift run Tamtri` or Xcode).
- [ ] Open **Settings** → gateway servers → **Probe capabilities**.
- [ ] For `m7-app-mcp`: **Apps** shows **supported** (green).
- [ ] For `m7-task-mcp`: **Tasks** shows **supported** (green).
- [ ] For `m7-roots-mcp`: **Roots** shows **supported** (green).
- [ ] **Sampling** shows **declined** on every server (tamtri never samples; the harness owns the model).
- [ ] Unknown RC extensions (e.g. `io.example/unknown` on `m7-rc-mcp`) do not break initialize or parsing.
- [ ] A 2025-11-25 server without extensions still connects and proxies tools.

Badge legend: **supported** (green) means tamtri and the server both wire the feature. **server only** (orange) means the downstream server advertises it but tamtri has not enabled it yet.

## Automated tests

```bash
cargo test -p tamtri-core mcp_capabilities
cargo test -p tamtri-core
cargo clippy -p tamtri-core --all-targets -- -D warnings
cd macos && swift test && swift build
```

Coverage lives in `core/tests/mcp_capabilities.rs`. Feature-specific tests: `gateway_app.rs`, `gateway_tasks.rs`, `m7_roots.rs`, `app_webview_bridge.rs`.

## Known limitations

- Sampling stays declined by design; tamtri is not the model.
- RC extension identifiers (`io.modelcontextprotocol/apps`, `/tasks`, `/roots`) parse without breaking 2025-11-25 servers, but behavior is gated on `TamtriFeatureSupport::current()`.
- Capability badges reflect last probe; re-probe after config or server changes.
- No automatic discovery of server capabilities beyond initialize and explicit list calls.
