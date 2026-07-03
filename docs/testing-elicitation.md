# Testing Elicitation (Milestone 6)

This guide covers manual and automated testing of tamtri gateway elicitation using the `twenty-questions-mcp` fixture.

## Fixture

`fixtures/twenty-questions-mcp` is a stdio MCP server that elicits during `tools/call`:

| Tool | Elicits? | Purpose |
|------|----------|---------|
| `start_game` | No | Picks a secret word, returns `gameId` |
| `submit_question` | Yes (form) | User asks a yes/no question |
| `make_guess` | Yes (form) | User submits a final guess |

The server follows the same pattern as `fixtures/mock-mcp-server` (`elicit` tool): while handling `tools/call`, it writes an outbound `elicitation/create` request, reads the JSON-RPC response from stdin, then finishes the tool call.

## Build

```bash
cargo build -p tamtri-core --bin twenty-questions-mcp
```

Integration tests use `CARGO_BIN_EXE_twenty-questions-mcp` automatically.

## Vault config

tamtri reads gateway servers from `<vault>/config.json`. Example entry:

```json
{
  "gateway": {
    "default_call_timeout_secs": 300,
    "servers": [
      {
        "id": "twenty_questions",
        "display_name": "20 Questions",
        "enabled": true,
        "scope": "user",
        "transport": {
          "type": "stdio",
          "command": "/Users/you/Desktop/tamtri/target/debug/twenty-questions-mcp",
          "args": [],
          "env": [
            ["TWENTY_QUESTIONS_SEED", "42"]
          ]
        }
      }
    ]
  }
}
```

Use an absolute path to the built binary. The seed env var is optional; it makes the secret word repeatable.

Gateway-exposed tool names are prefixed with the server id: `twenty_questions__start_game`, `twenty_questions__submit_question`, `twenty_questions__make_guess`.

## Manual UI test

1. Register the server in your vault `config.json`.
2. Launch tamtri and open a conversation with an ACP harness that uses the tamtri gateway.
3. Prompt the agent to start a 20 Questions game via gateway tools.
4. Confirm elicitation cards appear nested under the active tool call.
5. Submit yes/no questions; the agent should receive `yes`, `no`, or `maybe` in structured tool output.
6. Finish with `twenty_questions__make_guess` and confirm win/loss text.

Form fields are non-secret by design (questions and guesses only).

## Automated test

`core/tests/elicitation_integration.rs` runs `gateway_elicitation_twenty_questions_form_accept_round_trip`:

1. Spawns the fixture through `McpGateway`.
2. Calls `start_game`.
3. Calls `submit_question` on a background task.
4. Waits for `GatewayEvent::ElicitationRequested`.
5. Responds with `respond_elicitation(accept, { question })`.
6. Asserts the tool result contains the expected answer.

Run:

```bash
cargo test -p tamtri-core gateway_elicitation_twenty_questions
```

## Related fixtures

- `mock-mcp-server` / `elicit` tool: minimal single-field elicitation smoke test.
- See `docs/milestone-6.md` for the full elicitation + OAuth milestone checklist.
