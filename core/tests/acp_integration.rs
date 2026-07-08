use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::config::GatewayConfig;
use tamtri_core::conversation::{ContentBlock, Message, Role, WorkingDir};
use tamtri_core::harness::acp::{AcpAdapter, AgentLaunchSpec};
use tamtri_core::harness::{
    ContextSeed, ConversationContext, HarnessAdapter, HarnessEvent, ToolStatus, TurnEndReason,
    TurnInput,
};
use tamtri_core::mcp::endpoint::start_loopback_gateway;
use tamtri_core::mcp::gateway::{McpGateway, MemoryCredentials};

fn adapter() -> AcpAdapter {
    AcpAdapter::new(AgentLaunchSpec {
        id: "mock-acp".into(),
        display_name: "Mock ACP".into(),
        command: env!("CARGO_BIN_EXE_mock-acp-agent").into(),
        args: Vec::new(),
        env: Vec::new(),
        adapter: Default::default(),
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
        adapter: Default::default(),
    }))
}

fn ctx_in(path: PathBuf) -> ConversationContext {
    ConversationContext {
        seed: ContextSeed::FreshTranscript {
            messages: Vec::new(),
        },
        working_dir: WorkingDir::VaultLocal,
        working_dir_path: path,
        roots: Vec::new(),
        mcp_servers: Vec::new(),
        model_id: "mock".into(),
        native_session: None,
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
async fn acp_handshake_and_session() {
    let temp = tempfile::tempdir().unwrap();
    let workdir = temp.path().to_path_buf();
    let adapter = adapter();
    let run = adapter
        .run(ctx_in(workdir.clone()), user_turn())
        .await
        .unwrap();

    let caps = adapter
        .agent_capabilities()
        .expect("initialize should record agentCapabilities");
    assert_eq!(caps["streaming"], true);
    assert_eq!(caps["tools"], true);
    assert_eq!(caps["models"][0]["id"], "mock");

    let cwd_marker = workdir.join(".session-cwd.txt");
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if cwd_marker.is_file() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let recorded_cwd = std::fs::read_to_string(&cwd_marker).expect("session/new cwd marker");
    let expected_cwd = workdir.canonicalize().unwrap();
    let recorded = std::path::PathBuf::from(recorded_cwd.trim());
    let recorded = recorded.canonicalize().unwrap_or(recorded);
    assert_eq!(recorded, expected_cwd);

    run.control.cancel().await.ok();
}

#[tokio::test]
async fn acp_handshake_and_session_streams_events() {
    let temp = tempfile::tempdir().unwrap();
    let run = adapter()
        .run(ctx_in(temp.path().to_path_buf()), user_turn())
        .await
        .unwrap();
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
async fn stream_normalizes_to_events() {
    let temp = tempfile::tempdir().unwrap();
    let run = adapter()
        .run(ctx_in(temp.path().to_path_buf()), user_turn())
        .await
        .unwrap();
    let events = collect_until_permission(run).await;

    let kinds: Vec<&str> = events
        .iter()
        .map(|event| match event {
            HarnessEvent::ThoughtDelta { .. } => "thought_delta",
            HarnessEvent::TextDelta { .. } => "text_delta",
            HarnessEvent::ToolCallStarted { .. } => "tool_call_started",
            HarnessEvent::ToolCallProgress { .. } => "tool_call_progress",
            HarnessEvent::FileChanged { .. } => "file_changed",
            HarnessEvent::PermissionRequested { .. } => "permission_requested",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "thought_delta",
            "text_delta",
            "text_delta",
            "tool_call_started",
            "tool_call_progress",
            "file_changed",
            "permission_requested",
        ]
    );

    assert!(
        events.iter().any(
            |event| matches!(event, HarnessEvent::ThoughtDelta { text } if text == "thinking")
        )
    );
    assert!(
        events
            .iter()
            .filter(|event| matches!(event, HarnessEvent::TextDelta { .. }))
            .map(|event| match event {
                HarnessEvent::TextDelta { text } => text.as_str(),
                _ => "",
            })
            .collect::<String>()
            == "Hello world"
    );
    assert!(events.iter().any(|event| matches!(
        event,
        HarnessEvent::ToolCallStarted { id, name, .. } if id == "tool-1" && name == "Write"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        HarnessEvent::ToolCallProgress {
            id,
            status: ToolStatus::Completed,
            ..
        } if id == "tool-1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        HarnessEvent::FileChanged { path, diff, .. } if path == "report.html" && diff.new_text.as_deref() == Some("<h1>ok</h1>")
    )));
    assert!(matches!(
        events.last(),
        Some(HarnessEvent::PermissionRequested { .. })
    ));
}

#[tokio::test]
async fn permission_round_trip_mid_turn() {
    let temp = tempfile::tempdir().unwrap();
    let mut run = adapter()
        .run(ctx_in(temp.path().to_path_buf()), user_turn())
        .await
        .unwrap();
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
async fn cancel_sends_session_cancel() {
    let temp = tempfile::tempdir().unwrap();
    let mut run = adapter()
        .run(ctx_in(temp.path().to_path_buf()), user_turn())
        .await
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), run.events.recv()).await;
    run.control.cancel().await.unwrap();

    let mut saw_cancelled = false;
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_secs(2), run.events.recv()).await
    {
        if matches!(
            event,
            HarnessEvent::TurnEnded {
                reason: TurnEndReason::Cancelled
            }
        ) {
            saw_cancelled = true;
            break;
        }
    }
    assert!(saw_cancelled);
}

#[test]
fn mock_acp_agent_calls_gateway_tool() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver);
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer)
        .expect("core");

    let mock_mcp = env!("CARGO_BIN_EXE_mock-mcp-server").to_string();
    tamtri_core::config::replace_gateway_servers(
        temp.path(),
        vec![gateway_mock_server(&mock_mcp)],
    )
    .expect("save config");

    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");

    let conversation = core
        .create_conversation(
            "Gateway echo".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "go".to_string())
        .expect("send");

    let conversation_id = uuid::Uuid::parse_str(&conversation.id).expect("uuid");
    let workdir = core
        .conversation_workdir_path(conversation.id.clone())
        .expect("workdir");
    let marker = std::path::Path::new(&workdir).join(".gateway-echo-ok");

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if marker.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    core.cancel_run(conversation.id).ok();
    assert_eq!(
        std::fs::read_to_string(&marker).expect("marker"),
        "gateway-echo-test"
    );
    let _ = conversation_id;
}

#[test]
fn acp_session_new_includes_gateway() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver);
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer)
        .expect("core");

    let mock_mcp = env!("CARGO_BIN_EXE_mock-mcp-server").to_string();
    tamtri_core::config::replace_gateway_servers(
        temp.path(),
        vec![gateway_mock_server(&mock_mcp)],
    )
    .expect("save config");

    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");

    let conversation = core
        .create_conversation(
            "Gateway session".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "go".to_string())
        .expect("send");

    let workdir = core
        .conversation_workdir_path(conversation.id.clone())
        .expect("workdir");
    let marker = std::path::Path::new(&workdir).join(".session-mcp-servers.json");

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if marker.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let raw = std::fs::read_to_string(&marker).expect("session/new mcpServers marker");
    let servers: Vec<serde_json::Value> = serde_json::from_str(&raw).expect("mcpServers json");
    assert_eq!(servers.len(), 1, "expected one tamtri gateway entry");
    assert_eq!(servers[0]["name"], "Tamtri Gateway");
    assert!(
        servers[0].get("command").is_some() || servers[0].get("url").is_some(),
        "gateway entry should expose transport endpoint only"
    );
    assert!(!raw.contains("super-secret"));
    assert!(!raw.contains("keychain://"));

    core.cancel_run(conversation.id).ok();
}

#[derive(Default)]
struct RecordingObserver;

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, _event: UiEvent) {}
}

fn gateway_mock_server(command: &str) -> tamtri_core::config::GatewayServerConfig {
    tamtri_core::config::GatewayServerConfig {
        id: "mock".to_string(),
        display_name: "Mock".to_string(),
        enabled: true,
        scope: tamtri_core::config::GatewayScope::User,
        transport: tamtri_core::config::GatewayTransport::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        timeout_secs: Some(30),
        credentials: Vec::new(),
        oauth: None,
    }
}

#[tokio::test]
#[ignore = "requires a locally installed real ACP agent; set TAMTRI_REAL_ACP_COMMAND and optional TAMTRI_REAL_ACP_ARGS"]
async fn integration_real_agent() {
    let adapter = real_adapter_from_env()
        .expect("set TAMTRI_REAL_ACP_COMMAND, for example /Users/dos/.local/bin/hermes");
    let mut turn = user_turn();
    turn.user_message.content = vec![ContentBlock::Text {
        text: "Reply with exactly: hello from acp".into(),
    }];

    let temp = tempfile::tempdir().unwrap();
    let mut run = adapter
        .run(ctx_in(temp.path().to_path_buf()), turn)
        .await
        .expect("start real ACP run");
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

#[tokio::test]
#[ignore = "requires a locally installed real ACP agent; set TAMTRI_REAL_ACP_COMMAND and optional TAMTRI_REAL_ACP_ARGS"]
async fn hermes_acp_accepts_tamtri_gateway_mcp_server() {
    let adapter = real_adapter_from_env()
        .expect("set TAMTRI_REAL_ACP_COMMAND, for example /Users/dos/.local/bin/hermes");
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig::default(),
            Arc::new(MemoryCredentials::default()),
            None,
        )
        .unwrap(),
    );
    let endpoint = start_loopback_gateway(gateway).await.unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut ctx = ctx_in(temp.path().to_path_buf());
    ctx.mcp_servers = vec![endpoint.mcp_ref()];
    let mut turn = user_turn();
    turn.user_message.content = vec![ContentBlock::Text {
        text: "Reply with exactly: gateway accepted".into(),
    }];

    let mut run = adapter
        .run(ctx, turn)
        .await
        .expect("Hermes should accept Tamtri gateway mcpServers shape");
    let mut saw_text = false;
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_secs(90), run.events.recv()).await
    {
        match event {
            HarnessEvent::TextDelta { text } => saw_text |= !text.trim().is_empty(),
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
                break;
            }
            HarnessEvent::Error { message } => panic!("real ACP gateway error: {message}"),
            _ => {}
        }
    }
    endpoint.shutdown().await;
    assert!(saw_text, "real ACP agent should stream text");
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
