use std::sync::Arc;

use futures_util::StreamExt;
use serde_json::json;
use tamtri_core::Result;
use tamtri_core::config::{GatewayConfig, GatewayScope, GatewayServerConfig, GatewayTransport};
use tamtri_core::mcp::endpoint::start_loopback_gateway;
use tamtri_core::mcp::gateway::{McpGateway, NoCredentials};

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
    }
}

#[tokio::test]
async fn loopback_gateway_serves_agent_http_requests() -> Result<()> {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let gateway = Arc::new(McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![stdio_server("mock", command)],
        },
        Arc::new(NoCredentials),
        None,
    )?);
    let endpoint = start_loopback_gateway(gateway).await?;
    let client = reqwest::Client::new();
    let response: serde_json::Value = client
        .post(endpoint.mcp_ref().endpoint)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "mock__echo", "arguments": {"message": "hello"}}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    endpoint.shutdown().await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_eq!(
        response["result"]["structuredContent"]["echo"]["message"],
        "hello"
    );
    Ok(())
}

#[tokio::test]
async fn loopback_gateway_pushes_sse_notifications() -> Result<()> {
    let gateway = Arc::new(McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: Vec::new(),
        },
        Arc::new(NoCredentials),
        None,
    )?);
    let endpoint = start_loopback_gateway(Arc::clone(&gateway)).await?;
    let client = reqwest::Client::new();
    let mut stream = client
        .get(endpoint.mcp_ref().endpoint)
        .send()
        .await
        .unwrap()
        .bytes_stream();
    gateway.agent_cancelled(json!({"requestId": 1}));

    let mut body = String::new();
    while let Some(chunk) = stream.next().await {
        body.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
        if body.contains("notifications/cancelled") {
            break;
        }
    }
    endpoint.shutdown().await;

    assert!(body.contains("event: message"));
    assert!(body.contains("\"method\":\"notifications/cancelled\""));
    Ok(())
}
