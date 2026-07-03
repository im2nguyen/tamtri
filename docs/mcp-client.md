# MCP Client And Gateway

Milestone 4 promotes the M2 MCP client onto the shared JSON-RPC dispatch loop introduced for ACP in M3. The public client surface stays `&self`, but requests now correlate through a background reader and pending-request map, so unrelated calls can be in flight concurrently.

## Client Surface

`McpClient` supports:

- `connect_stdio(command, args, env, config)`
- `connect_http(endpoint, headers, config)`
- `list_tools()` / `call_tool(name, arguments, meta)`
- `list_resources()` / `read_resource(uri)`
- `list_prompts()` / `get_prompt(name, arguments)`
- `close()`

Tool, resource, and prompt listing follows `nextCursor` pagination until exhausted. Result payloads preserve raw JSON where the MCP spec is still moving, especially tool content, resource contents, and prompt messages.

## Dispatch Loop

The client uses `rpc::dispatch::RpcConnection`.

- Responses route to pending requests by `RequestId`.
- Server requests and notifications go to an inbound driver while calls are pending.
- `ping` receives an empty result.
- Unknown server requests receive JSON-RPC `-32601`.
- Progress, logging, and cancellation notifications are accepted by the driver and fan out as gateway UI events plus `events.jsonl` receipts.
- A timed-out request removes its pending entry. Late responses are ignored by the dispatcher.

## Transports

### Stdio

`StdioTransport` uses newline-delimited JSON-RPC over child stdin/stdout. Stderr drains to `tracing::debug!` and is never parsed as protocol.

Child environments are scrubbed. The transport starts from an empty environment, preserves only `PATH`, `HOME`, `TMPDIR`, and `LANG`, then applies explicit env pairs. Gateway credential injection appends resolved values deliberately.

### Streamable HTTP

`HttpTransport` uses `reqwest` with Rustls, redirects disabled, and explicit headers only. It accepts normal JSON responses and `text/event-stream` responses containing JSON-RPC messages in `data:` frames. It preserves a server-provided `Mcp-Session-Id` header and sends it on later requests.

HTTP tests use a local `tokio::net::TcpListener` fixture. In restricted sandboxes they require loopback-network approval; they do not call the internet.

## Gateway Registry

The vault-level `config.json` contains the gateway registry:

- default MCP call timeout
- enabled downstream servers
- server scope (`system`, `user`, `project`)
- stdio or streamable HTTP transport config
- credential references only

Inline credential values are rejected by strict config deserialization. Secret values live outside the vault. The macOS shell saves entered values to Keychain and feeds them to core's in-memory resolver for the current app session.

## Gateway Router

`McpGateway` connects to enabled downstream servers and exposes stable gateway tool and prompt names:

```text
<server_id>__<downstream_tool_name>
```

Resources are exposed with Tamtri-owned URIs:

```text
tamtri://gateway/<server_id>/<resource_slug>
```

The gateway keeps route tables from exposed names/URIs to the original downstream `{server_id, original_name_or_uri}`. The UI can show both the gateway identifier and the original downstream provenance.

## Agent-Facing Endpoint

`core/src/mcp/server.rs` implements the MCP server surface that an agent sees: `initialize`, `ping`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, and `prompts/get`.

`core/src/mcp/endpoint.rs` starts a run-scoped loopback HTTP endpoint on `127.0.0.1:0`. It accepts JSON-RPC POSTs for normal request/response MCP calls and a GET SSE stream that forwards gateway progress/logging/cancellation as JSON-RPC notifications. `TamtriCore` creates one endpoint for each run and passes exactly one MCP server ref named `Tamtri Gateway` into ACP `session/new`. The endpoint shuts down when the run ends or is cancelled.

Implemented gateway events include server connection, tool routing, credential injection by reference, progress, logging, cancellation, and downstream errors. These events write `events.jsonl` receipts without storing secret values.

The `tamtri-gateway-stdio` helper forwards stdio JSON-RPC frames to that endpoint for agents that require stdio MCP server refs. Development builds discover it via `TAMTRI_GATEWAY_STDIO_HELPER`, next to the current executable, or under `target/debug`; release packaging still needs to bundle it beside the signed app executable.
