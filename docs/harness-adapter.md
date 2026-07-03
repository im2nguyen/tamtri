# Harness Adapter

`HarnessAdapter` is the seam between tamtri core and agent harnesses. It is the future plugin contract, so ACP details stay inside `harness/acp.rs`; callers only see normalized `HarnessEvent`s.

## Interface

An adapter exposes identity, capabilities, model listing, and `run(ctx, turn) -> HarnessRun`.

`ConversationContext` contains a `ContextSeed`, working directory mode, resolved working directory path, roots, MCP server refs, and selected model id. V1 supports only `FreshTranscript`.

`HarnessRun` returns:

- `events`: a stream of `HarnessEvent`.
- `control`: cancellation and permission-response methods.

No ACP type appears in the public adapter interface.

## Event Vocabulary

The event vocabulary mirrors `CLAUDE.md`:

- `TextDelta`, `ThoughtDelta`.
- `ToolCallStarted`, `ToolCallProgress`, `FileChanged`, `TerminalOutput`.
- `PermissionRequested`, `PermissionResolved`.
- `PlanUpdated`, `ModeChanged`, `Error`, `TurnEnded`.

Adapters emit the subset their harness can provide. Renderers and reducers consume the same normalized variants regardless of source.

## FreshTranscript Rendering

For ACP seed injection, `AcpAdapter` renders prior messages as a plain transcript:

```text
User: prior text
Assistant: prior response
User: current turn
```

Text and thinking blocks render as text. Tool calls render as `tool <name>: <input>`. Tool results render as JSON. This is intentionally simple and local to the adapter; future adapters may inject seeds differently.

## ACP Adapter Notes

`AcpAdapter` uses the shared `RpcConnection` dispatch loop because ACP can send `session/request_permission` while `session/prompt` is pending.

Current normalization is value-based and deliberately contained:

- `session/update` notification params with `type = agent_message_chunk` become `TextDelta`.
- `agent_thought_chunk` becomes `ThoughtDelta`.
- `tool_call` becomes `ToolCallStarted`.
- `tool_call_update` becomes `ToolCallProgress`; diff content also emits `FileChanged`.
- `session/request_permission` becomes `PermissionRequested` and waits for `RunControl::respond_permission`.

Exact ACP field names should be pinned again when integrating a real agent. Any harness-specific enrichment belongs in a composed adapter, not in the core event contract.

## Reduction

`TurnReducer` folds a stream into one assistant `Message` per completed turn:

- Text and thought deltas collapse into `Text` and `Thinking` blocks.
- Tool start/progress becomes `ToolCall` and `ToolResult`.
- Tool result output carries rendered content so the transcript redraws offline.
- `FileChanged` records a hook for M5, but does not create an `Artifact` block yet.
- Permission requests/resolutions persist compactly in transcript blocks. Full details belong in `events.jsonl`.

