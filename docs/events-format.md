# Events Format

`events.jsonl` is the local receipt log. It is append-only, one compact JSON `Event` per line, and lives beside `messages.jsonl` in each conversation folder.

```text
<vault>/conversations/<conversation>/events.jsonl
```

## Schema

```json
{
  "ts": "2026-07-03T00:00:00Z",
  "kind": "turn_started",
  "payload": {}
}
```

`kind` is one of:

- `turn_started`
- `turn_ended`
- `permission_requested`
- `permission_resolved`
- `tool_call_started`
- `tool_call_completed`
- `gateway_server_connected`
- `gateway_tool_routed`
- `gateway_progress`
- `gateway_log`
- `gateway_cancellation`
- `gateway_credential_injected`
- `gateway_downstream_error`
- `harness_spawned`
- `harness_exited`
- `error`

`payload` is JSON and intentionally event-specific.

## Rules

The event log is local by default. It is not part of a portable share bundle unless the user explicitly opts in.

Secrets never appear in payloads. The codec rejects secret-like keys such as `secret`, `token`, `password`, and `api_key`.

Gateway credential receipts record references and target kinds only, for example `{ "server_id": "linear", "credential_ref": "keychain://linear", "target_kind": "header" }`. The resolved value is never serialized.

Milestone 4 writes gateway receipts for server connection, tool routing, credential injection, progress, logging, cancellation, and downstream errors.

Standalone event appends take the same per-conversation advisory lock as message writes. Reads tolerate a torn final line in memory. Writers repair torn tails before appending.
