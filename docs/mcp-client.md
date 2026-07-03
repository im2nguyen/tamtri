# MCP Client

Milestone 2 implements the downstream MCP client baseline in `tamtri-core`. It speaks MCP `2025-11-25` over stdio, completes `initialize` plus `notifications/initialized`, lists tools with pagination, and calls tools.

## Transport

`StdioTransport` uses newline-delimited JSON-RPC over child stdin/stdout. Stderr is drained into `tracing::debug!` and is never parsed as protocol.

Child environments are scrubbed by default. The transport starts from an empty environment, preserves only `PATH`, `HOME`, `TMPDIR`, and `LANG` when present, then applies explicitly provided environment pairs. This keeps the later gateway credential story honest: credentials are passed deliberately, not inherited accidentally.

The real-subprocess integration test uses `fixtures/mock-mcp-server`, wired as a local Cargo fixture binary so Cargo exposes `CARGO_BIN_EXE_mock-mcp-server` to the test without invoking nested Cargo or Node.

## Protocol

All JSON-RPC messages are classified by field presence:

- `method` and `id`: server-initiated request.
- `method` without `id`: notification.
- `id` without `method`: response, with exactly one of `result` or `error`.

This avoids serde untagged fallthrough, where a server request could otherwise be misread as an empty response. JSON-RPC ids parse as numbers or strings, although tamtri emits numeric ids.

## Timeouts

`McpClientConfig` carries `init_timeout` and `call_timeout`. Defaults are 30 seconds for initialization/listing and 300 seconds for tool calls. Every request is wrapped in `tokio::time::timeout`.

On timeout, the client marks the connection poisoned and closes the transport. This is intentionally conservative: a stale response may still arrive later, and the milestone 2 sequential loop has no safe way to resynchronize.

Transport close is bounded. The stdio transport closes stdin and waits briefly; if the child does not exit, it kills the child so timeout cleanup cannot block forever.

The timeout unit test uses a tiny real duration instead of Tokio paused time so the dependency list can stay exactly as milestone 2 specifies, without adding Tokio's `test-util` feature.

## Server-Initiated Requests

While waiting for a response, the client keeps handling interleaved messages:

- Notifications are logged and ignored.
- `ping` requests receive an empty result.
- Unknown requests receive JSON-RPC `-32601` method not found.

This is deliberately minimal. Milestone 4 replaces the unknown-request branch with gateway routing for elicitation and related server-initiated primitives.

## Message Loop Limitation

Milestone 2 uses a sequential request/response loop behind public `&self` methods. Internally a mutex serializes access to the transport. That is correct for tools-only baseline work, but it is not the final gateway architecture.

Milestone 4 should replace the internals with a background reader task, a pending-request map, and an inbound server-request channel. The public surface should not need to change.
