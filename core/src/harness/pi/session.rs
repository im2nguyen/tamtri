use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::conversation::{ContentBlock, Message};
use crate::harness::acp::AgentLaunchSpec;
use crate::harness::pi::events::normalize_pi_event;
use crate::harness::pi::rpc::{PiRpcHandle, shutdown_child};
use crate::harness::spawn_env::preserve_spawn_env_tokio;
use crate::harness::{
    ContextSeed, ConversationContext, HarnessEvent, RunCommand, TurnEndReason, TurnInput,
};
use crate::{CoreError, Result};

const PI_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const PI_PROMPT_TIMEOUT: Duration = Duration::from_secs(90 * 60);

pub fn effective_args(args: &[String], ctx: &ConversationContext) -> Vec<String> {
    let mut out = args.to_vec();
    if !has_mode_rpc(&out) {
        out.extend(["--mode".to_string(), "rpc".to_string()]);
    }
    if !ctx.model_id.trim().is_empty() && ctx.model_id != "default" && !has_model_flag(&out) {
        out.extend(["--model".to_string(), ctx.model_id.clone()]);
    }
    if let Some(link) = ctx.native_session.as_ref()
        && link.provider == "pi"
        && let Some(source_path) = link.source_path.as_deref()
        && !has_session_flag(&out)
    {
        out.extend(["--session".to_string(), source_path.to_string()]);
    }
    out
}

pub fn spawn_cwd(ctx: &ConversationContext) -> std::path::PathBuf {
    let cwd = ctx.working_dir_path.clone();
    let _ = std::fs::create_dir_all(&cwd);
    cwd
}

pub async fn run_pi_session(
    launch: AgentLaunchSpec,
    ctx: ConversationContext,
    turn: TurnInput,
    mut command_rx: mpsc::Receiver<RunCommand>,
    event_tx: mpsc::Sender<HarnessEvent>,
    harness_id: String,
) {
    if let Err(err) =
        run_pi_session_inner(&launch, ctx, turn, &mut command_rx, &event_tx, &harness_id).await
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
}

async fn run_pi_session_inner(
    launch: &AgentLaunchSpec,
    ctx: ConversationContext,
    turn: TurnInput,
    command_rx: &mut mpsc::Receiver<RunCommand>,
    event_tx: &mpsc::Sender<HarnessEvent>,
    harness_id: &str,
) -> Result<()> {
    let cwd = spawn_cwd(&ctx);
    let args = effective_args(&launch.args, &ctx);
    let mut child = spawn_pi_process(launch, &args, &cwd)?;
    let stdin = child.stdin.take().ok_or(CoreError::TransportClosed)?;
    let stdout = child.stdout.take().ok_or(CoreError::TransportClosed)?;
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(target: "tamtri_core::pi::stderr", "{line}");
            }
        });
    }

    let (raw_event_tx, mut raw_event_rx) = mpsc::channel(128);
    let (rpc, reader) = PiRpcHandle::start(stdin, stdout, raw_event_tx);

    let prompt = select_prompt(&ctx, &turn);
    let (done_tx, mut done_rx) = oneshot::channel();
    let rpc_for_prompt = rpc.clone();
    tokio::spawn(async move {
        let result = rpc_for_prompt
            .request(
                json!({ "type": "prompt", "message": prompt }),
                PI_PROMPT_TIMEOUT,
            )
            .await;
        let _ = done_tx.send(result);
    });

    let mut turn_ended = false;
    loop {
        if turn_ended {
            break;
        }

        tokio::select! {
            raw = raw_event_rx.recv() => {
                let Some(raw) = raw else {
                    return Err(CoreError::TransportClosed);
                };
                for event in normalize_pi_event(&raw) {
                    if matches!(event, HarnessEvent::TurnEnded { .. }) {
                        turn_ended = true;
                    }
                    let _ = event_tx.send(event).await;
                }
            }
            command = command_rx.recv() => {
                match command {
                    Some(RunCommand::Cancel) => {
                        let _ = rpc.request(json!({ "type": "abort" }), PI_REQUEST_TIMEOUT).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Cancelled }).await;
                        turn_ended = true;
                    }
                    Some(RunCommand::RespondPermission { request_id, option_id }) => {
                        let mut payload = serde_json::Map::new();
                        payload.insert("type".to_string(), Value::String("extension_ui_response".to_string()));
                        payload.insert("id".to_string(), Value::String(request_id.clone()));
                        payload.extend(permission_response(&option_id));
                        let _ = rpc.write_line(&Value::Object(payload)).await;
                        let _ = event_tx.send(HarnessEvent::PermissionResolved { request_id, option_id }).await;
                    }
                    None => {}
                }
            }
            done = &mut done_rx => {
                match done {
                    Ok(Ok(_)) => {}
                    Ok(Err(err)) if !turn_ended => {
                        let _ = event_tx.send(HarnessEvent::Error { message: err.to_string() }).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Failed }).await;
                        turn_ended = true;
                    }
                    Err(_) if !turn_ended => {
                        let _ = event_tx.send(HarnessEvent::Error {
                            message: format!("{harness_id} prompt task dropped"),
                        }).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Failed }).await;
                        turn_ended = true;
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = reader.await;
    shutdown_child(child).await;
    Ok(())
}

fn spawn_pi_process(
    launch: &AgentLaunchSpec,
    args: &[String],
    cwd: &Path,
) -> Result<tokio::process::Child> {
    let mut cmd = Command::new(&launch.command);
    cmd.args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(cwd);
    preserve_spawn_env_tokio(&mut cmd);
    for (key, value) in &launch.env {
        cmd.env(key, value);
    }
    cmd.spawn().map_err(CoreError::from)
}

fn select_prompt(ctx: &ConversationContext, turn: &TurnInput) -> String {
    let ContextSeed::FreshTranscript { messages } = &ctx.seed;
    if messages.is_empty() {
        return render_message(&turn.user_message);
    }
    let mut out = String::new();
    for message in messages {
        out.push_str(&format!("{:?}: ", message.role));
        out.push_str(&render_message(message));
        out.push('\n');
    }
    out.push_str("User: ");
    out.push_str(&render_message(&turn.user_message));
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

fn permission_response(option_id: &str) -> serde_json::Map<String, Value> {
    match option_id {
        "allow_once" | "allow" => {
            let mut map = serde_json::Map::new();
            map.insert("confirmed".to_string(), Value::Bool(true));
            map
        }
        "deny" => {
            let mut map = serde_json::Map::new();
            map.insert("cancelled".to_string(), Value::Bool(true));
            map
        }
        _ => {
            let mut map = serde_json::Map::new();
            map.insert("cancelled".to_string(), Value::Bool(true));
            map
        }
    }
}

fn has_mode_rpc(args: &[String]) -> bool {
    args.windows(2)
        .any(|window| window[0] == "--mode" && window[1] == "rpc")
        || args.iter().any(|arg| arg == "--mode=rpc")
}

fn has_model_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--model" || arg.starts_with("--model="))
}

fn has_session_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--session" || arg.starts_with("--session="))
}

pub async fn connect_for_models(
    launch: &AgentLaunchSpec,
) -> Result<Vec<crate::harness::ModelInfo>> {
    let cwd = std::env::temp_dir();
    let args = effective_args(
        &launch.args,
        &ConversationContext {
            seed: ContextSeed::FreshTranscript {
                messages: Vec::new(),
            },
            working_dir: crate::conversation::WorkingDir::VaultLocal,
            working_dir_path: cwd.clone(),
            roots: Vec::new(),
            mcp_servers: Vec::new(),
            model_id: String::new(),
            native_session: None,
            tool_catalog: Vec::new(),
        },
    );
    let mut child = spawn_pi_process(launch, &args, &cwd)?;
    let stdin = child.stdin.take().ok_or(CoreError::TransportClosed)?;
    let stdout = child.stdout.take().ok_or(CoreError::TransportClosed)?;
    let (event_tx, event_rx) = mpsc::channel(8);
    let (rpc, reader) = PiRpcHandle::start(stdin, stdout, event_tx);
    let models = list_models(&rpc).await;
    drop(event_rx);
    let _ = reader.await;
    shutdown_child(child).await;
    models
}

pub async fn list_models(rpc: &PiRpcHandle) -> Result<Vec<crate::harness::ModelInfo>> {
    let data = rpc
        .request(
            json!({ "type": "get_available_models" }),
            PI_REQUEST_TIMEOUT,
        )
        .await?;
    let models = data
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(models
        .into_iter()
        .filter_map(|model| {
            let provider = model.get("provider")?.as_str()?;
            let id = model.get("id")?.as_str()?;
            let composite = format!("{provider}/{id}");
            let display_name = model
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(id)
                .to_string();
            Some(crate::harness::ModelInfo {
                id: composite,
                display_name,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_args_adds_rpc_mode_and_model() {
        let ctx = ConversationContext {
            seed: ContextSeed::FreshTranscript {
                messages: Vec::new(),
            },
            working_dir: crate::conversation::WorkingDir::VaultLocal,
            working_dir_path: std::path::PathBuf::from("/tmp/pi"),
            roots: Vec::new(),
            mcp_servers: Vec::new(),
            model_id: "anthropic/claude-sonnet".to_string(),
            native_session: None,
            tool_catalog: Vec::new(),
        };
        let args = effective_args(&[], &ctx);
        assert!(args.contains(&"--mode".to_string()));
        assert!(args.contains(&"rpc".to_string()));
        assert!(args.contains(&"--model".to_string()));
    }
}
