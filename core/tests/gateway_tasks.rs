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
    let tools = gateway.list_tools().await.expect("list tools");
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "progress_task")
        .map(|tool| tool.exposed_name.clone())
        .expect("progress_task");
    let mut rx = gateway.subscribe();
    gateway
        .call_tool(&exposed, json!({}))
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

    let block = ContentBlock::TaskRef {
        task_id: completed.task_id.clone(),
        status: completed.status.clone(),
        title: completed.title.clone(),
        result_summary: completed.result.as_ref().map(|value| value.to_string()),
    };
    assert_eq!(
        block,
        ContentBlock::TaskRef {
            task_id,
            status: TaskStatus::Completed,
            title: completed.title.clone(),
            result_summary: completed.result.as_ref().map(|value| value.to_string()),
        }
    );
    let serialized = serde_json::to_string(&block).expect("serialize task ref");
    assert!(serialized.contains("task_ref"));
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
