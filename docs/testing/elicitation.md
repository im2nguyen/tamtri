# Testing Elicitation

Verify tamtri intercepts downstream `elicitation/create` requests, renders follow-up UI, and returns accept/decline/cancel to the server.

## Prerequisites

- Built `tamtri-core` with fixture binaries (`cargo build -p tamtri-core`).
- A vault with gateway config at `<vault>/config.json`.
- An ACP harness connected to the tamtri gateway (for manual UI tests).

## Fixtures

| Fixture | Tool | Mode | Purpose |
|---------|------|------|---------|
| `mock-mcp-server` | `elicit` | form | Minimal single-field smoke test |
| `mock-mcp-server` | `elicit_url` | url | HTTPS consent card + browser handoff |

The `twenty-questions-mcp` fixture is a richer form-mode demo; see [twenty-questions.md](twenty-questions.md).

## Build

```bash
cargo build -p tamtri-core
```

Integration tests resolve fixture paths via `CARGO_BIN_EXE_*` automatically.

## Config example (mock-mcp-server)

Add to `<vault>/config.json` under `gateway.servers`:

```json
{
  "id": "mock",
  "display_name": "Mock MCP",
  "enabled": true,
  "scope": "user",
  "transport": {
    "type": "stdio",
    "command": "/absolute/path/to/tamtri/target/debug/mock-mcp-server",
    "args": [],
    "env": []
  }
}
```

Gateway tool names are prefixed with the server id: `mock__elicit`, `mock__elicit_url`.

## Manual verification

### Form mode

- [ ] Register the mock server in vault config.
- [ ] Start a conversation with a gateway-connected harness.
- [ ] Invoke a gateway tool that elicits in form mode.
- [ ] Confirm an elicitation card appears nested under the active tool call.
- [ ] Submit the form; the downstream tool completes with structured output.
- [ ] Decline or cancel; the tool fails cleanly and the transcript records the resolution.

Form fields are non-secret by design. Never route secrets through form-mode elicitation.

### URL mode

- [ ] Trigger `mock__elicit_url` (or any gateway tool that elicits with `mode: "url"`).
- [ ] Confirm the consent card shows origin and a redacted path (query stripped).
- [ ] Approve to open the system browser; verify the downstream tool completes.
- [ ] Decline and confirm the tool fails without opening the browser.

## Automated tests

```bash
cargo test -p tamtri-core gateway_elicitation
cargo test -p tamtri-core url_elicitation
```

`core/tests/gateway_elicitation.rs` covers form accept round-trips. URL-mode tests cover accept/decline and validation (HTTPS required, userinfo rejected). Audit logs store `url_origin` and a query-stripped `url`.

## Known limitations

- URL mode is for trusted-domain handoff only; do not use it to collect secrets in a form.
- Elicitation during parallel tool calls nests under the correlated `origin_tool_call_id` when the server provides one.
- Remote OAuth servers that elicit during connect use a separate path; see [oauth.md](oauth.md).
