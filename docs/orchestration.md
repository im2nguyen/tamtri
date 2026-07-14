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
| `orchestration.run` | Start a recipe in the background; returns run DTO with `status: "running"` |
| `orchestration.status` | Load run `meta.json` |
| `orchestration.cancel` | Cancel the active harness run and mark run cancelled |

### Async execution

When the daemon has installed a shared `TamtriCore` (`TamtriCore::install_shared`), `orchestration.run` returns immediately with `status: "running"`. The engine executes the recipe on a background task. Progress is pushed on the source conversation's UiEvent stream:

| UiEvent kind | Payload |
|--------------|---------|
| `orchestration_started` | `run_id`, `recipe_id`, `source_conversation_id` |
| `orchestration_step_started` | `run_id`, `step_index`, `step_type` |
| `orchestration_forked` | `run_id`, `conversation_id`, `harness_id`, `model_id` |
| `orchestration_branch_completed` | `run_id`, `conversation_id`, `reason` |
| `orchestration_finished` | full run DTO under `run` |

Poll `orchestration.status` for the latest snapshot. Cancel via `orchestration.cancel` (sets an atomic on the run handle and cancels the active harness turn).

Without a shared core (unit tests without the daemon), `orchestration.run` falls back to synchronous execution.

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

## Agent MCP tools (gateway)

During an active harness run, the gateway sets `agent_context` on the conversation. When orchestration is enabled for that context, tamtri prepends native tools on `tools/list`:

| Exposed name | Purpose | Consent |
|--------------|---------|---------|
| `tamtri__orchestration_run` | Start a background recipe from the current conversation | Yes |
| `tamtri__orchestration_status` | Read run status by `run_id` | No |
| `tamtri__orchestration_cancel` | Cancel a running orchestration | Yes |
| `tamtri__orchestration_handoff` | Shortcut for the handoff recipe (`harness_id`, `model_id`, `message`) | Yes |

Calls route through the gateway like any downstream tool. Consent-gated tools emit a `permission_requested` UiEvent (same shape as harness permissions). The user responds via `permission.respond`; orchestration consents are resolved before harness permission handlers run. Each routed call emits a `ToolRouted` audit event before execution.

Native tools only appear when `agent_context.orchestration_enabled` is true (set when a harness run starts on a host with orchestration feature enabled).

## Primitives

Orchestration composes existing conversation primitives:

1. **Fork** — new conversation folder, `forked_from` set, harness/model fixed for the branch
2. **Send** — append user message, start harness run
3. **Wait** — block until `TurnEnded` (turn waiter registered before send to avoid races)

Parallel steps register waiters for all forks, send all messages, then wait for all completions. Harness runs execute concurrently.

## Exposure

- **User-initiated:** wire methods above; shell run-recipe UI subscribes to orchestration UiEvents and polls status
- **Agent-initiated:** gateway MCP tools above, consent/audit gated

## Deferred

- Condition edges beyond `TurnEndReason` (verifier schema output)
- Schedule / cron triggers
- Run-graph visualization in the shell
