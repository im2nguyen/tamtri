# twenty-questions-mcp

Stdio MCP fixture for tamtri elicitation testing. Verification guide: [docs/testing/twenty-questions.md](../../docs/testing/twenty-questions.md). The server picks a secret word and plays [20 Questions](https://en.wikipedia.org/wiki/20_Questions) by pausing `tools/call` and sending `elicitation/create` (form mode) for each yes/no question or final guess.

## Build

From the repo root:

```bash
cargo build -p tamtri-core --bin twenty-questions-mcp
```

Binary path (debug):

```text
target/debug/twenty-questions-mcp
```

## Gateway config

Add this server to `<vault>/config.json` under `gateway.servers`. tamtri stores vault config as JSON, not TOML.

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
          "env": []
        }
      }
    ]
  }
}
```

Replace the `command` path with your local build output. Optional env `TWENTY_QUESTIONS_SEED` (integer string) makes the secret word deterministic for manual testing.

Exposed gateway tools:

- `twenty_questions__start_game`
- `twenty_questions__submit_question`
- `twenty_questions__make_guess`

## Play in tamtri

1. Build the binary and register the server in `~/.tamtri/vault/config.json` (or your vault path).
2. Start a conversation with any ACP harness connected to the tamtri gateway.
3. Ask the agent to play 20 Questions using the gateway tools, for example: "Call `twenty_questions__start_game`, then keep calling `twenty_questions__submit_question` until you can guess."
4. When a tool elicits, tamtri shows a form card under the tool call. Enter a yes/no question or your final guess and submit.
5. The agent sees the tool result (`yes` / `no` / `maybe`) and continues until it calls `twenty_questions__make_guess` or runs out of turns.

Secret words are drawn from a small fixed list: apple, elephant, bicycle, guitar, umbrella.
