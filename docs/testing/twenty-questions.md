# Testing 20 Questions (Elicitation Demo)

Hero manual demo for form-mode elicitation. The fixture plays [20 Questions](https://en.wikipedia.org/wiki/20_Questions) by pausing each `tools/call` and sending `elicitation/create` for yes/no questions and final guesses.

## Prerequisites

- Built `twenty-questions-mcp` binary.
- Vault `config.json` with gateway servers.
- An ACP harness that calls tamtri gateway tools.

## Fixture

`fixtures/twenty-questions-mcp` is a stdio MCP server that elicits during `tools/call`:

| Tool | Elicits? | Purpose |
|------|----------|---------|
| `start_game` | No | Picks a secret word, returns `gameId` |
| `submit_question` | Yes (form) | User asks a yes/no question |
| `make_guess` | Yes (form) | User submits a final guess |

The server follows the same pattern as `mock-mcp-server` (`elicit` tool): while handling `tools/call`, it writes an outbound `elicitation/create` request, reads the JSON-RPC response from stdin, then finishes the tool call.

Secret words are drawn from a small fixed list: apple, elephant, bicycle, guitar, umbrella.

## Build

```bash
cargo build -p tamtri-core --bin twenty-questions-mcp
```

Binary path (debug): `target/debug/twenty-questions-mcp`

## Config example

Add to `<vault>/config.json`:

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
          "command": "/absolute/path/to/tamtri/target/debug/twenty-questions-mcp",
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

Use an absolute path to the built binary. Optional `TWENTY_QUESTIONS_SEED` makes the secret word repeatable.

Gateway-exposed tool names:

- `twenty_questions__start_game`
- `twenty_questions__submit_question`
- `twenty_questions__make_guess`

## Manual verification

- [ ] Register the server in vault `config.json`.
- [ ] Launch tamtri and open a conversation with a gateway-connected harness.
- [ ] Prompt the agent to start a 20 Questions game via gateway tools.
- [ ] Confirm elicitation cards appear nested under the active tool call.
- [ ] Submit yes/no questions; the agent receives `yes`, `no`, or `maybe` in structured tool output.
- [ ] Finish with `twenty_questions__make_guess` and confirm win/loss text.

Example prompt: "Call `twenty_questions__start_game`, then keep calling `twenty_questions__submit_question` until you can guess."

## Automated test

`core/tests/gateway_elicitation.rs` runs `gateway_elicitation_twenty_questions_form_accept_round_trip`:

1. Spawns the fixture through `McpGateway`.
2. Calls `start_game`.
3. Calls `submit_question` on a background task.
4. Waits for `GatewayEvent::ElicitationRequested`.
5. Responds with `respond_elicitation(accept, { question })`.
6. Asserts the tool result contains the expected answer.

```bash
cargo test -p tamtri-core gateway_elicitation_twenty_questions
```

## Known limitations

- Form fields are non-secret (questions and guesses only).
- The fixture uses a fixed word list; it does not call a model.
- For general elicitation mechanics (URL mode, mock server), see [elicitation.md](elicitation.md).
