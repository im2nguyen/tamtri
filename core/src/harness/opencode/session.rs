use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::conversation::{ContentBlock, Message};
use crate::harness::acp::AgentLaunchSpec;
use crate::harness::opencode::client::{OpenCodeClient, absolute_directory};
use crate::harness::opencode::events::{OpenCodeEventState, normalize_opencode_event};
use crate::harness::opencode::server::OpenCodeServer;
use crate::harness::{
    ContextSeed, ConversationContext, HarnessEvent, ModelInfo, RunCommand, TurnEndReason, TurnInput,
};
use crate::{CoreError, Result};

const EVENT_READ_TIMEOUT: Duration = Duration::from_secs(90 * 60);

pub async fn run_opencode_session(
    launch: AgentLaunchSpec,
    ctx: ConversationContext,
    turn: TurnInput,
    mut command_rx: mpsc::Receiver<RunCommand>,
    event_tx: mpsc::Sender<HarnessEvent>,
    harness_id: String,
) {
    if let Err(err) =
        run_opencode_session_inner(launch, ctx, turn, &mut command_rx, &event_tx, &harness_id).await
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

async fn run_opencode_session_inner(
    launch: AgentLaunchSpec,
    ctx: ConversationContext,
    turn: TurnInput,
    command_rx: &mut mpsc::Receiver<RunCommand>,
    event_tx: &mpsc::Sender<HarnessEvent>,
    harness_id: &str,
) -> Result<()> {
    let directory = absolute_directory(&ctx.working_dir_path)?;
    let server = OpenCodeServer::spawn(&launch).await?;
    let client = OpenCodeClient::new(server.base_url.clone(), directory.clone())?;

    let session_id = if let Some(link) = ctx.native_session.as_ref()
        && link.provider == "opencode"
        && !link.session_id.is_empty()
    {
        client.resume_session(&link.session_id).await?
    } else {
        client.create_session().await?
    };

    let _ = event_tx
        .send(HarnessEvent::NativeSessionBound {
            provider: "opencode".to_string(),
            session_id: session_id.clone(),
            cwd: Some(directory.clone()),
        })
        .await;

    let prompt = select_prompt(&ctx, &turn);
    client
        .prompt_async(&session_id, &prompt, &ctx.model_id)
        .await?;

    let response = client.subscribe_events().await?;
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let state = OpenCodeEventState::new(session_id.clone());
    let mut turn_ended = false;
    let deadline = std::time::Instant::now() + EVENT_READ_TIMEOUT;

    loop {
        if turn_ended {
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err(CoreError::Protocol(format!(
                "{harness_id} timed out waiting for OpenCode turn end"
            )));
        }

        tokio::select! {
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(index) = buffer.find("\n\n") {
                            let frame = buffer[..index].to_string();
                            buffer.replace_range(..index + 2, "");
                            if let Some(data) = sse_data(&frame) {
                                let parsed: Value = serde_json::from_str(&data)
                                    .map_err(|err| CoreError::Protocol(format!("invalid OpenCode SSE JSON: {err}")))?;
                                for event in normalize_opencode_event(&parsed, &state) {
                                    if matches!(event, HarnessEvent::TurnEnded { .. }) {
                                        turn_ended = true;
                                    }
                                    let _ = event_tx.send(event).await;
                                }
                            }
                        }
                    }
                    Some(Err(err)) => {
                        return Err(CoreError::Protocol(format!("OpenCode SSE stream failed: {err}")));
                    }
                    None if !turn_ended => {
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::EndTurn }).await;
                        turn_ended = true;
                    }
                    None => break,
                }
            }
            command = command_rx.recv() => {
                match command {
                    Some(RunCommand::Cancel) => {
                        let _ = client.abort_session(&session_id).await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded { reason: TurnEndReason::Cancelled }).await;
                        turn_ended = true;
                    }
                    Some(RunCommand::RespondPermission { request_id, option_id }) => {
                        let allow = option_id == "allow_once" || option_id == "allow";
                        let _ = client.respond_permission(&session_id, &request_id, allow).await;
                        let _ = event_tx.send(HarnessEvent::PermissionResolved { request_id, option_id }).await;
                    }
                    None => {}
                }
            }
        }
    }

    server.shutdown().await;
    Ok(())
}

pub async fn list_models(launch: &AgentLaunchSpec) -> Result<Vec<ModelInfo>> {
    let server = OpenCodeServer::spawn(launch).await?;
    let directory = std::env::temp_dir().to_string_lossy().into_owned();
    let client = OpenCodeClient::new(server.base_url.clone(), directory)?;
    let models = client.list_models().await;
    server.shutdown().await;
    models
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

fn sse_data(frame: &str) -> Option<String> {
    let data = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() { None } else { Some(data) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_data_extracts_payload() {
        let frame = "event: message\ndata: {\"type\":\"session.idle\"}\n";
        assert_eq!(
            sse_data(frame),
            Some("{\"type\":\"session.idle\"}".to_string())
        );
    }
}
