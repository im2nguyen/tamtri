# MCP Client And Gateway

## M2 baseline (stdio tools client)

Milestone 2 shipped a sequential stdio MCP client before the shared `rpc::dispatch` loop arrived in Milestone 3 (ACP) and Milestone 4 (gateway promotion). The baseline behavior still applies to every RPC connection:

- **Timeouts.** `McpClientConfig` carries `init_timeout` (default 30s for `initialize`, `notifications/initialized`, and list calls) and `call_timeout` (default 300s for `tools/call`). Every request is wrapped in `tokio::time::timeout`.
- **Poison on timeout.** When a request times out, `RpcHandle` sets a poison flag, drops the pending entry, and sends `RpcCommand::Poison` to the reader task. The next call on that handle returns `CoreError::TransportClosed`. The timed-out method returns `CoreError::Timeout { method }`. Late responses on the wire are ignored. Callers must reconnect; there is no resync after a stale reply may be in flight.
- **Protocol version.** The client sends `MCP_PROTOCOL_VERSION` (`2025-11-25`). If the server negotiates a different version, core logs a warning and continues. Handshake failure is a JSON-RPC error, not a dedicated mismatch variant.
- **Server-initiated requests while pending.** Notifications are logged and skipped. `ping` gets an empty `{}` result. Any other server request gets JSON-RPC `-32601` so compliant servers do not hang.
- **Message classification.** `IncomingMessage::from_line` classifies by field presence. Serde untagged fallthrough is not used on the read path.
- **Layout.** JSON-RPC types live in `core/src/rpc/jsonrpc.rs` (re-exported from `core/src/mcp/jsonrpc.rs`). Stdio framing is in `core/src/rpc/transport/stdio.rs`. The M2 client loop lived in `core/src/mcp/client.rs` until Milestone 4 moved it onto `rpc::dispatch::RpcConnection`.

**Poison vs multiplex (do not conflate).** Poison-on-timeout is a connection-lifecycle rule: after `CoreError::Timeout`, the handle is poisoned, late wire responses are ignored, and the next call returns `CoreError::TransportClosed`. Callers reconnect; there is no in-band resync. Multiplexed dispatch (M4) is a concurrency upgrade on the same loop: multiple in-flight requests correlate by `RequestId` through a background reader. Multiplexing does not relax poison-on-timeout. A timed-out request still poisons the handle even though other calls could theoretically share the transport.

**Gateway eviction.** `McpGateway` evicts cached downstream `McpClient`s on `CoreError::Timeout` or `CoreError::TransportClosed` (`evict_client` in `core/src/mcp/gateway.rs`), then reconnects on the next `list_tools` / `call_tool`. See `gateway_list_tools_recovers_after_timeout` and `gateway_evicts_client_on_transport_closed` in `core/tests/gateway.rs`.

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

## OAuth for remote HTTP servers (Milestone 6)

`config.json` may attach an `oauth` block to streamable HTTP gateway servers. Core implements authorization code + PKCE in `core/src/mcp/oauth.rs`. Resolved bearer tokens inject into outbound HTTP via `CredentialResolver` using `token_ref` references only in the vault. The macOS shell owns the loopback callback listener and persists token bundles in Keychain.

On macOS, the shell stores OAuth bundles and static gateway secrets in Keychain (service `tamtri.gateway`, account = credential reference). On launch it reloads every configured `credential_ref` and `token_ref` from Keychain into core's in-memory credential store. When core silently refreshes an expiring OAuth bundle during an HTTP tool call, it emits a `gateway_credential_updated` UI event; the shell exports the updated bundle from core and persists it back to Keychain. Token and secret values never appear in `config.json` or `events.jsonl`.

If the app quits while elicitation cards are open, the shell calls `prepare_for_app_quit` so core cancels pending downstream elicitations and writes `elicitation_resolved` receipts with action `cancel`.

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

See [docs/testing/](testing/README.md) for manual verification guides (Apps, Tasks, Roots, capabilities).
