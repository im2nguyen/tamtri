use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::conversation::{ContentBlock, Message};
use crate::harness::acp::AgentLaunchSpec;
use super::events::{is_approval_request, notification_events, permission_decision, permission_event};
use crate::harness::{
    ContextSeed, ConversationContext, HarnessEvent, RunCommand, TurnEndReason, TurnInput,
};
use crate::rpc::dispatch::{InboundMessage, RpcHandle};
use crate::rpc::jsonrpc::{JsonRpcError, RequestId};
use crate::{CoreError, Result};

const CODEX_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CODEX_TURN_TIMEOUT: Duration = Duration::from_secs(90 * 60);

pub async fn run_codex_session(
    rpc: RpcHandle,
    mut inbound: mpsc::Receiver<InboundMessage>,
    mut command_rx: mpsc::Receiver<RunCommand>,
    event_tx: mpsc::Sender<HarnessEvent>,
    ctx: ConversationContext,
    turn: TurnInput,
    harness_id: String,
) {
    if let Err(err) = run_codex_session_inner(
        rpc.clone(),
        &mut inbound,
        &mut command_rx,
        &event_tx,
        ctx,
        turn,
        &harness_id,
    )
    .await
    {
        let _ = event_tx
            .send(HarnessEvent::Error {
                message: err.to_string(),
            })
            .await;
        let _ = event_tx
            .send(HarnessEvent::TurnEnded {
                reason: TurnEndReason::Failed,
            })
            .await;
    }

    let _ = rpc.close().await;
}

async fn run_codex_session_inner(
    rpc: RpcHandle,
    inbound: &mut mpsc::Receiver<InboundMessage>,
    command_rx: &mut mpsc::Receiver<RunCommand>,
    event_tx: &mpsc::Sender<HarnessEvent>,
    ctx: ConversationContext,
    turn: TurnInput,
    harness_id: &str,
) -> Result<()> {
    handshake(&rpc).await?;

    let cwd = absolute_cwd(&ctx.working_dir_path)?;
    let model = resolve_model(&rpc, &ctx.model_id).await?;
    let thread_id = start_thread(&rpc, &model, &cwd).await?;

    let prompt = render_prompt(&ctx.seed, &turn.user_message);
    let turn_params = build_turn_start_params(&thread_id, &model, &cwd, &prompt);

    let (done_tx, mut done_rx) = oneshot::channel();
    let turn_rpc = rpc.clone();
    tokio::spawn(async move {
        let result = turn_rpc
            .request("turn/start", Some(turn_params), CODEX_TURN_TIMEOUT)
            .await;
        let _ = done_tx.send(result);
    });

    let mut permission_waiters: HashMap<String, RequestId> = HashMap::new();
    let mut turn_ended = false;

    loop {
        if turn_ended {
            break;
        }

        tokio::select! {
            inbound_msg = inbound.recv() => {
                let Some(inbound_msg) = inbound_msg else {
                    return Err(CoreError::TransportClosed);
                };
                if handle_inbound(&rpc, inbound_msg, event_tx, &mut permission_waiters).await {
                    turn_ended = true;
                }
            }
            command = command_rx.recv() => {
                match command {
                    Some(RunCommand::Cancel) => {
                        let _ = rpc.request(
                            "turn/interrupt",
                            Some(json!({ "threadId": thread_id })),
                            CODEX_REQUEST_TIMEOUT,
                        ).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Cancelled }).await;
                        turn_ended = true;
                    }
                    Some(RunCommand::RespondPermission { request_id, option_id }) => {
                        if let Some(id) = permission_waiters.remove(&request_id) {
                            let decision = permission_decision(&option_id);
                            let _ = rpc.respond(id, Ok(json!({ "decision": decision }))).await;
                            let _ = event_tx.send(HarnessEvent::PermissionResolved { request_id, option_id }).await;
                        }
                    }
                    None => {}
                }
            }
            done = &mut done_rx => {
                match done {
                    Ok(Ok(_)) if !turn_ended => {
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::EndTurn }).await;
                    }
                    Ok(Err(err)) if !turn_ended => {
                        let _ = event_tx.send(HarnessEvent::Error { message: err.to_string() }).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Failed }).await;
                    }
                    Err(_) if !turn_ended => {
                        let _ = event_tx.send(HarnessEvent::Error {
                            message: format!("{harness_id} turn task dropped"),
                        }).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Failed }).await;
                    }
                    _ => {}
                }
                turn_ended = true;
            }
        }
    }

    Ok(())
}

pub async fn connect_and_initialize(launch: &AgentLaunchSpec, cwd: Option<&Path>) -> Result<(RpcHandle, mpsc::Receiver<InboundMessage>)> {
    use crate::rpc::dispatch::RpcConnection;
    use crate::rpc::transport::stdio::StdioTransport;

    let args = effective_args(&launch.args);
    let transport = StdioTransport::spawn_with_cwd(
        &launch.command,
        &args,
        &launch.env,
        cwd,
    )
    .await?;
    let (rpc, inbound) = RpcConnection::start(Box::new(transport));
    handshake(&rpc).await?;
    Ok((rpc, inbound))
}

async fn handshake(rpc: &RpcHandle) -> Result<()> {
    rpc.request(
        "initialize",
        Some(json!({
            "clientInfo": {
                "name": "tamtri-daemon",
                "title": "tamtri",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "experimentalApi": true,
            },
        })),
        CODEX_REQUEST_TIMEOUT,
    )
    .await?;

    rpc.notify("initialized", Some(json!({}))).await?;
    Ok(())
}

async fn resolve_model(rpc: &RpcHandle, configured: &str) -> Result<String> {
    if !configured.trim().is_empty() && configured != "default" {
        return Ok(configured.to_string());
    }

    let response = rpc
        .request("model/list", Some(json!({})), CODEX_REQUEST_TIMEOUT)
        .await?;
    let models = response
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for model in &models {
        if model.get("isDefault").and_then(Value::as_bool) == Some(true)
            && let Some(id) = model.get("id").and_then(Value::as_str)
        {
            return Ok(id.to_string());
        }
    }
    models
        .first()
        .and_then(|model| model.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CoreError::Protocol("Codex app-server returned no models".to_string()))
}

async fn start_thread(rpc: &RpcHandle, model: &str, cwd: &str) -> Result<String> {
    let response = rpc
        .request(
            "thread/start",
            Some(json!({
                "model": model,
                "cwd": cwd,
                "approvalPolicy": "on-request",
                "sandbox": "workspace-write",
            })),
            CODEX_REQUEST_TIMEOUT,
        )
        .await?;
    response
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CoreError::Protocol("Codex app-server did not return thread id".to_string()))
}

fn build_turn_start_params(thread_id: &str, model: &str, cwd: &str, prompt: &str) -> Value {
    json!({
        "threadId": thread_id,
        "model": model,
        "cwd": cwd,
        "approvalPolicy": "on-request",
        "sandboxPolicy": {
            "type": "workspaceWrite",
            "networkAccess": false,
        },
        "input": [text_input(prompt)],
    })
}

fn text_input(text: &str) -> Value {
    json!({
        "type": "text",
        "text": text,
        "text_elements": [],
    })
}

pub fn effective_args(args: &[String]) -> Vec<String> {
    if args.is_empty() {
        vec!["app-server".to_string()]
    } else {
        args.to_vec()
    }
}

async fn handle_inbound(
    rpc: &RpcHandle,
    inbound_msg: InboundMessage,
    event_tx: &mpsc::Sender<HarnessEvent>,
    permission_waiters: &mut HashMap<String, RequestId>,
) -> bool {
    match inbound_msg {
        InboundMessage::Notification(note) => {
            let Some(params) = note.params else {
                return false;
            };
            for event in notification_events(&note.method, &params) {
                if matches!(event, HarnessEvent::TurnEnded { .. }) {
                    let _ = event_tx.send(event).await;
                    return true;
                }
                let _ = event_tx.send(event).await;
            }
        }
        InboundMessage::Request(req) if is_approval_request(&req.method) => {
            let params = req.params.unwrap_or_else(|| json!({}));
            let event = permission_event(&req.method, &params);
            let request_id = match &event {
                HarnessEvent::PermissionRequested { request_id, .. } => request_id.clone(),
                _ => String::new(),
            };
            permission_waiters.insert(request_id, req.id);
            let _ = event_tx.send(event).await;
        }
        InboundMessage::Request(req) => {
            let _ = rpc
                .respond(
                    req.id,
                    Err(JsonRpcError {
                        code: -32601,
                        message: format!("method not supported: {}", req.method),
                        data: None,
                    }),
                )
                .await;
        }
    }
    false
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

fn absolute_cwd(path: &Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(absolute.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_args_include_app_server() {
        assert_eq!(effective_args(&[]), vec!["app-server".to_string()]);
    }
}
