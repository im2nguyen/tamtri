# Milestone 2: MCP Client Baseline

Second Claude Code session. Build an MCP client in `tamtri-core` that connects to a local MCP server over stdio, completes the lifecycle handshake, lists tools, and calls a tool end to end. This is the foundation of tamtri's whole thesis, so the protocol layer must be correct and the message-loop design must leave room for server-initiated requests (elicitation, sampling) in later milestones.

Scope is tools only. No elicitation, Apps, Tasks, Sampling, or Roots yet. No HTTP transport. No FFI. No UI.

## Definition of done

- `McpClient` connects to a local stdio MCP server and completes `initialize` + `notifications/initialized`.
- `list_tools()` returns the server's tools.
- `call_tool(name, args)` executes and returns a result.
- A `CallToolResult` converts cleanly into a `ContentBlock::ToolResult` from Milestone 1.
- Every request is bounded by a timeout. A hung server surfaces `CoreError::Timeout`; the client never blocks forever.
- The client answers server `ping` requests and replies method-not-found to any other server-initiated request, so no compliant server blocks waiting on us.
- Incoming messages are classified by field presence, never by serde untagged fallthrough. A server-initiated request is never misread as a response.
- Hermetic unit tests (mock transport) plus one real-subprocess integration test, all green.
- `cargo clippy` clean. No `unwrap()` / `expect()` in non-test code.
- Protocol version, negotiated capabilities, timeout policy, ping handling, and the message-loop limitation are documented in `/docs/mcp-client.md`.

## Architectural note: the core becomes async

Milestone 1 was synchronous. The MCP client spawns subprocesses and reads a stream, so this milestone adopts an async runtime. Use tokio. Every downstream piece (harness adapters return async streams) already assumed this. Keep the async surface inside the core. The FFI bridge to Swift will convert async to the shell's expectations in a later milestone. That conversion is not this milestone's problem, but do not leak `tokio` types across public API boundaries you expect the FFI layer to call. Return your own types and `Result`.

## New dependencies (core `Cargo.toml`)

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "process", "io-util", "sync", "time"] }
async-trait = "0.1"
tracing = "0.1"

[dev-dependencies]
tracing-subscriber = "0.3"
```

## MCP facts that matter (get these right)

- MCP is JSON-RPC 2.0. Requests have an `id`; notifications do not.
- All MCP JSON field names are camelCase: `protocolVersion`, `clientInfo`, `serverInfo`, `inputSchema`, `isError`, `nextCursor`, `structuredContent`. Use `#[serde(rename_all = "camelCase")]` on every protocol struct.
- Protocol version target: `2025-11-25`. Gate any `2026-07-28` RC behavior behind capability checks later. Define `pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";`.
- stdio transport framing: newline-delimited JSON. One JSON-RPC message per line on stdin/stdout. Server logs go to stderr; capture and forward to `tracing`, never parse stderr as protocol.
- Lifecycle: client sends `initialize` (request), server replies, client sends `notifications/initialized` (notification), then normal operation.

## Task 1: JSON-RPC layer (`mcp/jsonrpc.rs`)

Minimal, correct JSON-RPC 2.0 types. Derive serde on all.

```rust
// JSON-RPC ids may be numbers or strings. We always EMIT numbers, but a server
// chooses its own ids for server-initiated requests, so we must PARSE both.
// An i64-only id would fail to parse a string-id server request and break the
// exact seam later milestones depend on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

pub struct JsonRpcRequest {
    pub jsonrpc: String,   // always "2.0"; set on construction, validated on parse
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

pub struct JsonRpcNotification {
    pub jsonrpc: String,   // "2.0"
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// Well-known codes we emit.
pub const METHOD_NOT_FOUND: i64 = -32601;
```

Note `jsonrpc` is `String`, not `&'static str`: these types sit on the read path too, and serde cannot deserialize into a `&'static str`. Constructors set `"2.0"`; parsing rejects anything else.

The incoming-message enum for the read path, since a server may send responses, notifications, or (later) server-initiated requests:

```rust
pub enum IncomingMessage {
    Response(JsonRpcResponse),
    Request(JsonRpcRequest),        // server-initiated; ping is answered this milestone, the rest in milestone 4
    Notification(JsonRpcNotification),
}

impl IncomingMessage {
    /// Classify one wire message by field presence:
    ///   has "method" and "id"          -> Request (server-initiated)
    ///   has "method", no "id"          -> Notification
    ///   has "id", no "method"          -> Response; must carry EXACTLY one of
    ///                                     "result" / "error", else CoreError::Protocol
    ///   anything else                  -> CoreError::Protocol
    pub fn from_line(line: &str) -> Result<IncomingMessage>;
}
```

**Do not derive `#[serde(untagged)]` on `IncomingMessage`.** Untagged tries variants in order and ignores unknown fields; because `result` and `error` are both `Option`, a server request `{jsonrpc, id, method, params}` deserializes "successfully" as a `Response` with both fields `None`, silently swallowing the very messages the later milestones need. Classify by field presence in `from_line` instead, and unit-test the classification directly.

Parsing (and minimally answering) server-initiated requests now is what keeps Milestone 4 from being a rewrite.

## Task 2: Transport (`mcp/transport/`)

Trait (`transport/mod.rs`):

```rust
#[async_trait]
pub trait Transport: Send {
    async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()>;
    async fn send_notification(&mut self, note: &JsonRpcNotification) -> Result<()>;
    async fn send_response(&mut self, resp: &JsonRpcResponse) -> Result<()>;   // replies to server-initiated requests (ping now, more in M4)
    async fn recv(&mut self) -> Result<IncomingMessage>;
    async fn close(&mut self) -> Result<()>;
}
```

`StdioTransport` (`transport/stdio.rs`):
- Spawn with `tokio::process::Command`, piping stdin, stdout, stderr.
- Child environment is scrubbed by default: start from a clean env, set `PATH`, `HOME`, `TMPDIR`, `LANG`, then apply the explicitly provided `env` pairs. Never inherit the full parent environment; the gateway story is credential hygiene, and leaking the host env to every downstream server contradicts it.
- Write each outgoing message as compact JSON followed by `\n` to child stdin.
- Read incoming with a `BufReader` over child stdout, one line per message, classify via `IncomingMessage::from_line`.
- Spawn a task that drains stderr into `tracing::debug!` so a chatty server never blocks or corrupts the protocol stream.
- `close` shuts stdin and waits for the child to exit.

Keep transport dumb: it frames and moves bytes. It does not correlate ids or understand MCP semantics.

## Task 3: MCP protocol types (`mcp/protocol.rs`)

All with `#[serde(rename_all = "camelCase")]`.

```rust
pub struct Implementation { pub name: String, pub version: String }

pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
}

// Minimal for this milestone. Expand in later milestones (roots, sampling, elicitation).
pub struct ClientCapabilities { /* empty object for now, serialize as {} */ }

pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    // resources, prompts, etc. added later
}

pub struct ToolsCapability { pub list_changed: Option<bool> }

pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,   // JSON Schema
}

pub struct ListToolsParams { pub cursor: Option<String> }
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    pub next_cursor: Option<String>,
}

pub struct CallToolParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

pub struct CallToolResult {
    pub content: Vec<serde_json::Value>,        // text/image/resource blocks; keep raw for robustness
    pub is_error: Option<bool>,
    pub structured_content: Option<serde_json::Value>,
}
```

Keeping `content` as raw `Value`s is intentional. The tool-content shape is still evolving in the spec, and Milestone 1's `ContentBlock::ToolResult.output` is already a `Value`, so this maps straight through. Add typed text extractors as helpers if convenient, but do not block on modeling every content variant.

## Task 4: McpClient (`mcp/client.rs`)

```rust
pub struct McpClientConfig {
    pub init_timeout: Duration,      // default 30s: initialize, notifications/initialized, tools/list
    pub call_timeout: Duration,      // default 300s: tools/call (tools legitimately run long pre-Tasks)
}
```

Timeouts are user-configurable, not compile-time constants. `McpClientConfig::default()` carries the 30s/300s defaults; the values come from the vault-level app config once settings land (global `mcp.call_timeout_secs`, plus an optional per-server override on each entry in the gateway's downstream-server registry, since some servers are known-slow). This milestone just takes the struct; the config-file plumbing arrives with the gateway registry in milestone 4.

```rust
pub struct McpClient {
    inner: tokio::sync::Mutex<ClientInner>,   // transport + next_id; see API-shape note below
    config: McpClientConfig,
    server_info: Option<Implementation>,
    server_capabilities: Option<ServerCapabilities>,
}

impl McpClient {
    pub async fn connect_stdio(command: &str, args: &[String], env: &[(String, String)], config: McpClientConfig) -> Result<Self>;
    pub async fn initialize(&mut self) -> Result<InitializeResult>;   // called once by connect_stdio; &mut is fine here
    pub async fn list_tools(&self) -> Result<Vec<Tool>>;              // follow nextCursor to completion
    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<CallToolResult>;
    pub async fn close(self) -> Result<()>;

    pub fn server_info(&self) -> Option<&Implementation>;
    pub fn server_capabilities(&self) -> Option<&ServerCapabilities>;
}
```

**API shape matters more than the internals.** Post-init methods take `&self`, not `&mut self`, with the transport behind a `tokio::sync::Mutex` this milestone. The mutex serializes calls (correct for M2's sequential design), and in milestone 4 the internals swap to a background reader task + pending-request map + request channel without changing a single public signature. If the surface were `&mut self`, every M4 caller would break.

Behavior:
- `connect_stdio` builds a `StdioTransport`, then calls `initialize`, then sends the `notifications/initialized` notification.
- `initialize` sends `MCP_PROTOCOL_VERSION`. If the server returns a different version, accept it, record it, and log a warning when it differs from ours. Hard-fail only if the server errors the handshake.
- `list_tools` loops on `nextCursor` until exhausted, concatenating pages.
- `call_tool` sends `tools/call`, returns the `CallToolResult`.
- **Every request is wrapped in `tokio::time::timeout`** (`init_timeout` for lifecycle/list, `call_timeout` for tool calls). On timeout, return `CoreError::Timeout` and treat the connection as poisoned: the next message on the wire may be the stale reply, so the client closes rather than resynchronize.

While waiting for a matching response, incoming messages are handled, never dropped:
- `Notification` — log at debug, keep reading.
- Server `Request` with method `"ping"` — reply immediately with an empty `{}` result via `send_response`, keep reading. Servers may ping mid-session and block on the answer; silence hangs them.
- Any other server `Request` — reply with JSON-RPC error `-32601` (method not found), log at warn, keep reading. An error reply lets a compliant server proceed; silence would hang it. Milestone 4 replaces this branch with real routing (elicitation and friends).

Request/response correlation (baseline, and its seam): this milestone does sequential request/response. Send a request, then read from the transport until a `Response` with the matching `id` arrives, handling interleaved messages per the rules above. This is correct for tools because we drive the calls one at a time.

**The multiplexed dispatch loop lands in milestone 4, not later.** The gateway makes tamtri the MCP server for the agent, and ACP agents issue parallel tool calls; the gateway must fan those out to downstream clients concurrently. Two concurrent calls to the same downstream server through a sequential client would serialize behind the mutex at best. So milestone 4 promotes this read path to a background reader task plus a map of pending `RequestId` to oneshot sender, and a channel for inbound server requests. Isolate correlation logic in one place now (one function that owns "read until my reply") so that upgrade is local, not a rewrite.

## Task 5: Bridge to the conversation model (`mcp/bridge.rs`)

A small, pure mapping so tool activity lands in the Milestone 1 model:

- `fn tool_call_block(id: &str, name: &str, arguments: &serde_json::Value) -> ContentBlock` returns `ContentBlock::ToolCall`.
- `fn tool_result_block(call_id: &str, result: &CallToolResult) -> ContentBlock` returns `ContentBlock::ToolResult` whose `output` packs `content`, `is_error`, and `structured_content` into one JSON value.

No harness exists yet, so nothing calls these in a loop this milestone. They exist to prove the MCP result maps onto the canonical format, and they are unit-tested.

## Task 6: Test fixture + tests

Two layers. Keep the default test run hermetic and offline.

Mock transport (unit tests): a `MockTransport` implementing `Transport` with a scripted queue of `IncomingMessage`s and a captured log of what was sent. Use it to test the client's lifecycle, id correlation, pagination, and error handling without any process.

Fixture server (integration test): a tiny mock MCP server binary in the workspace (for example `fixtures/mock-mcp-server`) that speaks stdio JSON-RPC, answers `initialize`, exposes one echo tool via `tools/list`, and returns the echoed input on `tools/call`. One integration test spawns it with a real `StdioTransport` and drives the full path. This proves the subprocess wiring without needing node or network.

Optional, `#[ignore]` by default: a test that spawns `npx @modelcontextprotocol/server-everything` for anyone who wants to check against a reference server. Never in the default run.

Enumerated tests:
1. `initialize_handshake` — client sends correct `initialize` params (version, clientInfo) and stores `serverInfo` from the reply.
2. `sends_initialized_notification` — after initialize, an `notifications/initialized` notification is sent.
3. `list_tools_single_page` — returns tools when `nextCursor` is absent.
4. `list_tools_paginates` — follows `nextCursor` across two pages and concatenates.
5. `call_tool_returns_result` — parses `content` and `isError`.
6. `call_tool_error_flag` — `isError: true` surfaces as an error result the caller can detect.
7. `ignores_interleaved_notification` — a notification arriving before the matching response does not break correlation.
8. `jsonrpc_error_maps_to_core_error` — a JSON-RPC error response becomes a typed `CoreError`.
9. `result_maps_to_tool_result_block` — `tool_result_block` produces a `ContentBlock::ToolResult` with the expected packed `output`.
10. `classifies_by_field_presence` — `{"jsonrpc":"2.0","id":"srv-1","method":"roots/list"}` parses as `Request` (string id intact), `{"jsonrpc":"2.0","method":"notifications/progress"}` as `Notification`, `{"jsonrpc":"2.0","id":1,"result":{}}` as `Response`. The request case is the regression guard against untagged fallthrough.
11. `response_with_result_and_error_is_protocol_error` — a response carrying both (or neither) of `result`/`error` returns `CoreError::Protocol`.
12. `answers_ping_while_waiting` — mock transport queues a server `ping` request before the matching response; assert an empty-result response with the ping's id was sent and the original call still completes.
13. `unknown_server_request_gets_method_not_found` — a non-ping server request receives a `-32601` error response and does not break correlation.
14. `request_times_out` — mock transport that never yields; with `tokio::time::pause`, assert `CoreError::Timeout` at the configured budget.
15. `integration_echo_tool` — real subprocess: connect to the fixture server, list tools, call echo, assert the round-tripped payload.

## Errors (`error.rs`, extend Milestone 1)

Add variants:

```rust
#[error("mcp protocol error: {0}")]
Protocol(String),
#[error("json-rpc error {code}: {message}")]
JsonRpc { code: i64, message: String },
#[error("transport closed")]
TransportClosed,
#[error("protocol version mismatch: server {0}")]
VersionMismatch(String),
#[error("request timed out: {method}")]
Timeout { method: String },
```

## Out of scope this milestone (do not build yet)

Elicitation, MCP Apps, Tasks, Sampling, Roots. HTTP / streamable transport. The multiplexed dispatch loop (design the seam here; the loop itself is built in milestone 4, where the gateway's concurrent fan-out requires it). FFI / UniFFI. Any SwiftUI. Harness adapters.

## Kickoff prompt for Claude Code

> Read CLAUDE.md and milestone-2.md. Implement Milestone 2 in tamtri-core. Start with Task 1 (JSON-RPC layer) and Task 2 (Transport trait + StdioTransport), then stop and show me the transport design and the incoming-message enum before building the client, since the message-loop seam matters for later milestones.
