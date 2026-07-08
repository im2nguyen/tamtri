use std::path::PathBuf;
use std::process::Stdio;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::conversation::{ContentBlock, Message, NativeSessionLink};
use crate::harness::acp::AgentLaunchSpec;
use crate::harness::claude::events::stream_line_events;
use crate::harness::{
    ContextSeed, ConversationContext, HarnessEvent, RunCommand, TurnEndReason, TurnInput,
};
use crate::Result;

pub async fn run_claude_session(
    launch: AgentLaunchSpec,
    ctx: ConversationContext,
    turn: TurnInput,
    mut command_rx: mpsc::Receiver<RunCommand>,
    event_tx: mpsc::Sender<HarnessEvent>,
    harness_id: String,
) {
    let prompt = select_prompt(&ctx, &turn);
    let cwd = spawn_cwd(&ctx);
    let args = build_claude_args(&prompt, &ctx, &launch);

    let mut cmd = Command::new(&launch.command);
    cmd.args(&args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&cwd);
    preserve_env(&mut cmd, "PATH");
    preserve_env(&mut cmd, "HOME");
    preserve_env(&mut cmd, "TMPDIR");
    preserve_env(&mut cmd, "LANG");
    preserve_env(&mut cmd, "CLAUDE_CONFIG_DIR");
    for (key, value) in &launch.env {
        cmd.env(key, value);
    }

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
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
            return;
        }
    };

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(target: "tamtri_core::claude::stderr", "{line}");
            }
        });
    }

    let stdout = match child.stdout.take() {
        Some(stdout) => BufReader::new(stdout),
        None => {
            let _ = event_tx
                .send(HarnessEvent::Error {
                    message: "Claude process missing stdout".to_string(),
                })
                .await;
            let _ = event_tx
                .send(HarnessEvent::TurnEnded {
                    reason: TurnEndReason::Failed,
                })
                .await;
            return;
        }
    };

    if let Err(err) = read_stdout_loop(stdout, &mut command_rx, &event_tx, &mut child).await {
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
        let _ = child.kill().await;
        let _ = child.wait().await;
        return;
    }

    match child.wait().await {
        Ok(status) if !status.success() => {
            let _ = event_tx
                .send(HarnessEvent::Error {
                    message: format!("{harness_id} exited with {status}"),
                })
                .await;
            let _ = event_tx
                .send(HarnessEvent::TurnEnded {
                    reason: TurnEndReason::Failed,
                })
                .await;
        }
        Err(err) => {
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
        _ => {}
    }
}

async fn read_stdout_loop(
    mut stdout: BufReader<tokio::process::ChildStdout>,
    command_rx: &mut mpsc::Receiver<RunCommand>,
    event_tx: &mpsc::Sender<HarnessEvent>,
    child: &mut Child,
) -> Result<()> {
    let mut turn_ended = false;
    let mut line = String::new();

    loop {
        if turn_ended {
            break;
        }

        tokio::select! {
            read = stdout.read_line(&mut line) => {
                match read {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty()
                            && let Ok(value) = serde_json::from_str::<Value>(trimmed)
                        {
                            for event in stream_line_events(&value) {
                                if matches!(event, HarnessEvent::TurnEnded { .. }) {
                                    turn_ended = true;
                                }
                                let _ = event_tx.send(event).await;
                            }
                        }
                        line.clear();
                    }
                    Err(err) => return Err(err.into()),
                }
            }
            command = command_rx.recv() => {
                match command {
                    Some(RunCommand::Cancel) => {
                        let _ = child.kill().await;
                        let _ = event_tx.send(HarnessEvent::TurnEnded {
                            reason: TurnEndReason::Cancelled,
                        }).await;
                        turn_ended = true;
                    }
                    Some(RunCommand::RespondPermission { .. }) => {}
                    None => {}
                }
            }
        }
    }

    Ok(())
}

pub fn build_claude_args(prompt: &str, ctx: &ConversationContext, launch: &AgentLaunchSpec) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        prompt.to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(),
        "--permission-mode".to_string(),
        "default".to_string(),
    ];
    if let Some(model) = non_empty_model(&ctx.model_id) {
        args.push("--model".to_string());
        args.push(model);
    }
    if let Some(NativeSessionLink { provider, session_id, .. }) = &ctx.native_session
        && provider == "claude"
    {
        args.push("--resume".to_string());
        args.push(session_id.clone());
    }
    args.extend(launch.args.iter().cloned());
    args
}

fn select_prompt(ctx: &ConversationContext, turn: &TurnInput) -> String {
    if ctx
        .native_session
        .as_ref()
        .is_some_and(|link| link.provider == "claude")
    {
        return render_message(&turn.user_message);
    }
    render_prompt(&ctx.seed, &turn.user_message)
}

fn spawn_cwd(ctx: &ConversationContext) -> PathBuf {
    if let Some(link) = &ctx.native_session
        && !link.cwd.is_empty()
    {
        return PathBuf::from(&link.cwd);
    }
    ctx.working_dir_path.clone()
}

fn non_empty_model(model_id: &str) -> Option<String> {
    let trimmed = model_id.trim();
    if trimmed.is_empty() || trimmed == "default" {
        None
    } else {
        Some(trimmed.to_string())
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

fn preserve_env(cmd: &mut Command, key: &str) {
    if let Ok(value) = std::env::var(key) {
        cmd.env(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::{Role, WorkingDir};
    use crate::harness::{ContextSeed, ConversationContext};

    #[test]
    fn resume_uses_only_latest_user_message() {
        let ctx = ConversationContext {
            seed: ContextSeed::FreshTranscript {
                messages: vec![Message {
                    id: uuid::Uuid::now_v7(),
                    role: Role::User,
                    harness_id: None,
                    content: vec![ContentBlock::Text {
                        text: "old".into(),
                    }],
                    created_at: chrono::Utc::now(),
                }],
            },
            working_dir: WorkingDir::VaultLocal,
            working_dir_path: PathBuf::from("/tmp"),
            roots: Vec::new(),
            mcp_servers: Vec::new(),
            model_id: String::new(),
            native_session: Some(NativeSessionLink {
                provider: "claude".into(),
                session_id: "abc".into(),
                cwd: "/tmp".into(),
                source_path: None,
            }),
        };
        let turn = TurnInput {
            user_message: Message {
                id: uuid::Uuid::now_v7(),
                role: Role::User,
                harness_id: None,
                content: vec![ContentBlock::Text {
                    text: "new".into(),
                }],
                created_at: chrono::Utc::now(),
            },
        };
        assert_eq!(select_prompt(&ctx, &turn), "new");
    }

    #[test]
    fn build_args_include_resume_and_stream_json() {
        let ctx = ConversationContext {
            seed: ContextSeed::FreshTranscript {
                messages: Vec::new(),
            },
            working_dir: WorkingDir::VaultLocal,
            working_dir_path: PathBuf::from("/tmp"),
            roots: Vec::new(),
            mcp_servers: Vec::new(),
            model_id: "sonnet".into(),
            native_session: Some(NativeSessionLink {
                provider: "claude".into(),
                session_id: "abc".into(),
                cwd: "/tmp".into(),
                source_path: None,
            }),
        };
        let args = build_claude_args(
            "hello",
            &ctx,
            &AgentLaunchSpec {
                id: "claude".into(),
                display_name: "Claude".into(),
                command: "claude".into(),
                args: vec![],
                env: vec![],
                adapter: Default::default(),
            },
        );
        assert!(args.contains(&"--resume".to_string()));
        assert!(args.contains(&"abc".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
    }
}
