use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::conversation::{ContentBlock, McpServerRef, Message};
use crate::harness::{
    ContextSeed, ConversationContext, Diff, FileChange, HarnessAdapter, HarnessCapabilities,
    HarnessEvent, HarnessRun, ModelInfo, PermissionDetail, PermissionOption, RunCommand,
    RunControl, ToolContent, ToolKind, ToolStatus, TurnEndReason, TurnInput,
};
use crate::rpc::dispatch::{InboundMessage, RpcConnection, RpcHandle};
use crate::rpc::jsonrpc::{JsonRpcError, RequestId};
use crate::rpc::transport::stdio::StdioTransport;
use crate::{CoreError, Result};

const ACP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentLaunchSpec {
    pub id: String,
    pub display_name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub struct AcpAdapter {
    launch: AgentLaunchSpec,
    agent_capabilities: Mutex<Option<Value>>,
}

impl AcpAdapter {
    pub fn new(launch: AgentLaunchSpec) -> Self {
        Self {
            launch,
            agent_capabilities: Mutex::new(None),
        }
    }

    pub fn agent_capabilities(&self) -> Option<Value> {
        self.agent_capabilities
            .lock()
            .ok()
            .and_then(|caps| caps.clone())
    }
}

#[async_trait]
impl HarnessAdapter for AcpAdapter {
    fn id(&self) -> &str {
        &self.launch.id
    }

    fn display_name(&self) -> &str {
        &self.launch.display_name
    }

    fn capabilities(&self) -> HarnessCapabilities {
        HarnessCapabilities {
            streaming: true,
            tools: true,
            permissions: true,
            thinking: true,
        }
    }

    async fn run(&self, ctx: ConversationContext, turn: TurnInput) -> Result<HarnessRun> {
        let transport = StdioTransport::spawn_with_cwd(
            &self.launch.command,
            &self.launch.args,
            &self.launch.env,
            Some(&ctx.working_dir_path),
        )
        .await?;
        let (rpc, inbound) = RpcConnection::start(Box::new(transport));

        let initialize = rpc
            .request(
                "initialize",
                Some(json!({
                    "protocolVersion": 1,
                    "clientInfo": {"name": "tamtri-core", "version": env!("CARGO_PKG_VERSION")},
                    "clientCapabilities": {}
                })),
                ACP_REQUEST_TIMEOUT,
            )
            .await?;
        if let Ok(mut caps) = self.agent_capabilities.lock() {
            *caps = initialize.get("agentCapabilities").cloned();
        }

        let cwd = absolute_cwd(&ctx.working_dir_path)?;
        let session = rpc
            .request(
                "session/new",
                Some(json!({
                    "cwd": cwd,
                    "mcpServers": acp_mcp_servers(&ctx.mcp_servers)
                })),
                ACP_REQUEST_TIMEOUT,
            )
            .await?;
        let session_id = session_id_from(&session)?;

        let (event_tx, event_rx) = mpsc::channel(128);
        let (command_tx, command_rx) = mpsc::channel(32);
        let control = RunControl::new(command_tx);
        let prompt = render_prompt(&ctx.seed, &turn.user_message);
        let harness_id = self.launch.id.clone();
        tokio::spawn(run_prompt_loop(
            rpc, inbound, command_rx, event_tx, prompt, session_id, harness_id,
        ));

        Ok(HarnessRun {
            events: event_rx,
            control,
        })
    }

    async fn available_models(&self) -> Result<Vec<ModelInfo>> {
        if let Some(caps) = self.agent_capabilities() {
            let models = models_from_agent_capabilities(&caps);
            if !models.is_empty() {
                return Ok(models);
            }
        }

        let transport = StdioTransport::spawn(
            &self.launch.command,
            &self.launch.args,
            &self.launch.env,
        )
        .await?;
        let (rpc, _inbound) = RpcConnection::start(Box::new(transport));
        let initialize = rpc
            .request(
                "initialize",
                Some(json!({
                    "protocolVersion": 1,
                    "clientInfo": {"name": "tamtri-core", "version": env!("CARGO_PKG_VERSION")},
                    "clientCapabilities": {}
                })),
                ACP_REQUEST_TIMEOUT,
            )
            .await?;
        if let Ok(mut caps) = self.agent_capabilities.lock() {
            *caps = initialize.get("agentCapabilities").cloned();
        }
        let _ = rpc.close().await;
        Ok(models_from_agent_capabilities(
            &initialize
                .get("agentCapabilities")
                .cloned()
                .unwrap_or(Value::Null),
        ))
    }
}

fn models_from_agent_capabilities(caps: &Value) -> Vec<ModelInfo> {
    caps.get("models")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = string_field(item, &["id", "modelId"]);
                    if id.is_empty() {
                        return None;
                    }
                    let display_name = string_field(item, &["displayName", "display_name", "name"]);
                    Some(ModelInfo {
                        id: id.clone(),
                        display_name: if display_name.is_empty() {
                            id
                        } else {
                            display_name
                        },
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

async fn run_prompt_loop(
    rpc: RpcHandle,
    mut inbound: mpsc::Receiver<InboundMessage>,
    mut command_rx: mpsc::Receiver<RunCommand>,
    event_tx: mpsc::Sender<HarnessEvent>,
    prompt: String,
    session_id: String,
    harness_id: String,
) {
    let (done_tx, mut done_rx) = oneshot::channel();
    let prompt_rpc = rpc.clone();
    let prompt_session_id = session_id.clone();
    tokio::spawn(async move {
        let result = prompt_rpc
            .request(
                "session/prompt",
                Some(json!({
                    "sessionId": prompt_session_id,
                    "prompt": [{"type": "text", "text": prompt}]
                })),
                Duration::from_secs(60 * 60),
            )
            .await;
        let _ = done_tx.send(result);
    });

    let mut permission_waiters: HashMap<String, RequestId> = HashMap::new();

    loop {
        tokio::select! {
            inbound_msg = inbound.recv() => {
                let Some(inbound_msg) = inbound_msg else {
                    let _ = event_tx.send(HarnessEvent::Error { message: "ACP connection closed".to_string() }).await;
                    break;
                };
                handle_inbound(&rpc, inbound_msg, &event_tx, &mut permission_waiters).await;
            }
            command = command_rx.recv() => {
                match command {
                    Some(RunCommand::Cancel) => {
                        let _ = rpc.notify("session/cancel", Some(json!({ "sessionId": session_id }))).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Cancelled }).await;
                        break;
                    }
                    Some(RunCommand::RespondPermission { request_id, option_id }) => {
                        if let Some(id) = permission_waiters.remove(&request_id) {
                            let _ = rpc.respond(id, Ok(json!({ "optionId": option_id }))).await;
                            let _ = event_tx.send(HarnessEvent::PermissionResolved { request_id, option_id }).await;
                        }
                    }
                    None => {}
                }
            }
            done = &mut done_rx => {
                match done {
                    Ok(Ok(value)) => {
                        if let Some(reason) = value.get("stopReason").and_then(Value::as_str) {
                            let _ = event_tx.send(HarnessEvent::TurnEnded { reason: map_stop_reason(reason) }).await;
                        } else {
                            let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::EndTurn }).await;
                        }
                    }
                    Ok(Err(err)) => {
                        let _ = event_tx
                            .send(HarnessEvent::Error {
                                message: format_harness_run_error(&err),
                            })
                            .await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Failed }).await;
                    }
                    Err(_) => {
                        let _ = event_tx.send(HarnessEvent::Error { message: format!("{harness_id} prompt task dropped") }).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Failed }).await;
                    }
                }
                break;
            }
        }
    }

    let _ = rpc.close().await;
}

async fn handle_inbound(
    rpc: &RpcHandle,
    inbound_msg: InboundMessage,
    event_tx: &mpsc::Sender<HarnessEvent>,
    permission_waiters: &mut HashMap<String, RequestId>,
) {
    match inbound_msg {
        InboundMessage::Notification(note) if note.method == "session/update" => {
            if let Some(params) = note.params {
                for event in normalize_update(&params) {
                    let _ = event_tx.send(event).await;
                }
            }
        }
        InboundMessage::Notification(_) => {}
        InboundMessage::Request(req) if req.method == "session/request_permission" => {
            let request_id = req
                .params
                .as_ref()
                .and_then(|params| params.get("requestId"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| request_id_to_string(&req.id));
            if let Some(params) = req.params.as_ref() {
                let event = permission_event(&request_id, params);
                permission_waiters.insert(request_id, req.id);
                let _ = event_tx.send(event).await;
            } else {
                let _ = rpc
                    .respond(
                        req.id,
                        Err(JsonRpcError {
                            code: -32602,
                            message: "missing permission params".to_string(),
                            data: None,
                        }),
                    )
                    .await;
            }
        }
        InboundMessage::Request(req) => {
            let _ = rpc
                .respond(
                    req.id,
                    Err(JsonRpcError {
                        code: -32601,
                        message: "method not found".to_string(),
                        data: None,
                    }),
                )
                .await;
        }
    }
}

fn normalize_update(value: &Value) -> Vec<HarnessEvent> {
    let update = value.get("update").unwrap_or(value);
    let kind = update
        .get("sessionUpdate")
        .or_else(|| update.get("session_update"))
        .or_else(|| update.get("type"))
        .or_else(|| update.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    match kind {
        "agent_message_chunk" => vec![HarnessEvent::TextDelta {
            text: text_field(update),
        }],
        "agent_thought_chunk" => vec![HarnessEvent::ThoughtDelta {
            text: text_field(update),
        }],
        "tool_call" => vec![HarnessEvent::ToolCallStarted {
            id: string_field(update, &["id", "toolCallId"]),
            name: string_field(update, &["name", "toolName"]),
            kind: map_tool_kind(
                update
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("other"),
            ),
            title: string_field(update, &["title", "name"]),
            input: update
                .get("input")
                .or_else(|| update.get("rawInput"))
                .cloned()
                .unwrap_or_else(|| json!({})),
        }],
        "tool_call_update" => tool_update_events(update),
        "terminal_output" => vec![HarnessEvent::TerminalOutput {
            tool_call_id: string_field(update, &["toolCallId", "id"]),
            chunk: text_field(update),
        }],
        "plan" => vec![HarnessEvent::PlanUpdated { steps: Vec::new() }],
        "mode_changed" => vec![HarnessEvent::ModeChanged {
            mode: string_field(update, &["mode"]),
        }],
        _ => Vec::new(),
    }
}

fn tool_update_events(update: &Value) -> Vec<HarnessEvent> {
    let id = string_field(update, &["toolCallId", "id"]);
    let status = map_tool_status(update.get("status").and_then(Value::as_str));
    let content = parse_tool_content(update);
    let mut events = vec![HarnessEvent::ToolCallProgress {
        id: id.clone(),
        status,
        content: content.clone(),
    }];
    for item in content {
        if let ToolContent::Diff { diff } = item {
            events.push(HarnessEvent::FileChanged {
                tool_call_id: id.clone(),
                path: diff.path.clone(),
                change: diff.change.clone(),
                diff,
            });
        }
    }
    events
}

fn parse_tool_content(update: &Value) -> Vec<ToolContent> {
    let mut content = Vec::new();
    if let Some(text) = update.get("text").and_then(Value::as_str) {
        content.push(ToolContent::Text {
            text: text.to_string(),
        });
    }
    if let Some(diff) = update.get("diff").and_then(parse_diff) {
        content.push(ToolContent::Diff { diff });
    }
    if let Some(uri) = update.get("uri").and_then(Value::as_str) {
        content.push(ToolContent::ResourceRef {
            uri: uri.to_string(),
        });
    }
    if content.is_empty() {
        content.push(ToolContent::Json {
            value: update.clone(),
        });
    }
    content
}

fn parse_diff(value: &Value) -> Option<Diff> {
    let path = string_field(value, &["path"]);
    if path.is_empty() {
        return None;
    }
    Some(Diff {
        path,
        change: match value.get("change").and_then(Value::as_str) {
            Some("created") => FileChange::Created,
            Some("deleted") => FileChange::Deleted,
            _ => FileChange::Modified,
        },
        old_text: value
            .get("oldText")
            .or_else(|| value.get("old_text"))
            .and_then(Value::as_str)
            .map(str::to_string),
        new_text: value
            .get("newText")
            .or_else(|| value.get("new_text"))
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn permission_event(request_id: &str, params: &Value) -> HarnessEvent {
    let tool_call = params.get("toolCall").or_else(|| params.get("tool_call"));
    let detail_source = tool_call.unwrap_or(params);
    let detail = if let Some(diff) = detail_source.get("diff").and_then(parse_diff) {
        PermissionDetail::FileEdit { diff }
    } else if let Some(command) = detail_source
        .get("command")
        .or_else(|| {
            detail_source
                .get("rawInput")
                .and_then(|value| value.get("command"))
        })
        .and_then(Value::as_str)
    {
        PermissionDetail::Command {
            command: command.to_string(),
        }
    } else {
        PermissionDetail::Other {
            value: params.clone(),
        }
    };
    let options = params
        .get("options")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| PermissionOption {
                    id: string_field(item, &["id", "optionId"]),
                    label: string_field(item, &["label", "name", "title", "id", "optionId"]),
                })
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                PermissionOption {
                    id: "allow_once".to_string(),
                    label: "Allow once".to_string(),
                },
                PermissionOption {
                    id: "deny".to_string(),
                    label: "Deny".to_string(),
                },
            ]
        });

    HarnessEvent::PermissionRequested {
        request_id: request_id.to_string(),
        action: string_field(params, &["action"]),
        detail,
        options,
    }
}

fn render_prompt(seed: &ContextSeed, user_message: &Message) -> String {
    let mut out = String::new();
    let ContextSeed::FreshTranscript { messages } = seed;
    for message in messages {
        out.push_str(&format!("{:?}: ", message.role));
        out.push_str(&render_message(message));
        out.push('\n');
    }
    out.push_str("User: ");
    out.push_str(&render_message(user_message));
    out
}

fn render_message(message: &Message) -> String {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } | ContentBlock::Thinking { text } => Some(text.clone()),
            ContentBlock::ToolResult { output, .. } => Some(output.to_string()),
            ContentBlock::ToolCall { name, input, .. } => Some(format!("tool {name}: {input}")),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn map_tool_kind(kind: &str) -> ToolKind {
    match kind {
        "read" => ToolKind::Read,
        "edit" => ToolKind::Edit,
        "write" => ToolKind::Write,
        "execute" | "bash" | "command" => ToolKind::Execute,
        "search" => ToolKind::Search,
        "fetch" => ToolKind::Fetch,
        "think" => ToolKind::Think,
        other => ToolKind::Other(other.to_string()),
    }
}

fn map_tool_status(status: Option<&str>) -> ToolStatus {
    match status {
        Some("pending") => ToolStatus::Pending,
        Some("completed") | Some("done") => ToolStatus::Completed,
        Some("failed") | Some("error") => ToolStatus::Failed,
        _ => ToolStatus::InProgress,
    }
}

fn map_stop_reason(reason: &str) -> TurnEndReason {
    match reason {
        "cancelled" => TurnEndReason::Cancelled,
        "max_tokens" => TurnEndReason::MaxTokens,
        "failed" => TurnEndReason::Failed,
        _ => TurnEndReason::EndTurn,
    }
}

fn text_field(value: &Value) -> String {
    if let Some(text) = string_field_opt(value, &["text", "chunk"]) {
        return text;
    }
    if let Some(content) = value.get("content") {
        if let Some(text) = content.as_str() {
            return text.to_string();
        }
        if let Some(text) = content.get("text").and_then(Value::as_str) {
            return text.to_string();
        }
    }
    String::new()
}

fn string_field(value: &Value, keys: &[&str]) -> String {
    string_field_opt(value, keys).unwrap_or_default()
}

fn string_field_opt(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

fn format_harness_run_error(err: &CoreError) -> String {
    match err {
        CoreError::JsonRpc { code: -32603, message } => format!(
            "json-rpc error -32603: {message}. Gateway tools need tamtri-gateway-stdio (rebuild tamtri) and a valid twenty-questions path in Settings → Gateway; disable unreachable servers."
        ),
        _ => err.to_string(),
    }
}

fn absolute_cwd(path: &std::path::Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(absolute.to_string_lossy().to_string())
}

fn acp_mcp_servers(servers: &[McpServerRef]) -> Vec<serde_json::Value> {
    servers
        .iter()
        .map(|server| match server.transport.as_str() {
            "stdio" => {
                let (command, args) = split_stdio_endpoint(&server.endpoint);
                json!({
                    "type": "stdio",
                    "name": server.name,
                    "command": command,
                    "args": args,
                    "env": []
                })
            }
            "http" | "streamable_http" => json!({
                "type": "http",
                "name": server.name,
                "url": server.endpoint,
                "headers": []
            }),
            "sse" => json!({
                "type": "sse",
                "name": server.name,
                "url": server.endpoint,
                "headers": []
            }),
            other => json!({
                "type": other,
                "name": server.name,
                "url": server.endpoint
            }),
        })
        .collect()
}

fn split_stdio_endpoint(endpoint: &str) -> (String, Vec<String>) {
    let mut parts = endpoint.split_whitespace();
    let command = parts.next().unwrap_or_default().to_string();
    let args = parts.map(str::to_string).collect();
    (command, args)
}

fn session_id_from(value: &Value) -> Result<String> {
    let session_id = string_field(value, &["sessionId", "session_id"]);
    if session_id.is_empty() {
        return Err(CoreError::Protocol(
            "session/new response missing sessionId".to_string(),
        ));
    }
    Ok(session_id)
}

fn request_id_to_string(id: &RequestId) -> String {
    match id {
        RequestId::Number(id) => id.to_string(),
        RequestId::String(id) => id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acp_mcp_servers_serializes_gateway_refs() {
        let servers = vec![McpServerRef {
            id: "tamtri-gateway".to_string(),
            name: "Tamtri Gateway".to_string(),
            transport: "http".to_string(),
            endpoint: "http://127.0.0.1:8765/mcp".to_string(),
        }];
        let value = acp_mcp_servers(&servers);
        assert_eq!(value[0]["name"], "Tamtri Gateway");
        assert_eq!(value[0]["type"], "http");
        assert_eq!(value[0]["url"], "http://127.0.0.1:8765/mcp");
        assert_eq!(value[0]["headers"], json!([]));
    }

    #[test]
    fn acp_mcp_servers_serializes_stdio_helper_args() {
        let servers = vec![McpServerRef {
            id: "tamtri-gateway".to_string(),
            name: "Tamtri Gateway".to_string(),
            transport: "stdio".to_string(),
            endpoint: "/tmp/tamtri-gateway-stdio http://127.0.0.1:1234/mcp".to_string(),
        }];
        let value = acp_mcp_servers(&servers);
        assert_eq!(value[0]["type"], "stdio");
        assert_eq!(value[0]["command"], "/tmp/tamtri-gateway-stdio");
        assert_eq!(value[0]["args"][0], "http://127.0.0.1:1234/mcp");
        assert_eq!(value[0]["env"], json!([]));
    }

    #[test]
    fn seed_renders_prior_transcript() {
        use crate::conversation::{ContentBlock, Message, Role};
        use chrono::Utc;
        use uuid::Uuid;

        let prior = Message {
            id: Uuid::now_v7(),
            role: Role::User,
            harness_id: None,
            content: vec![ContentBlock::Text {
                text: "prior context".to_string(),
            }],
            created_at: Utc::now(),
        };
        let user = Message {
            id: Uuid::now_v7(),
            role: Role::User,
            harness_id: None,
            content: vec![ContentBlock::Text {
                text: "follow up".to_string(),
            }],
            created_at: Utc::now(),
        };
        let prompt = render_prompt(
            &ContextSeed::FreshTranscript {
                messages: vec![prior],
            },
            &user,
        );
        assert!(prompt.contains("prior context"));
        assert!(prompt.contains("follow up"));
    }

    #[test]
    fn format_harness_run_error_adds_gateway_hint_for_internal_error() {
        let err = CoreError::JsonRpc {
            code: -32603,
            message: "Internal error".to_string(),
        };
        let message = format_harness_run_error(&err);
        assert!(message.contains("tamtri-gateway-stdio"));
        assert!(message.contains("twenty-questions"));
    }

    #[test]
    fn models_from_agent_capabilities_parses_model_list() {
        let caps = json!({
            "streaming": true,
            "models": [
                {"id": "mock", "displayName": "Mock Model"},
                {"id": "mock-fast", "name": "Mock Fast"}
            ]
        });
        let models = models_from_agent_capabilities(&caps);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "mock");
        assert_eq!(models[0].display_name, "Mock Model");
        assert_eq!(models[1].id, "mock-fast");
        assert_eq!(models[1].display_name, "Mock Fast");
    }
}
