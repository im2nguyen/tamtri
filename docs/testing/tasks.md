# Testing Tasks

Verify long-running downstream work surfaces as live task cards with cancellation, mid-task input, and durable `task_ref` blocks.

## Prerequisites

- Built `m7-task-mcp` and `m7-task-subscribe-mcp` fixtures (`cargo build -p tamtri-core`).
- Vault gateway config with the tasks fixture registered (`id`: `tasks`).
- An ACP harness that calls tamtri gateway tools.

## Build

```bash
cargo build -p tamtri-core
```

Binaries: `target/debug/m7-task-mcp`, `target/debug/m7-task-subscribe-mcp`

## Config example

Add to `<vault>/config.json`:

```json
{
  "id": "tasks",
  "display_name": "Tasks fixture",
  "enabled": true,
  "scope": "user",
  "transport": {
    "type": "stdio",
    "command": "/absolute/path/to/tamtri/target/debug/m7-task-mcp",
    "args": [],
    "env": []
  }
}
```

Gateway tools (prefixed `tasks__`):

- `tasks__progress_task` — emits status updates until completion
- `tasks__cancelable_task` — supports cancellation from the card
- `tasks__input_task` — pauses for mid-task elicitation

## Manual verification

- [ ] Register the server; probe capabilities; **Tasks** badge is green.
- [ ] Call `tasks__progress_task` — live task card with status updates.
- [ ] Call `tasks__cancelable_task` and press **Cancel task** on the card.
- [ ] Call `tasks__input_task` — mid-task elicitation form; completing it finishes the task.
- [ ] After completion, transcript contains a `task_ref` block.
- [ ] Detailed task events appear in `events.jsonl` (local audit, not portable).

## Automated tests

```bash
cargo test -p tamtri-core gateway_tasks
```

Coverage lives in `core/tests/gateway_tasks.rs`.

## Known limitations

- RC task subscribe is gated behind capability checks; polling is the fallback. `m7-task-subscribe-mcp` exercises the subscribe path (`task_subscribe_updates_live_card`).
- Mid-task input reuses the M6 elicitation path (form mode).
- Task cards are non-blocking; the harness may continue other work in parallel.
