use std::path::PathBuf;
use std::time::Duration;

use tamtri_core::conversation::{ContentBlock, Message, Role, WorkingDir};
use tamtri_core::harness::acp::{AcpAdapter, AgentLaunchSpec};
use tamtri_core::harness::{
    ContextSeed, ConversationContext, HarnessAdapter, HarnessEvent, ToolStatus, TurnEndReason,
    TurnInput,
};

fn adapter() -> AcpAdapter {
    AcpAdapter::new(AgentLaunchSpec {
        id: "mock-acp".into(),
        display_name: "Mock ACP".into(),
        command: env!("CARGO_BIN_EXE_mock-acp-agent").into(),
        args: Vec::new(),
        env: Vec::new(),
    })
}

fn real_adapter_from_env() -> Option<AcpAdapter> {
    let command = std::env::var("TAMTRI_REAL_ACP_COMMAND").ok()?;
    let args = std::env::var("TAMTRI_REAL_ACP_ARGS")
        .ok()
        .map(|raw| raw.split_whitespace().map(str::to_string).collect())
        .unwrap_or_else(|| vec!["acp".to_string()]);
    Some(AcpAdapter::new(AgentLaunchSpec {
        id: "real-acp".into(),
        display_name: "Real ACP".into(),
        command,
        args,
        env: Vec::new(),
    }))
}

fn ctx() -> ConversationContext {
    ConversationContext {
        seed: ContextSeed::FreshTranscript {
            messages: Vec::new(),
        },
        working_dir: WorkingDir::VaultLocal,
        working_dir_path: PathBuf::from("."),
        roots: Vec::new(),
        mcp_servers: Vec::new(),
        model_id: "mock".into(),
    }
}

fn user_turn() -> TurnInput {
    TurnInput {
        user_message: Message {
            id: uuid::Uuid::now_v7(),
            role: Role::User,
            harness_id: None,
            content: vec![ContentBlock::Text {
                text: "make a report".into(),
            }],
            created_at: chrono::Utc::now(),
        },
    }
}

#[tokio::test]
async fn acp_handshake_and_session_streams_events() {
    let run = adapter().run(ctx(), user_turn()).await.unwrap();
    let mut events = collect_until_permission(run).await;

    assert!(
        events.iter().any(
            |event| matches!(event, HarnessEvent::ThoughtDelta { text } if text == "thinking")
        )
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, HarnessEvent::TextDelta { text } if text == "Hello"))
    );
    assert!(
        events.iter().any(
            |event| matches!(event, HarnessEvent::ToolCallStarted { id, .. } if id == "tool-1")
        )
    );
    assert!(events.iter().any(|event| matches!(event, HarnessEvent::ToolCallProgress { id, status: ToolStatus::Completed, .. } if id == "tool-1")));
    assert!(events.iter().any(
        |event| matches!(event, HarnessEvent::FileChanged { path, .. } if path == "report.html")
    ));
    assert!(matches!(
        events.pop(),
        Some(HarnessEvent::PermissionRequested { .. })
    ));
}

#[tokio::test]
async fn permission_round_trip_mid_turn() {
    let mut run = adapter().run(ctx(), user_turn()).await.unwrap();
    let mut permission_id = None;

    while let Some(event) = run.events.recv().await {
        if let HarnessEvent::PermissionRequested { request_id, .. } = event {
            permission_id = Some(request_id);
            break;
        }
    }
    let permission_id = permission_id.unwrap();
    run.control
        .respond_permission(&permission_id, "allow_once")
        .await
        .unwrap();

    let mut saw_resolution = false;
    let mut saw_end = false;
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_secs(2), run.events.recv()).await
    {
        match event {
            HarnessEvent::PermissionResolved {
                request_id,
                option_id,
            } => {
                saw_resolution = request_id == permission_id && option_id == "allow_once";
            }
            HarnessEvent::TurnEnded {
                reason: TurnEndReason::EndTurn,
            } => {
                saw_end = true;
                break;
            }
            _ => {}
        }
    }

    assert!(saw_resolution);
    assert!(saw_end);
}

#[tokio::test]
#[ignore = "requires a locally installed real ACP agent; set TAMTRI_REAL_ACP_COMMAND and optional TAMTRI_REAL_ACP_ARGS"]
async fn hermes_acp_smoke() {
    let adapter = real_adapter_from_env()
        .expect("set TAMTRI_REAL_ACP_COMMAND, for example /Users/dos/.local/bin/hermes");
    let mut turn = user_turn();
    turn.user_message.content = vec![ContentBlock::Text {
        text: "Reply with exactly: hello from acp".into(),
    }];

    let mut run = adapter.run(ctx(), turn).await.expect("start real ACP run");
    let mut saw_text = false;
    let mut saw_end = false;

    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_secs(90), run.events.recv()).await
    {
        match event {
            HarnessEvent::TextDelta { text } => {
                saw_text |= !text.trim().is_empty();
            }
            HarnessEvent::PermissionRequested {
                request_id,
                options,
                ..
            } => {
                let option = options
                    .iter()
                    .find(|option| option.id.contains("allow"))
                    .or_else(|| options.first())
                    .expect("permission option")
                    .id
                    .clone();
                run.control
                    .respond_permission(&request_id, &option)
                    .await
                    .expect("respond permission");
            }
            HarnessEvent::TurnEnded { reason } => {
                assert_eq!(reason, TurnEndReason::EndTurn);
                saw_end = true;
                break;
            }
            HarnessEvent::Error { message } => panic!("real ACP error: {message}"),
            _ => {}
        }
    }

    assert!(saw_text, "real ACP agent should stream text");
    assert!(saw_end, "real ACP agent should end the turn");
}

async fn collect_until_permission(mut run: tamtri_core::harness::HarnessRun) -> Vec<HarnessEvent> {
    let mut events = Vec::new();
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_secs(2), run.events.recv()).await
    {
        let done = matches!(event, HarnessEvent::PermissionRequested { .. });
        events.push(event);
        if done {
            break;
        }
    }
    events
}
