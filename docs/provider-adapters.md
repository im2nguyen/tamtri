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

`sessions.list_native` scans Claude Code `~/.claude/projects/**/*.jsonl` and Codex
`~/.codex/sessions` for on-ramp rows (title, cwd, session id when known).

`sessions.import` seeds a new vault conversation from a native session file. Claude
jsonl history becomes the tamtri transcript; `meta.json` stores a
`native_session` link so the next run uses `claude --resume`.

Params: `{ provider, path, harness_id, model_id }`.

## Status

- **ACP:** production path today.
- **Codex native:** `codex app-server` transport wired (`CodexNativeAdapter`).
  Set `adapter: codex_native` and `command: codex` with `args: ["app-server"]`
  in roster config. Integration tests: `TAMTRI_CODEX_COMMAND=codex`.
- **Claude native:** `claude --print --output-format stream-json --verbose`
  transport wired (`ClaudeNativeAdapter`). Set `adapter: claude_native`. Resume
  uses `native_session` from import or the first successful run. Integration
  tests: `TAMTRI_CLAUDE_COMMAND=claude`.
- **Codex import:** rollout jsonl under `~/.codex/sessions/**.jsonl` parses into
  the vault transcript; `native_session.session_id` is the Codex thread id for
  `thread/resume` on the next run.
