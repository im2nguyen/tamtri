# Provider adapters

Harnesses plug in behind the `HarnessAdapter` trait (`core/src/harness/mod.rs`).
The roster in `config.json` maps each entry to an adapter implementation via
`adapter` kind:

| `adapter` | Implementation | When to use |
|-----------|----------------|-------------|
| `acp` (default) | `AcpAdapter` | Long-tail ACP agents (Hermes, OpenCode, …) |
| `claude_native` | `ClaudeNativeAdapter` | Claude Code native fidelity (ACP is insufficient) |
| `codex_native` | `CodexNativeAdapter` | Codex via codex-app-server |

Registry: `core/src/harness/registry.rs`.

## Capability flags

`HarnessCapabilities.native_tools`: when true, the adapter can accept a runtime
tool catalog directly. When false, tamtri passes itself as the sole MCP server
(the gateway) and the adapter uses MCP for rich primitives.

Never blur **gateway tools** (proxied, consent-gated, credential-brokered) vs
**agent-native tools** (harness's own servers) in the UI.

## Native session import

` sessions.list_native` scans `~/.claude/projects` and `~/.codex/sessions` for
on-ramp rows. Import into the vault (seeding a new conversation) is the next
step; listing is available now.

## Status

- **ACP:** production path today.
- **Claude / Codex native:** roster + capability seams landed; subprocess/SDK
  transport is the next implementation slice.
