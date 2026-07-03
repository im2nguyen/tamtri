use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tamtri_core::config::{GatewayConfig, GatewayScope, GatewayServerConfig, GatewayTransport};
use tamtri_core::conversation::ElicitationAction;
use tamtri_core::mcp::elicitation::{schema_is_renderable, schema_looks_secret};
use tamtri_core::mcp::gateway::{GatewayEvent, McpGateway, NoCredentials};
use tokio::sync::mpsc;

fn stdio_server(id: &str, command: &str) -> GatewayServerConfig {
    stdio_server_with_env(id, command, Vec::new())
}

fn stdio_server_with_env(
    id: &str,
    command: &str,
    env: Vec<(String, String)>,
) -> GatewayServerConfig {
    GatewayServerConfig {
        id: id.to_string(),
        display_name: id.to_string(),
        enabled: true,
        scope: GatewayScope::Project,
        transport: GatewayTransport::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env,
        },
        timeout_secs: None,
        credentials: Vec::new(),
        oauth: None,
    }
}

#[tokio::test]
async fn gateway_elicitation_form_accept_round_trip() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "elicit")
        .map(|tool| tool.exposed_name.clone())
        .expect("elicit tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move {
        gateway_for_call
            .call_tool_with_meta(
                &exposed,
                json!({}),
                Some(json!({"toolCallId": "tool-42"})),
            )
            .await
    });

    let requested = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("gateway event");
            if let GatewayEvent::ElicitationRequested {
                request_id,
                origin_tool_call_id,
                message,
                ..
            } = event
            {
                return (request_id, origin_tool_call_id, message);
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    assert_eq!(requested.1.as_deref(), Some("tool-42"));
    assert!(requested.2.contains("name"));

    gateway
        .respond_elicitation(
            &requested.0,
            ElicitationAction::Accept,
            Some(json!({"name": "tamtri"})),
        )
        .await
        .unwrap();

    let result = call_task.await.unwrap().unwrap();
    assert_eq!(result.structured_content.unwrap()["name"], "tamtri");
}

#[tokio::test]
async fn concurrent_tool_calls_keep_elicitation_correlation() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let tools = gateway.list_tools().await.unwrap();
    let elicit = tools
        .iter()
        .find(|tool| tool.original_name == "elicit")
        .map(|tool| tool.exposed_name.clone())
        .expect("elicit tool");
    let echo = tools
        .iter()
        .find(|tool| tool.original_name == "echo")
        .map(|tool| tool.exposed_name.clone())
        .expect("echo tool");

    let gateway_for_elicit = Arc::clone(&gateway);
    let elicit_task = tokio::spawn(async move {
        gateway_for_elicit
            .call_tool_with_meta(
                &elicit,
                json!({}),
                Some(json!({"toolCallId": "tool-elicit"})),
            )
            .await
    });

    let gateway_for_echo = Arc::clone(&gateway);
    let echo_task = tokio::spawn(async move {
        gateway_for_echo
            .call_tool_with_meta(
                &echo,
                json!({"ping": true}),
                Some(json!({"toolCallId": "tool-echo"})),
            )
            .await
    });

    let requested = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested {
                request_id,
                origin_tool_call_id,
                ..
            }) = rx.recv().await
            {
                return (request_id, origin_tool_call_id);
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    assert_eq!(requested.1.as_deref(), Some("tool-elicit"));

    gateway
        .respond_elicitation(
            &requested.0,
            ElicitationAction::Accept,
            Some(json!({"name": "parallel"})),
        )
        .await
        .unwrap();

    let elicit_result = elicit_task.await.unwrap().unwrap();
    assert_eq!(elicit_result.structured_content.unwrap()["name"], "parallel");

    let echo_result = echo_task.await.unwrap().unwrap();
    assert_eq!(echo_result.structured_content.unwrap()["echo"]["ping"], true);
}

#[tokio::test]
async fn gateway_elicitation_decline_round_trip() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit")
        .map(|tool| tool.exposed_name)
        .expect("elicit tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move { gateway_for_call.call_tool(&exposed, json!({})).await });

    let request_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested { request_id, .. }) = rx.recv().await {
                return request_id;
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    gateway
        .respond_elicitation(&request_id, ElicitationAction::Decline, None)
        .await
        .unwrap();

    let result = call_task.await.unwrap().unwrap();
    assert!(
        result.content[0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("decline")
    );
}

#[tokio::test]
async fn gateway_elicitation_twenty_questions_form_accept_round_trip() {
    let command = env!("CARGO_BIN_EXE_twenty-questions-mcp");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server_with_env(
                    "twenty_questions",
                    command,
                    vec![("TWENTY_QUESTIONS_SEED".to_string(), "42".to_string())],
                )],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let start = gateway
        .call_tool("twenty_questions__start_game", json!({}))
        .await
        .unwrap();
    let game_id = start.structured_content.unwrap()["gameId"]
        .as_u64()
        .expect("game id");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move {
        gateway_for_call
            .call_tool(
                "twenty_questions__submit_question",
                json!({ "gameId": game_id }),
            )
            .await
    });

    let request_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested {
                request_id,
                message,
                schema,
                ..
            }) = rx.recv().await
            {
                assert!(message.contains("yes/no"));
                assert!(schema.is_some());
                return request_id;
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    gateway
        .respond_elicitation(
            &request_id,
            ElicitationAction::Accept,
            Some(json!({ "question": "Is it an animal?" })),
        )
        .await
        .unwrap();

    let result = call_task.await.unwrap().unwrap();
    let structured = result.structured_content.unwrap();
    assert_eq!(structured["answer"], "no");
    assert_eq!(structured["question"], "Is it an animal?");
    assert_eq!(structured["turnsRemaining"], 19);
}

#[tokio::test]
async fn gateway_elicitation_url_accept_round_trip() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit_url")
        .map(|tool| tool.exposed_name)
        .expect("elicit_url tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move { gateway_for_call.call_tool(&exposed, json!({})).await });

    let (request_id, url) = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested {
                request_id,
                mode,
                url,
                message,
                ..
            }) = rx.recv().await
            {
                assert_eq!(mode, tamtri_core::conversation::ElicitationMode::Url);
                assert!(message.contains("Sign in"));
                return (request_id, url.expect("url elicitation url"));
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    assert!(url.starts_with("https://example.com/"));

    gateway
        .respond_elicitation(&request_id, ElicitationAction::Accept, None)
        .await
        .unwrap();

    let result = call_task.await.unwrap().unwrap();
    assert_eq!(result.structured_content.unwrap()["elicitation"], "accept");
}

#[tokio::test]
async fn gateway_elicitation_url_decline_round_trip() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit_url")
        .map(|tool| tool.exposed_name)
        .expect("elicit_url tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move { gateway_for_call.call_tool(&exposed, json!({})).await });

    let request_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested { request_id, .. }) = rx.recv().await {
                return request_id;
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    gateway
        .respond_elicitation(&request_id, ElicitationAction::Decline, None)
        .await
        .unwrap();

    let result = call_task.await.unwrap().unwrap();
    assert_eq!(result.structured_content.unwrap()["elicitation"], "decline");
}

#[test]
fn url_elicitation_requires_https() {
    use tamtri_core::mcp::elicitation::validate_elicitation_url;
    assert!(validate_elicitation_url("http://example.com/login").is_err());
    assert!(validate_elicitation_url("https://example.com/login").is_ok());
}

#[test]
fn url_elicitation_rejects_userinfo() {
    use tamtri_core::mcp::elicitation::validate_elicitation_url;
    assert!(validate_elicitation_url("https://user:pass@example.com/path").is_err());
}

#[tokio::test]
async fn gateway_elicitation_user_cancel_round_trip() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit")
        .map(|tool| tool.exposed_name)
        .expect("elicit tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move { gateway_for_call.call_tool(&exposed, json!({})).await });

    let request_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested { request_id, .. }) = rx.recv().await {
                return request_id;
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    gateway
        .respond_elicitation(&request_id, ElicitationAction::Cancel, None)
        .await
        .unwrap();

    let result = call_task.await.unwrap().unwrap();
    assert_eq!(
        result.structured_content.unwrap()["elicitation"],
        "cancel"
    );
}

#[tokio::test]
async fn gateway_elicitation_cancel_on_run_cancel() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit")
        .map(|tool| tool.exposed_name)
        .expect("elicit tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move { gateway_for_call.call_tool(&exposed, json!({})).await });

    let request_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested { request_id, .. }) = rx.recv().await {
                return request_id;
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    gateway.cancel_pending_elicitations().await;

    let result = call_task.await.unwrap().unwrap();
    assert!(
        result.content[0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("cancel")
    );
    let _ = request_id;
}

#[tokio::test]
async fn elicitation_nested_under_origin_tool_call() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit")
        .map(|tool| tool.exposed_name)
        .expect("elicit tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move {
        gateway_for_call
            .call_tool_with_meta(
                &exposed,
                json!({}),
                Some(json!({"toolCallId": "parent-tool"})),
            )
            .await
    });

    let (request_id, origin) = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested {
                request_id,
                origin_tool_call_id,
                ..
            }) = rx.recv().await
            {
                return (request_id, origin_tool_call_id);
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    assert_eq!(origin.as_deref(), Some("parent-tool"));
    gateway
        .respond_elicitation(
            &request_id,
            ElicitationAction::Accept,
            Some(json!({"name": "nested"})),
        )
        .await
        .unwrap();
    call_task.await.unwrap().unwrap();
}

#[tokio::test]
async fn agent_receives_tool_result_after_elicitation() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit")
        .map(|tool| tool.exposed_name)
        .expect("elicit tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move {
        gateway_for_call
            .call_tool(&exposed, json!({}))
            .await
            .map(|result| result.structured_content)
    });

    let request_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested { request_id, .. }) = rx.recv().await {
                return request_id;
            }
        }
    })
    .await
    .expect("elicitation request timed out");

    gateway
        .respond_elicitation(
            &request_id,
            ElicitationAction::Accept,
            Some(json!({"name": "agent-visible"})),
        )
        .await
        .unwrap();

    let structured = call_task.await.unwrap().unwrap().unwrap();
    assert_eq!(structured["name"], "agent-visible");
}

#[test]
fn elicitation_secret_field_rejected() {
    let schema = json!({
        "type": "object",
        "properties": {
            "api_key": { "type": "string", "title": "API key" }
        }
    });
    assert!(schema_looks_secret(&schema));
}

#[test]
fn elicitation_complex_schema_graceful_fallback() {
    let schema = json!({
        "type": "object",
        "properties": {
            "address": {
                "type": "object",
                "properties": {
                    "street": { "type": "string" }
                }
            }
        }
    });
    assert!(!schema_is_renderable(&schema));
}

#[tokio::test]
async fn gateway_elicitation_secret_schema_auto_decline_round_trip() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit_secret")
        .map(|tool| tool.exposed_name)
        .expect("elicit_secret tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move {
        gateway_for_call.call_tool(&exposed, json!({})).await
    });

    let saw_elicitation = tokio::time::timeout(Duration::from_millis(500), async {
        while let Some(event) = rx.recv().await {
            if matches!(event, GatewayEvent::ElicitationRequested { .. }) {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    assert!(
        !saw_elicitation,
        "secret schemas must auto-decline without surfacing elicitation"
    );

    let result = call_task.await.unwrap().unwrap();
    assert_eq!(result.structured_content.unwrap()["elicitation"], "decline");
}

#[tokio::test]
async fn gateway_elicitation_complex_schema_fallback_round_trip() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            Some(tx),
        )
        .unwrap(),
    );

    let exposed = gateway
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|tool| tool.original_name == "elicit_complex")
        .map(|tool| tool.exposed_name)
        .expect("elicit_complex tool");

    let gateway_for_call = Arc::clone(&gateway);
    let call_task = tokio::spawn(async move {
        gateway_for_call.call_tool(&exposed, json!({})).await
    });

    let request_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(GatewayEvent::ElicitationRequested {
                request_id,
                schema,
                ..
            }) = rx.recv().await
            {
                assert!(
                    schema
                        .as_ref()
                        .is_some_and(|value| !schema_is_renderable(value)),
                    "expected non-renderable schema to pass through for UI fallback"
                );
                return request_id;
            }
        }
    })
    .await
    .expect("complex schema elicitation request timed out");

    gateway
        .respond_elicitation(&request_id, ElicitationAction::Decline, None)
        .await
        .unwrap();

    let result = call_task.await.unwrap().unwrap();
    assert_eq!(result.structured_content.unwrap()["elicitation"], "decline");
}

#[test]
fn url_elicitation_redacts_query_in_events() {
    use tamtri_core::mcp::elicitation::audit_safe_elicitation_url;
    let redacted = audit_safe_elicitation_url("https://example.com/oauth?client_id=demo&state=abc");
    assert!(!redacted.contains("client_id"));
    assert_eq!(redacted, "https://example.com/oauth");
}
