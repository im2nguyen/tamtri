# Test fixtures

This directory holds **development and test binaries only**. Nothing here ships in the tamtri app or daemon release.

## Why it exists

Integration tests in `tamtri-core` spawn these programs as mock harnesses and MCP servers. They let CI and local `cargo test` run without Claude Code, Codex, or real downstream MCP servers installed.

## Contents

| Fixture | Purpose |
|---------|---------|
| `mock-acp-agent/` | Scripted ACP agent over stdio (thought, text, tool call, permission) |
| `mock-mcp-server/` | Stdio MCP server with tools/resources/prompts |
| `twenty-questions-mcp/` | Elicitation demo server (form-mode game) |
| `m7-rc-mcp/` | MCP capability / RC extension probes |
| `m7-roots-mcp/` | Roots listing tests |
| `m7-task-mcp/` | Task lifecycle tests |
| `m7-task-subscribe-mcp/` | Task subscription tests |
| `m7-app-mcp/` | MCP Apps sandbox tests |
| `mock-pi-rpc.sh` | Mock Pi `--mode rpc` NDJSON for native adapter tests |
| `config.example.json` | Example vault `config.json` (roster + gateway servers) |

The `m7-*` names are historical; they refer to MCP feature areas (Apps, Tasks, Roots), not product milestones.

## Usage

Fixtures are registered as Cargo binary targets in [`core/Cargo.toml`](../core/Cargo.toml). Build them with:

```bash
cargo build -p tamtri-core
```

Optional integration tests use env overrides, for example `TAMTRI_PI_COMMAND=fixtures/mock-pi-rpc.sh`.

## Not user-facing

End users install real agent apps (Claude Code, Codex, Hermes, etc.) and configure them through the tamtri **Agents & providers** screen (`/health`). See [docs/getting-started.md](../docs/getting-started.md).
