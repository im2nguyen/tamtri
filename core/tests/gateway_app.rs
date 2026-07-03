use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tamtri_core::config::{GatewayConfig, GatewayScope, GatewayServerConfig, GatewayTransport};
use tamtri_core::conversation::{ContentBlock, Conversation, Message, Role};
use tamtri_core::mcp::app::{MCP_APP_MIME, Origin, template_from_resource_contents};
use tamtri_core::mcp::capabilities::{FeatureStatus, TamtriFeatureSupport};
use tamtri_core::mcp::gateway::{GatewayEvent, McpGateway, NoCredentials};
use tamtri_core::vault::fs::FilesystemVault;
use tamtri_core::vault::ConversationVault;
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
        timeout_secs: None,
        credentials: Vec::new(),
        oauth: None,
    }
}

#[tokio::test]
async fn app_template_declared() {
    let command = env!("CARGO_BIN_EXE_m7-app-mcp");
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("m7-app", command)],
            },
            Arc::new(NoCredentials),
            None,
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "show_app")
        .map(|tool| tool.exposed_name.clone())
        .expect("show_app tool");

    let result = gateway
        .call_tool_with_meta(
            &exposed,
            json!({}),
            Some(json!({"toolCallId": "tool-app-1"})),
        )
        .await
        .expect("tool call");

    assert_eq!(result.structured_content.unwrap()["value"], 42);

    let reports = gateway.probe_server_capabilities().await.unwrap();
    assert_eq!(reports[0].apps, FeatureStatus::Supported);
    assert!(TamtriFeatureSupport::current().apps);
}

#[tokio::test]
async fn app_resource_persists() {
    let command = env!("CARGO_BIN_EXE_m7-app-mcp");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("m7-app", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "show_app")
        .unwrap()
        .exposed_name
        .clone();

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move {
        gateway_for_call
            .call_tool_with_meta(
                &exposed,
                json!({}),
                Some(json!({"toolCallId": "tool-app-2"})),
            )
            .await
    });

    let app_event = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("gateway event");
            if let GatewayEvent::AppReturned {
                origin_tool_call_id,
                server_id,
                template_ref,
                state,
                ..
            } = event
            {
                return (origin_tool_call_id, server_id, template_ref, state);
            }
        }
    })
    .await
    .expect("app returned event timed out");

    assert_eq!(app_event.0.as_deref(), Some("tool-app-2"));
    assert_eq!(app_event.1, "m7-app");
    assert_eq!(app_event.2, "ui://m7-app/demo");
    assert_eq!(app_event.3["title"], "Demo App");

    call_task.await.unwrap().unwrap();

    let block = ContentBlock::AppResource {
        uri: "ui://m7-app/demo".into(),
        template_ref: "ui://m7-app/demo".into(),
        state: app_event.3,
        server_id: Some("m7-app".into()),
        origin_tool_call_id: app_event.0,
    };

    let dir = tempfile::tempdir().unwrap();
    let vault = FilesystemVault::new(dir.path()).unwrap();
    let mut conversation = Conversation::new("App persistence");
    conversation.push_message(Message {
        id: tamtri_core::conversation::Id::now_v7(),
        role: Role::Assistant,
        harness_id: None,
        content: vec![block.clone()],
        created_at: chrono::Utc::now(),
    });
    vault.create(&conversation).unwrap();
    let loaded = vault.load(conversation.id).unwrap();
    assert_eq!(loaded.messages[0].content[0], block);
}

#[test]
fn app_template_declared_origin_loads_from_fixture_shape() {
    let contents = [json!({
        "uri": "ui://m7-app/demo",
        "mimeType": MCP_APP_MIME,
        "text": "<!DOCTYPE html><html><body>demo</body></html>",
        "_meta": {
            "ui": {
                "csp": {
                    "connectDomains": ["https://api.example.com"]
                }
            }
        }
    })];
    let template =
        template_from_resource_contents("m7-app", "ui://m7-app/demo", &contents).unwrap();
    assert_eq!(
        template.allowed_origins,
        vec![Origin("https://api.example.com".into())]
    );
}
