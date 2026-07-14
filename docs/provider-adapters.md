# Provider adapters

Harnesses plug in behind the `HarnessAdapter` trait (`core/src/harness/mod.rs`).
The roster in `config.json` maps each entry to an adapter implementation via
`adapter` kind:

| `adapter` | Implementation | When to use |
|-----------|----------------|-------------|
| `acp` (default) | `AcpAdapter` | Long-tail ACP agents (Hermes, Goose, …) |
| `claude_native` | `ClaudeNativeAdapter` | Claude Code native fidelity (ACP is insufficient) |
| `codex_native` | `CodexNativeAdapter` | Codex via codex-app-server |
| `opencode_native` | `OpenCodeNativeAdapter` | OpenCode via `opencode serve` HTTP API |
| `pi_native` | `PiNativeAdapter` | Pi via `pi --mode rpc` stdio JSON lines |

Registry: `core/src/harness/registry.rs`.

## Capability flags

`HarnessCapabilities.native_tools`: when true, the adapter can accept a runtime
tool catalog directly. When false, tamtri passes itself as the sole MCP server
(the gateway) and the adapter uses MCP for rich primitives.

Never blur **gateway tools** (proxied, consent-gated, credential-brokered) vs
**agent-native tools** (harness's own servers) in the UI.

## Native tool catalog

When `native_tools: true`, the core lists gateway tools into
`ConversationContext.tool_catalog` before each run. Codex receives them on
`turn/start` as a `tools` array; Claude receives JSON in
`TAMTRI_NATIVE_TOOL_CATALOG`. ACP adapters continue to receive tamtri as an MCP
server via `mcp_servers`.

## Roster discovery

`discover_known_agents()` probes PATH (and known install locations for Hermes) and
returns `AgentLaunchSpec` rows for auto-seeding an empty roster. ACP agents use
`adapter: acp` (default); native adapters set `claude_native`, `codex_native`,
`opencode_native`, or `pi_native`.

| id | command | args | notes |
|----|---------|------|-------|
| `hermes-acp` | `hermes` | `["acp"]` | also checks `~/.local/bin/hermes` |
| `claude-native` | `claude` | `[]` | `claude_native` adapter |
| `claude-code-acp` | `claude` | `["acp"]` | ACP fallback |
| `codex-native` | `codex` | `["app-server"]` | `codex_native` adapter |
| `goose-acp` | `goose` | `[]` | Goose speaks ACP on stdio by default |
| `opencode-native` | `opencode` | `["serve"]` | `opencode_native` adapter; per-run server |
| `opencode-acp` | `opencode` | `["acp"]` | ACP fallback |
| `pi-native` | `pi` | `[]` | `pi_native` adapter; adds `--mode rpc` |
| `pi-acp` | `pi-acp` | `[]` | [pi-acp bridge](https://github.com/svkozak/pi-acp) fallback |

`seed_agent_roster_if_empty` prefers native entries when binaries are present:

- `claude-native` (`claude_native`) before `claude-code-acp`
- `codex-native` (`codex_native`) when `codex` is on PATH
- `opencode-native` (`opencode_native`) when `opencode` is on PATH (ACP listed separately)
- `pi-native` (`pi_native`) when `pi` is on PATH (ACP bridge only if `pi-acp` is present)

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
- **Pi native:** `pi --mode rpc` NDJSON stdio wired (`PiNativeAdapter`). Set
  `adapter: pi_native`. Resume passes `--session` from `native_session.source_path`
  when set. Integration tests: `TAMTRI_PI_COMMAND=pi` or the mock at
  `fixtures/mock-pi-rpc.sh`.
- **OpenCode native:** `opencode serve` HTTP + SSE wired (`OpenCodeNativeAdapter`).
  Spawns a local server per run, creates a session with `x-opencode-directory`,
  sends `POST /session/:id/prompt_async`, and maps `/event` SSE to `HarnessEvent`.
  Set `adapter: opencode_native`. Integration tests: `TAMTRI_OPENCODE_COMMAND=opencode`
  (requires a configured OpenCode install with at least one provider).
- **Codex import:** rollout jsonl under `~/.codex/sessions/**.jsonl` parses into
  the vault transcript; `native_session.session_id` is the Codex thread id for
  `thread/resume` on the next run.

### OpenCode native limitations (MVP)

- Per-run `opencode serve` subprocess (no shared server pool yet).
- Event mapping covers text/reasoning deltas, basic tool parts, permissions, and
  `session.idle` / `session.error`. Sub-agents, questions, todos, and rewind are
  not mapped yet.
- MCP gateway injection is not wired; OpenCode uses its own MCP config.
- No OpenCode session import/list_native yet (unlike Claude/Codex).

### Pi native limitations (MVP)

- Transcript seeding is prompt replay (no Pi tree/rewind RPC).
- MCP config and extension paths from `ConversationContext` are not injected yet.
- `extension_ui_request` permission mapping is generic (no ask_user specialization).
