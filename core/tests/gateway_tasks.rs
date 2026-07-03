use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tamtri_core::config::{GatewayConfig, GatewayScope, GatewayServerConfig, GatewayTransport};
use tamtri_core::conversation::{ContentBlock, ElicitationAction, TaskStatus};
use tamtri_core::mcp::capabilities::TamtriFeatureSupport;
use tamtri_core::mcp::gateway::{GatewayEvent, McpGateway, NoCredentials};
use tokio::sync::mpsc;

fn stdio_server(id: &str, command: &str) -> GatewayServerConfig {
    GatewayServerConfig {
        id: id.to_string(),
        display_name: id.to_string(),
        enabled: true,
        scope: GatewayScope::Project,
        transport: GatewayTransport::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        timeout_secs: Some(30),
        credentials: Vec::new(),
        oauth: None,
    }
}

async fn start_progress_task(gateway: Arc<McpGateway>) -> (String, String) {
    start_progress_task_with_origin(gateway, "tool-task-1").await
}

async fn start_progress_task_with_origin(
    gateway: Arc<McpGateway>,
    origin_tool_call_id: &str,
) -> (String, String) {
    let tools = gateway.list_tools().await.expect("list tools");
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "progress_task")
        .map(|tool| tool.exposed_name.clone())
        .expect("progress_task");
    let mut rx = gateway.subscribe();
    gateway
        .call_tool_with_meta(
            &exposed,
            json!({}),
            Some(json!({"toolCallId": origin_tool_call_id})),
        )
        .await
        .expect("call progress_task");
    loop {
        let event = rx.recv().await.expect("gateway event");
        if let GatewayEvent::TaskStarted { state } = event {
            return (exposed, state.task_id);
        }
    }
}

#[tokio::test]
async fn task_progress_updates_live_card() {
    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );
    assert!(TamtriFeatureSupport::current().tasks);

    let (_exposed, task_id) = start_progress_task(Arc::clone(&gateway)).await;
    let mut updates = gateway.subscribe();
    let mut saw_update = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let Ok(event) = tokio::time::timeout(Duration::from_millis(500), updates.recv()).await
        else {
            continue;
        };
        let event = event.expect("event");
        match event {
            GatewayEvent::TaskUpdated { state } if state.task_id == task_id => {
                saw_update = true;
                break;
            }
            GatewayEvent::TaskCompleted { state, .. } if state.task_id == task_id => {
                saw_update = true;
                break;
            }
            _ => {}
        }
    }
    assert!(saw_update, "expected live task update");
}

#[tokio::test]
async fn task_completion_persists_task_ref() {
    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let (_exposed, task_id) = start_progress_task(Arc::clone(&gateway)).await;
    let mut rx = gateway.subscribe();
    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskCompleted { state, .. } = event
                && state.task_id == task_id
            {
                return state;
            }
        }
    })
    .await
    .expect("task should complete");
    assert_eq!(completed.status, TaskStatus::Completed);
    assert_eq!(
        completed.origin_tool_call_id.as_deref(),
        Some("tool-task-1")
    );

    let block = ContentBlock::TaskRef {
        task_id: completed.task_id.clone(),
        status: completed.status.clone(),
        title: completed.title.clone(),
        result_summary: completed.result.as_ref().map(|value| value.to_string()),
        origin_tool_call_id: completed.origin_tool_call_id.clone(),
    };
    assert_eq!(
        block,
        ContentBlock::TaskRef {
            task_id,
            status: TaskStatus::Completed,
            title: completed.title.clone(),
            result_summary: completed.result.as_ref().map(|value| value.to_string()),
            origin_tool_call_id: Some("tool-task-1".to_string()),
        }
    );
    let serialized = serde_json::to_string(&block).expect("serialize task ref");
    assert!(serialized.contains("task_ref"));
    assert!(serialized.contains("origin_tool_call_id"));
    assert!(serialized.contains("tool-task-1"));
}

#[tokio::test]
async fn task_ref_origin_persisted_in_messages_jsonl() {
    use chrono::Utc;
    use std::fs;
    use tamtri_core::conversation::{Conversation, Id, Message, Role};
    use tamtri_core::vault::ConversationVault;
    use tamtri_core::vault::fs::FilesystemVault;

    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let (_exposed, task_id) = start_progress_task(Arc::clone(&gateway)).await;
    let mut rx = gateway.subscribe();
    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskCompleted { state, .. } = event
                && state.task_id == task_id
            {
                return state;
            }
        }
    })
    .await
    .expect("task should complete");

    let block = ContentBlock::TaskRef {
        task_id: completed.task_id.clone(),
        status: completed.status.clone(),
        title: completed.title.clone(),
        result_summary: completed.result.as_ref().map(|value| value.to_string()),
        origin_tool_call_id: completed.origin_tool_call_id.clone(),
    };

    let dir = tempfile::tempdir().unwrap();
    let vault = FilesystemVault::new(dir.path().to_path_buf()).unwrap();
    let mut conversation = Conversation::new("Task replay");
    conversation.push_message(Message {
        id: Id::now_v7(),
        role: Role::Assistant,
        harness_id: None,
        content: vec![
            ContentBlock::ToolCall {
                id: "tool-task-1".into(),
                name: "tasks__progress_task".into(),
                input: json!({}),
            },
            block.clone(),
        ],
        created_at: Utc::now(),
    });
    vault.create(&conversation).unwrap();

    let messages_path = dir
        .path()
        .join("conversations")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("messages.jsonl");
    let messages_text = fs::read_to_string(&messages_path).unwrap();
    assert!(
        messages_text.contains("origin_tool_call_id"),
        "messages.jsonl should include origin_tool_call_id: {messages_text}"
    );
    assert!(messages_text.contains("tool-task-1"));

    let loaded = vault.load(conversation.id).unwrap();
    match &loaded.messages[0].content[1] {
        ContentBlock::TaskRef {
            origin_tool_call_id,
            ..
        } => assert_eq!(origin_tool_call_id.as_deref(), Some("tool-task-1")),
        other => panic!("expected task_ref block, got {other:?}"),
    }
    assert_eq!(loaded.messages[0].content[1], block);
}

#[tokio::test]
async fn task_cancel_routes_to_server() {
    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "cancelable_task")
        .map(|tool| tool.exposed_name.clone())
        .unwrap();
    let mut rx = gateway.subscribe();
    gateway.call_tool(&exposed, json!({})).await.unwrap();

    let task_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskStarted { state } = event {
                return state.task_id;
            }
        }
    })
    .await
    .expect("task started");

    gateway.cancel_task(&task_id).await.expect("cancel task");
    let cancelled = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskCompleted { state, .. } = event
                && state.task_id == task_id
            {
                return state;
            }
        }
    })
    .await
    .expect("cancelled task event");
    assert_eq!(cancelled.status, TaskStatus::Failed);
}

#[tokio::test]
async fn task_failure_terminal_state() {
    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "cancelable_task")
        .map(|tool| tool.exposed_name.clone())
        .unwrap();
    let mut rx = gateway.subscribe();
    gateway.call_tool(&exposed, json!({})).await.unwrap();

    let task_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskStarted { state } = event {
                return state.task_id;
            }
        }
    })
    .await
    .expect("task started");

    gateway.cancel_task(&task_id).await.expect("cancel task");
    let failed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskCompleted { state, .. } = event
                && state.task_id == task_id
            {
                return state;
            }
        }
    })
    .await
    .expect("failed task event");
    assert_eq!(failed.status, TaskStatus::Failed);

    // Terminal tasks reject a second cancel attempt.
    assert!(gateway.cancel_task(&task_id).await.is_err());
}

#[tokio::test]
async fn task_survives_background_resume() {
    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let (_exposed, task_id) = start_progress_task(Arc::clone(&gateway)).await;
    gateway.suspend_task_polling(&task_id).await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    gateway.resume_task_polling(&task_id).await.expect("resume");

    let mut rx = gateway.subscribe();
    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskCompleted { state, .. } = event
                && state.task_id == task_id
            {
                return state;
            }
        }
    })
    .await
    .expect("task completed after resume");
    assert_eq!(completed.status, TaskStatus::Completed);
}

#[tokio::test]
async fn task_subscribe_updates_live_card() {
    let command = env!("CARGO_BIN_EXE_m7-task-subscribe-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("task-subscribe", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.expect("list tools");
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "subscribe_task")
        .map(|tool| tool.exposed_name.clone())
        .expect("subscribe_task");
    let mut rx = gateway.subscribe();
    gateway
        .call_tool(&exposed, json!({}))
        .await
        .expect("call subscribe_task");

    let task_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskStarted { state } = event {
                return state.task_id;
            }
        }
    })
    .await
    .expect("task started");

    let mut saw_subscribe_update = false;
    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            match event {
                GatewayEvent::TaskUpdated { state } if state.task_id == task_id => {
                    saw_subscribe_update = true;
                }
                GatewayEvent::TaskCompleted { state, .. } if state.task_id == task_id => {
                    return state;
                }
                _ => {}
            }
        }
    })
    .await
    .expect("task completed via subscribe notifications");
    assert!(saw_subscribe_update, "expected subscribe-driven task update");
    assert_eq!(completed.status, TaskStatus::Completed);
}

#[tokio::test]
async fn task_mid_input_uses_elicitation_path() {
    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "input_task")
        .map(|tool| tool.exposed_name.clone())
        .unwrap();
    let mut rx = gateway.subscribe();
    let call = tokio::spawn({
        let gateway = Arc::clone(&gateway);
        let exposed = exposed.clone();
        async move { gateway.call_tool(&exposed, json!({})).await }
    });

    let requested = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("gateway event");
            if let GatewayEvent::ElicitationRequested {
                request_id,
                message,
                ..
            } = event
            {
                return (request_id, message);
            }
        }
    })
    .await
    .expect("elicitation during task");
    assert!(requested.1.contains("name"));

    gateway
        .respond_elicitation(
            &requested.0,
            ElicitationAction::Accept,
            Some(json!({"name": "tamtri"})),
        )
        .await
        .unwrap();
    call.await.unwrap().expect("tool call");

    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskCompleted { state, .. } = event {
                return state;
            }
        }
    })
    .await
    .expect("task completed after elicitation");
    assert_eq!(completed.status, TaskStatus::Completed);
}

#[tokio::test]
async fn task_failed_persists_task_ref() {
    use chrono::Utc;
    use std::fs;
    use tamtri_core::conversation::{Conversation, Id, Message, Role};
    use tamtri_core::vault::ConversationVault;
    use tamtri_core::vault::fs::FilesystemVault;

    let command = env!("CARGO_BIN_EXE_m7-task-mcp");
    let (tx, _rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("tasks", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "cancelable_task")
        .map(|tool| tool.exposed_name.clone())
        .unwrap();
    let mut rx = gateway.subscribe();
    gateway
        .call_tool_with_meta(
            &exposed,
            json!({}),
            Some(json!({"toolCallId": "tool-task-cancel"})),
        )
        .await
        .unwrap();

    let task_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskStarted { state } = event {
                return state.task_id;
            }
        }
    })
    .await
    .expect("task started");

    gateway.cancel_task(&task_id).await.expect("cancel task");
    let failed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let GatewayEvent::TaskCompleted { state, .. } = event
                && state.task_id == task_id
            {
                return state;
            }
        }
    })
    .await
    .expect("failed task event");
    assert_eq!(failed.status, TaskStatus::Failed);
    assert_eq!(
        failed.origin_tool_call_id.as_deref(),
        Some("tool-task-cancel")
    );

    let block = ContentBlock::TaskRef {
        task_id: failed.task_id.clone(),
        status: failed.status.clone(),
        title: failed.title.clone(),
        result_summary: failed.result.as_ref().map(|value| value.to_string()),
        origin_tool_call_id: failed.origin_tool_call_id.clone(),
    };

    let dir = tempfile::tempdir().unwrap();
    let vault = FilesystemVault::new(dir.path().to_path_buf()).unwrap();
    let mut conversation = Conversation::new("Task failed replay");
    conversation.push_message(Message {
        id: Id::now_v7(),
        role: Role::Assistant,
        harness_id: None,
        content: vec![
            ContentBlock::ToolCall {
                id: "tool-task-cancel".into(),
                name: "tasks__cancelable_task".into(),
                input: json!({}),
            },
            block.clone(),
        ],
        created_at: Utc::now(),
    });
    vault.create(&conversation).unwrap();

    let messages_path = dir
        .path()
        .join("conversations")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("messages.jsonl");
    let messages_text = fs::read_to_string(&messages_path).unwrap();
    assert!(
        messages_text.contains("task_ref"),
        "messages.jsonl should include task_ref: {messages_text}"
    );
    assert!(messages_text.contains("\"failed\""));
    assert!(messages_text.contains("tool-task-cancel"));

    let loaded = vault.load(conversation.id).unwrap();
    match &loaded.messages[0].content[1] {
        ContentBlock::TaskRef {
            status,
            origin_tool_call_id,
            ..
        } => {
            assert_eq!(*status, TaskStatus::Failed);
            assert_eq!(origin_tool_call_id.as_deref(), Some("tool-task-cancel"));
        }
        other => panic!("expected task_ref block, got {other:?}"),
    }
    assert_eq!(loaded.messages[0].content[1], block);
}
