# Milestone 4: Gateway + Full MCP Client

Fourth build session. tamtri takes its place in the middle: the agent sees tamtri as its MCP server, and tamtri connects to the real downstream MCP servers as a client. This is the largest core milestone because it turns the M2 downstream client and the M3 shared JSON-RPC loop into the gateway that the product thesis depends on.

Two boundaries shape the work. First, the gateway owns the capability plane. Rich MCP primitives must flow through tamtri, not disappear inside a harness. Second, the core stays platform-agnostic. The macOS shell may read and write keychain entries, but Rust core only receives credential references from config and resolved secret values in memory.

## Definition of done

- `AcpAdapter` passes exactly one tamtri-owned MCP server entry in `session/new`, and the agent can call downstream tools through that gateway.
- The existing `McpClient` public API remains stable, but its internals run on the shared `rpc::dispatch::RpcConnection`: concurrent requests correlate correctly, late responses after timeouts are ignored, and inbound server requests/notifications keep flowing while calls are pending.
- tamtri connects to both a local stdio MCP server and a remote streamable HTTP MCP server. The stdio path still uses scrubbed environments.
- The gateway proxies tools, resources, and prompts with pagination. Tool calls carry progress, cancellation, and logging through the gateway so long-running work is visible in the live tool card and recorded in `events.jsonl`.
- A vault-level gateway server registry exists. It stores server definitions, scopes, timeout overrides, and credential references only. It never stores secret values.
- Static credential injection works for v1: env vars for stdio servers, headers for streamable HTTP servers. The audit log records that a credential reference was injected for a server, never the value.
- A minimal settings surface shows enabled gateway servers and distinguishes tamtri gateway tools from agent-native harness tools. If the selected harness cannot expose native tool details yet, the UI says so plainly.
- Fork-into-harness is usable from the app: fork a conversation, pick a different harness/model, open the new thread, and seed it with the parent transcript. No mid-thread switch is introduced.
- Hermetic tests cover the gateway, registry, credential redaction, HTTP transport framing, and concurrent MCP calls. `cargo test` and `cargo clippy` are clean.
- `/docs/mcp-client.md`, `/docs/events-format.md`, and `/docs/vault-format.md` are updated for the new gateway, registry, and transport behavior.

## Implementation checkpoints and gaps

Current repo status:

- `McpClient` now uses the shared `rpc::dispatch::RpcConnection` internally while preserving the public `&self` API shape.
- The MCP client supports tools, resources, prompts, pagination, stdio, and streamable HTTP. HTTP JSON and SSE behavior is covered by a local loopback fixture.
- A vault-level `config.json` registry exists in `core/src/config.rs` with atomic writes, duplicate-id validation, enabled-server filtering, and strict credential-reference-only deserialization.
- `McpGateway` connects to enabled downstream servers, injects credentials by reference, aggregates tools/resources/prompts, exposes stable gateway names and resource URIs, routes calls/reads/gets, and emits in-memory gateway events.
- An in-process agent-facing gateway server surface exists in `core/src/mcp/server.rs` for `initialize`, `ping`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, and `prompts/get`.
- `core/src/mcp/endpoint.rs` starts a run-scoped loopback HTTP MCP endpoint. It supports request/response POSTs plus an SSE GET stream for upstream progress/logging/cancellation notifications. `TamtriCore` creates one endpoint per run, passes exactly that single Tamtri gateway server to `AcpAdapter` in `session/new`, and shuts the endpoint down when the run ends.
- Gateway server connection, tool routing, credential-injection, progress, logging, cancellation, and downstream-error events write local receipts to `events.jsonl` and emit UI events.
- The Swift shell has a minimal settings panel that separates Tamtri gateway servers from agent-native tools, shows server status/scope/transport, shows credential-reference presence, and saves entered values to Keychain before feeding the in-memory core resolver.
- The Swift shell has a "Fork Into" affordance that forks the current conversation into a selected harness/model.
- `tamtri-gateway-stdio` is a small stdio forwarding helper binary that can bridge ACP agents requiring stdio MCP server refs to the same run-scoped loopback endpoint. `TamtriCore` discovers it next to the app/test binary, through `TAMTRI_GATEWAY_STDIO_HELPER`, or under `target/debug`, and uses it as the default ACP-facing MCP ref when available.

Remaining packaging follow-up (deferred to Milestone 9):

- Bundle `tamtri-gateway-stdio` beside the signed macOS executable in release packaging. Development and tests discover `target/debug/tamtri-gateway-stdio`; copying the helper into the app bundle, DMG layout, and notarized release packaging are M9 scope, not M4.

## Architecture note: gateway topology

Implement gateway logic once, independent of how the agent connects to it:

```rust
pub struct McpGateway {
    registry: GatewayRegistry,
    clients: GatewayClientPool,
    credentials: Arc<dyn CredentialResolver>,
    events: GatewayEventSink,
}
```

The agent-facing side is an MCP server. Prefer a loopback streamable HTTP endpoint when the ACP agent accepts HTTP MCP servers. For agents that only accept stdio server configs, add a tiny stdio helper that forwards JSON-RPC frames to the same in-process gateway endpoint. Do not duplicate gateway routing logic in the helper.

The downstream side uses `McpClient` instances keyed by server id. Stdio and streamable HTTP are transports behind the same client surface. The gateway fans out concurrently; do not serialize unrelated tool calls behind a global mutex.

## Task 1: Promote `McpClient` to `RpcConnection`

Replace the M2 sequential request/read loop with the shared M3 RPC dispatch loop.

Target shape:

```rust
pub struct McpClient {
    handle: RpcHandle,
    inbound: InboundDriver,
    config: McpClientConfig,
    server_info: Implementation,
    server_capabilities: ServerCapabilities,
}

impl McpClient {
    pub async fn connect_stdio(command: &str, args: &[String], env: &[(String, String)], config: McpClientConfig) -> Result<Self>;
    pub async fn connect_http(endpoint: Url, headers: HeaderMap, config: McpClientConfig) -> Result<Self>;
    pub async fn list_tools(&self) -> Result<Vec<Tool>>;
    pub async fn call_tool(&self, name: &str, arguments: Value, meta: Option<Value>) -> Result<CallToolResult>;
    pub async fn list_resources(&self) -> Result<Vec<Resource>>;
    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult>;
    pub async fn list_prompts(&self) -> Result<Vec<Prompt>>;
    pub async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult>;
    pub async fn close(self) -> Result<()>;
}
```

Keep the existing `&self` call shape. `McpClient` now owns an inbound driver task that handles server requests and notifications:

- `ping` gets an empty result immediately.
- `notifications/progress` becomes a gateway progress event and, when the upstream agent provided a progress token, is forwarded upstream.
- logging notifications become gateway log events and audit receipts.
- server-initiated requests that belong to later milestones, such as elicitation or sampling, receive honest method-not-found or capability-not-advertised behavior for now. Do not silently drop them.

Timeout behavior keeps the M2 full-poison policy on `RpcConnection`: a timed-out request removes its pending entry, returns `CoreError::Timeout`, and poisons the handle (subsequent calls return `CoreError::TransportClosed`). Late responses on the wire are ignored and logged. Callers must reconnect; there is no resync after a stale reply may be in flight. Initialization timeout still closes the connection because the server is not usable without a completed handshake.

Tests: reuse the dispatch tests from M3 and add MCP-client-level tests for two concurrent `tools/call` requests whose responses arrive out of order, inbound progress while calls are pending, timeout poison with late response ignored, and ping handling through the inbound driver.

## Task 2: Streamable HTTP transport

Add streamable HTTP beside stdio under the shared transport layer. The transport moves JSON-RPC messages; it does not know MCP semantics.

Requirements:

- Support MCP streamable HTTP sessions: initialize over HTTP, preserve the server-provided session id/header when present, and close the session cleanly.
- Accept JSON responses and server-sent event streams. Parse only JSON-RPC messages from the stream.
- Apply explicit headers supplied by the gateway after credential resolution. Never inherit process env or global headers.
- Enforce per-request timeout from `McpClientConfig`.
- Reject redirects to a different origin unless a later trusted-domain flow explicitly allows them. M4 has no OAuth/browser handoff yet.
- Keep logs redacted. URLs may be logged without query strings; headers are never logged with values.

Dependencies are acceptable here if they stay boring and auditable. Prefer `reqwest` or `hyper` with Rustls rather than an OpenSSL dependency unless the existing toolchain forces a different choice. Record the choice in `/docs/mcp-client.md`.

Tests: a hermetic HTTP fixture server that returns a normal JSON response, an SSE response, progress before a result, a session header, and an error status. No internet dependency in the default test run.

## Task 3: Expand MCP protocol types

Extend `mcp/protocol.rs` for the full M4 surface while keeping raw JSON where the spec is still fluid.

Add typed request/result structs for:

- `tools/list` and `tools/call`, preserving the M2 raw `content` and `structuredContent` result shape.
- `resources/list`, `resources/read`, and resource templates if the server advertises them.
- `prompts/list` and `prompts/get`.
- pagination params/results using `cursor` and `nextCursor`.
- progress params, cancellation params, and logging message params.

Capability structs should represent what tamtri honestly supports. Advertise tools/resources/prompts, progress, cancellation, and logging as supported where the MCP version/capabilities allow. Do not advertise elicitation, Apps, Tasks, Roots, or Sampling from the gateway until their milestones.

Tests: serde round trips for every new type, camelCase field names, pagination helpers, and "unknown fields do not break parsing" for evolving result payloads.

## Task 4: Gateway registry and vault config

Introduce a vault-level app config, likely `<vault>/config.json`, if it does not already exist. Keep it legible JSON.

Suggested shape:

```rust
pub struct AppConfig {
    pub default_harness_id: Option<String>,
    pub agent_roster: Vec<AgentLaunchSpec>,
    pub gateway: GatewayConfig,
}

pub struct GatewayConfig {
    pub default_call_timeout_secs: u64,
    pub servers: Vec<GatewayServerConfig>,
}

pub struct GatewayServerConfig {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub scope: GatewayScope,              // System | User | Project
    pub transport: GatewayTransport,
    pub timeout_secs: Option<u64>,
    pub credentials: Vec<CredentialBinding>,
}

pub enum GatewayTransport {
    Stdio { command: String, args: Vec<String>, env: Vec<(String, String)> },
    StreamableHttp { endpoint: String, headers: Vec<(String, String)> },
}

pub struct CredentialBinding {
    pub credential_ref: String,            // keychain lookup key, not a value
    pub target: CredentialTarget,
}

pub enum CredentialTarget {
    EnvVar { name: String },
    Header { name: String, prefix: Option<String> },
}
```

Rules:

- Config may contain credential references and non-secret header/env names only.
- Secret values live in the macOS keychain. The shell resolves them and passes them to the core through an in-memory `CredentialResolver` or equivalent FFI callback.
- The core injects resolved values only when starting a downstream server or building HTTP headers.
- `events.jsonl` records `{server_id, credential_ref, target_kind}` for injection. Never record a value.
- Config writes are atomic like `meta.json`.

Tests: load/save round trip, duplicate server id rejection, disabled servers ignored, credential values rejected if accidentally present in config, and event payload redaction for injection receipts.

## Task 5: Agent-facing MCP server surface

Build the gateway server that the agent connects to.

Server behavior:

1. `initialize`: return tamtri's MCP server info and capabilities for tools/resources/prompts plus progress/cancellation/logging where supported. Decline sampling by omission.
2. `notifications/initialized`: mark the upstream connection ready.
3. `tools/list`: aggregate tools from all enabled downstream servers and expose stable gateway names.
4. `tools/call`: route to the owning downstream server, bridge progress/logging/cancellation, return the downstream result to the agent.
5. `resources/list` / `resources/read`: aggregate and route resources.
6. `prompts/list` / `prompts/get`: aggregate and route prompts.
7. `ping`: answer immediately.

Name routing must be deterministic and collision-safe. Use a gateway-local name at the agent boundary, for example:

```text
<server_id>__<downstream_tool_name>
```

If a downstream name is not safe to expose directly, slug it and add a short hash suffix. Keep an in-memory route table from exposed name to `{server_id, original_name}`. The UI should still show the real server and original downstream tool name.

For resources, rewrite exposed URIs so `resources/read` can be routed back to the right server without guessing. For prompts, use the same collision strategy as tools.

Tests: aggregate list from two servers, collision handling, route table lookup, downstream error propagation, disabled server exclusion, and `ping` while a downstream call is pending.

## Task 6: Gateway proxy events and audit receipts

The gateway must be visible and auditable.

Add gateway-level live events for:

- downstream server connected/disconnected.
- tool routed `{origin_tool_call_id?, server_id, exposed_name, original_name}`.
- progress update.
- log message.
- cancellation requested/completed.
- credential injected by reference.
- downstream error.

These events decorate live UI cards and write local receipts to `events.jsonl`. They do not become transcript `ContentBlock`s yet unless the reducer already has a matching block. The transcript remains the readable conversation; `events.jsonl` is the detailed receipt trail.

Extend `EventKind` as needed. Keep the existing secret rejection guard, and add tests with realistic gateway payloads. Pay special attention to field names: avoid names like `token`, `password`, `secret`, or `api_key` in event payloads so the redaction guard remains strict and simple.

## Task 7: Wire `AcpAdapter` to the gateway

Replace the M3 `mcpServers: []` session config with a single tamtri gateway server.

Flow:

1. When a run starts, create or reuse the gateway instance for the conversation.
2. Start the agent-facing MCP endpoint.
3. Pass that endpoint as the only gateway-owned MCP server in `session/new`.
4. Leave agent-native MCP servers alone. If the harness loads its own project servers, those are shown separately in settings when detectable, but tamtri does not pretend to proxy them.
5. On run end or cancel, close the upstream endpoint and any idle downstream clients.

The gateway should use the conversation's `mcp_servers` refs plus vault-level config to decide which downstream servers are enabled. Forks copy the refs; changing harness/model still requires a fork.

Tests: `session/new` contains one tamtri gateway entry, no secret values appear in ACP params, gateway shuts down on cancel, and a mock ACP agent can call a mock downstream MCP tool through the full path.

## Task 8: Fork-into-harness

Ship the switching model as a real affordance.

Core:

- Ensure `fork_conversation(id, harness_id, model_id)` copies the folder, sets a new id, preserves `forked_from`, updates active harness/model, and leaves the parent untouched.
- Seed the next run with `ContextSeed::FreshTranscript` from the parent transcript.
- Do not carry live MCP state into the fork. Rendered history stays static; new gateway connections are created when the fork runs.

Swift shell:

- Add a "fork into" action from the conversation toolbar or menu.
- Picker shows harness + model from the roster.
- The new conversation opens after creation and displays its fork lineage.

Tests: parent unchanged, child id and folder differ, active harness/model updated, transcript copied, `forked_from` set, and the first run in the fork uses the copied transcript as seed.

## Task 9: Minimal settings and capability UI

Add just enough UI to make the gateway legible.

- Gateway server list: enabled/disabled, display name, transport type, scope, connection status, and last error.
- Tool provenance: separate "tamtri gateway tools" from "agent-native tools." Gateway tools list server, original tool name, and exposed route name. Agent-native tools may be "not exposed by this harness yet" if ACP cannot introspect them.
- Credential status: show whether a required credential reference is present in keychain. Never show the value.
- Timeout setting: global MCP call timeout plus optional per-server override.

This is not the M8 onboarding health screen. Keep it functional and restrained.

## Task 10: Fixtures and tests

Add fixtures that make the gateway testable without network or installed agents:

- `fixtures/mock-mcp-http-server`: streamable HTTP MCP server with tools/resources/prompts, progress, logging, and an error mode.
- Extend `fixtures/mock-mcp-server`: stdio server with the same feature set.
- Extend `fixtures/mock-acp-agent`: after `session/new`, call a tool through the supplied tamtri gateway server.

Enumerated tests:

1. `mcp_client_concurrent_requests_correlate` - two requests to one server, responses out of order.
2. `mcp_client_inbound_progress_while_pending` - progress arrives before the result and reaches the gateway event sink.
3. `mcp_client_timeout_removes_pending` - late response after timeout is ignored and the connection is poisoned.
4. `streamable_http_json_response` - simple request/response over HTTP.
5. `streamable_http_sse_response` - SSE stream yields progress then result.
6. `streamable_http_preserves_session_header` - session id/header is reused after initialize.
7. `registry_round_trip` - config writes and reads without losing scopes, transports, or timeout overrides.
8. `registry_rejects_duplicate_server_ids`.
9. `credential_refs_only_in_config` - config containing an inline secret-like value is rejected.
10. `credential_injection_redacts_events` - audit receipt contains reference and target only.
11. `gateway_tools_list_aggregates_servers` - enabled downstream tools appear with stable exposed names.
12. `gateway_tool_name_collision_is_stable` - two same-named tools route to the correct server.
13. `gateway_tools_call_routes_to_downstream` - agent call reaches the right downstream server and returns result unchanged.
14. `gateway_resources_and_prompts_paginate` - gateway follows downstream `nextCursor` and exposes aggregated results.
15. `gateway_cancellation_receipt` - upstream cancellation is recorded and surfaced; downstream abort forwarding waits for the push-capable gateway transport.
16. `acp_session_new_includes_gateway` - ACP launch params contain tamtri gateway and no raw credentials.
17. `mock_acp_agent_calls_gateway_tool` - full hermetic agent -> gateway -> downstream tool path.
18. `events_jsonl_gateway_receipts` - routing, progress, credential injection, and downstream error receipts are written without secrets.
19. `fork_into_harness_updates_model_and_harness` - fork semantics match the switching model.
20. `settings_gateway_tools_snapshot` - Swift-facing state separates gateway tools from agent-native tools.

Keep any real remote-server test `#[ignore]` by default.

## Out of scope this milestone

Do not build artifact rendering or the sandboxed webview (M5). Do not build elicitation, URL handoff, or OAuth (M6). Static API keys and bearer tokens are enough here; browser auth is later. Do not build Apps, Tasks, Roots, or Sampling (M7). Do not build the harness health onboarding, search, share/import UX, diagnostics bundle, or full accessibility pass (M8). Do not add cloud, accounts, telemetry, or a harness plugin system.

## Kickoff prompt for Claude Code

> Read CLAUDE.md, docs/milestone-3.md, docs/milestone-4.md, docs/tamtri-decisions.md sections 15-18, and docs/mcp-client.md. Implement Milestone 4. Start with Task 1 (move McpClient onto rpc::dispatch) and Task 2 (streamable HTTP transport), then stop and show me the updated client/transport shape plus the gateway topology before building the full proxy. The gateway is now on the hot path for every MCP tool call, so concurrency, credential redaction, and routing names matter most.
