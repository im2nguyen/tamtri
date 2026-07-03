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
- `artifact_snapshotted`
- `artifact_navigation_blocked`
- `gateway_server_connected`
- `gateway_server_disconnected`
- `gateway_tool_routed`
- `gateway_progress`
- `gateway_log`
- `gateway_cancellation`
- `gateway_credential_injected`
- `gateway_downstream_error`
- `elicitation_requested`
- `elicitation_resolved`
- `oauth_handoff_started`
- `oauth_handoff_completed`
- `oauth_refresh_failed`
- `harness_spawned`
- `harness_exited`
- `error`

`payload` is JSON and intentionally event-specific.

## Rules

The event log is local by default. It is not part of a portable share bundle unless the user explicitly opts in.

Secrets never appear in payloads. The codec rejects secret-like keys such as `secret`, `token`, `password`, and `api_key`.

Gateway credential receipts record references and target kinds only, for example `{ "server_id": "linear", "credential_ref": "keychain://linear", "target_kind": "header" }`. The resolved value is never serialized.

Milestone 6 writes `elicitation_requested` and `elicitation_resolved` when a downstream server elicits during a tool call. URL-mode receipts include `url_origin` and a query-stripped `url`; query parameters are never stored. OAuth lifecycle receipts (`oauth_handoff_started`, `oauth_handoff_completed`, `oauth_refresh_failed`) record server id, credential references, and status only. Token values never appear in payloads.

Milestone 4 writes gateway receipts for server connection, tool routing, credential injection, progress, logging, cancellation, and downstream errors.

Milestone 5 writes `artifact_snapshotted` when a renderable file is copied from `workdir/` into `attachments/`. The payload records original path, attachment path, MIME type, size, and SHA-256, never file contents. When the snapshot came from an explicit harness `FileChanged`, the payload also includes `tool_call_id`. Snapshots from `referenced_paths` without an explicit `FileChanged` omit `tool_call_id`.

Blocked artifact or workdir HTML navigation (external `http`, `https`, `file`, or custom schemes) is logged as `artifact_navigation_blocked` with `{ "url": "<blocked-url>" }`.

Milestone 7 adds gateway receipts for Apps, Tasks, and Roots:

- `app_returned` — `{ "server_id", "template_ref", "uri", "origin_tool_call_id?", "state" }` (state is JSON; no secrets).
- `app_bridge_consent_requested` / `app_bridge_resolved` — app-initiated bridge actions with `request_id`, `server_id`, `app_id`, `template_ref`, action summary, and resolution. No secret values.
- `app_navigation_blocked` — `{ "server_id", "template_ref", "url" }` when an App webview hits an undeclared origin.
- `task_started`, `task_updated`, `task_completed` — task state snapshots (`task_id`, `server_id`, `status`, optional `title`, `progress`, `result`). Full payloads mirror `TaskState` in core.
- `roots_listed` — `{ "count" }` when a downstream server requests roots through the gateway (paths only, never bookmark bytes).

Standalone event appends take the same per-conversation advisory lock as message writes.
