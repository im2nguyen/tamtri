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

Implemented gateway events include server connection, tool routing, credential injection by reference, progress, logging, cancellation, downstream errors, and elicitation request/resolution. These events write `events.jsonl` receipts without storing secret values.

## Elicitation (Milestone 6)

Downstream servers may call `elicitation/create` while a `tools/call` is pending. The gateway advertises elicitation to downstream clients, emits UI events, waits for the shell response, and returns `accept`, `decline`, or `cancel` to the server.

- **Form mode** renders a native SwiftUI card for structured, non-secret answers.
- **URL mode** renders a consent card with exact destination origin and path (query stripped in audit logs), then opens the system browser only after explicit approval.
- Shared URL validation lives in `core/src/mcp/url_handoff.rs`. Non-HTTPS URLs are rejected except loopback OAuth callbacks. Userinfo URLs are rejected.

## OAuth for remote HTTP servers (Milestone 6 scaffold)

`config.json` may attach an `oauth` block to streamable HTTP gateway servers. Core implements authorization code + PKCE in `core/src/mcp/oauth.rs`. Resolved bearer tokens inject into outbound HTTP via `CredentialResolver` using `token_ref` references only in the vault. The macOS shell owns the loopback callback listener and persists token bundles in Keychain.

On macOS, the shell stores OAuth bundles in Keychain (service `tamtri.gateway`, account = `token_ref`). On startup it loads those bundles into core's in-memory credential store. When core silently refreshes an expiring bundle during an HTTP tool call, it emits a `gateway_credential_updated` UI event; the shell responds by exporting the updated bundle from core and persisting it back to Keychain. Token values never appear in `config.json` or `events.jsonl`.

The `tamtri-gateway-stdio` helper forwards stdio JSON-RPC frames to that endpoint for agents that require stdio MCP server refs. Development builds discover it via `TAMTRI_GATEWAY_STDIO_HELPER`, next to the current executable, or under `target/debug`; release packaging still needs to bundle it beside the signed app executable.

## Apps, Tasks, and Roots (Milestone 7)

### Apps

Downstream MCP servers may declare `ui://` templates with `text/html;profile=mcp-app` MIME type and CSP `connectDomains`. The gateway indexes declared templates at `tools/list` and loads HTML on `AppReturned` events.

- **Artifacts** stay no-network with `script-src 'none'` and no host bridge.
- **Apps** use `WebContentPolicy.app` with declared origins only and a consent-gated JSON-RPC bridge (`tamtriAppBridge`). App-initiated tool calls route through the same audit path as harness tool calls.
- Persisted `app_resource` blocks carry `server_id`, `template_ref`, `uri`, `state`, and optional `origin_tool_call_id`.

### Tasks

Long-running downstream work surfaces as `task_started`, `task_updated`, and `task_completed` gateway events. The core polls task status (RC subscribe is gated behind capability checks). Live task cards allow cancellation when supported; mid-task input reuses the M6 elicitation path. Final state persists as `task_ref` blocks; detailed updates land in `events.jsonl`.

### Roots

Per-conversation roots live in `meta.json` as portable refs (`id`, `name`, `uri`, `kind`, `scope`). macOS stores security-scoped bookmarks in Application Support (`~/Library/Application Support/tamtri/root-bookmarks/<conversation_id>/<root_id>.bookmark`); bookmark bytes never enter the vault. The gateway answers downstream `roots/list` and enforces path scope for tools that validate paths.

### Capability gates

`TamtriFeatureSupport::current()` enables Apps, Tasks, and Roots end-to-end. RC extension identifiers (`io.modelcontextprotocol/apps`, `/tasks`, `/roots`) parse without breaking 2025-11-25 servers. Settings shows per-server capability badges after **Probe capabilities**. **Sampling is always declined** — tamtri is not the model.

See `docs/testing-m7.md` for a manual demo script and fixture wiring.
