# Orchestration

Workstream C: a **prompt-free run-graph engine** in the daemon. The engine sequences harness runs and branches on structured signals (`TurnEnded`, future verifier output). It never authors prompts or makes model decisions.

## Intelligence vs coordination

| Layer | Owner | Examples |
|-------|-------|----------|
| **Intra-run intelligence** | Harness | Reasoning, tool use, inference within one turn |
| **Inter-run coordination** | tamtri orchestration engine | Fork, send user-authored message, wait, parallel fan-out, loop |

Message text lives in **user-authored recipe files**, **caller-supplied inputs**, or **prior run output** (future). The engine substitutes `{{placeholders}}` only; it does not generate copy.

## Vault layout

Recipes (declarative, legible):

```
<vault>/recipes/
  handoff.json
  committee.json
  my-custom-loop.json
```

Orchestration run records:

```
<vault>/orchestration/<run-id>/
  meta.json    status, recipe id, source conversation, latest fork, branch ids
```

Starter recipes ship with the core and are copied into `<vault>/recipes/` on first `recipes.list`.

## Recipe schema (v1)

```json
{
  "schema_version": 1,
  "id": "handoff",
  "title": "Handoff",
  "description": "Fork to another harness with a briefing.",
  "steps": [
    {
      "type": "fork_run",
      "harness_id": "{{harness_id}}",
      "model_id": "{{model_id}}",
      "message": "{{message}}"
    }
  ]
}
```

### Step types

| Type | Behavior |
|------|----------|
| `fork_run` | Fork from the current conversation, send one message, wait for `TurnEnded` |
| `parallel` | Fork one branch per entry, send in parallel, wait for all |
| `loop` | Repeat fork+send up to `max_iterations`, stop early on `EndTurn` |

Templates use `{{key}}` replaced from `inputs_json` on `orchestration.run`.

## Wire protocol

Gated by `ServerInfo.features.orchestration`.

| Method | Purpose |
|--------|---------|
| `recipes.list` | List recipe summaries |
| `recipes.load` | Load full recipe JSON (`recipe_json`) |
| `orchestration.run` | Execute a recipe synchronously; returns run DTO |
| `orchestration.status` | Load run `meta.json` |
| `orchestration.cancel` | Cancel the active harness run and mark run cancelled |

### Example: handoff

```json
{
  "recipe_id": "handoff",
  "source_conversation_id": "<uuid>",
  "inputs_json": "{\"harness_id\":\"codex-native\",\"model_id\":\"default\",\"message\":\"Continue this analysis.\"}"
}
```

### Example: committee

```json
{
  "recipe_id": "committee",
  "source_conversation_id": "<uuid>",
  "inputs_json": "{\"harness_a\":\"claude-native\",\"model_a\":\"default\",\"harness_b\":\"codex-native\",\"model_b\":\"default\",\"prompt\":\"Review this plan. Analysis only, no edits.\"}"
}
```

## Primitives

Orchestration composes existing conversation primitives:

1. **Fork** — new conversation folder, `forked_from` set, harness/model fixed for the branch
2. **Send** — append user message, start harness run
3. **Wait** — block until `TurnEnded` (turn waiter registered before send to avoid races)

Parallel steps register waiters for all forks, send all messages, then wait for all completions. Harness runs execute concurrently.

## Exposure (current)

- **User-initiated:** wire methods above; shell UI ("run recipe") deferred to remaining UI workstream
- **Agent-initiated:** MCP tools on the gateway surface deferred; all orchestration calls will use the same consent/audit path when added

## Deferred

- Async `orchestration.run` (background job + event stream)
- Condition edges beyond `TurnEndReason` (verifier schema output)
- Schedule / cron triggers
- Agent-facing MCP tools (`spawn_run`, `wait`, `handoff`)
- Run-graph visualization in the shell
