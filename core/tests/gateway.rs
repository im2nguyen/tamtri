use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tamtri_core::config::{
    CredentialBinding, CredentialTarget, GatewayConfig, GatewayScope, GatewayServerConfig,
    GatewayTransport,
};
use tamtri_core::mcp::gateway::{CredentialResolver, GatewayEvent, McpGateway, NoCredentials};
use tamtri_core::mcp::server::serve_gateway_transport;
use tamtri_core::rpc::jsonrpc::{
    IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
};
use tamtri_core::rpc::transport::Transport;
use tamtri_core::{CoreError, Result};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

struct StaticCredentials(HashMap<String, String>);

#[async_trait]
impl CredentialResolver for StaticCredentials {
    async fn resolve(&self, credential_ref: &str) -> Result<Option<String>> {
        Ok(self.0.get(credential_ref).cloned())
    }
}

struct AgentTransport {
    incoming: VecDeque<IncomingMessage>,
    responses: Arc<Mutex<Vec<JsonRpcResponse>>>,
    expected_responses: usize,
}

#[async_trait]
impl Transport for AgentTransport {
    async fn send_request(&mut self, _req: &JsonRpcRequest) -> Result<()> {
        Ok(())
    }

    async fn send_notification(&mut self, _note: &JsonRpcNotification) -> Result<()> {
        Ok(())
    }

    async fn send_response(&mut self, resp: &JsonRpcResponse) -> Result<()> {
        self.responses.lock().await.push(resp.clone());
        Ok(())
    }

    async fn recv(&mut self) -> Result<IncomingMessage> {
        if let Some(message) = self.incoming.pop_front() {
            return Ok(message);
        }
        loop {
            if self.responses.lock().await.len() >= self.expected_responses {
                return Err(CoreError::TransportClosed);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

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
async fn gateway_tools_call_routes_to_downstream() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![stdio_server("mock", command)],
        },
        Arc::new(NoCredentials),
        Some(tx),
    )
    .unwrap();

    let tools = gateway.list_tools().await.unwrap();
    assert_eq!(tools[0].exposed_name, "mock__echo");
    let result = gateway
        .call_tool("mock__echo", json!({"message": "hello"}))
        .await
        .unwrap();
    assert_eq!(
        result.structured_content.unwrap()["echo"]["message"],
        "hello"
    );

    let mut saw_route = false;
    while let Ok(event) = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
        let Some(event) = event else { break };
        if matches!(event, GatewayEvent::ToolRouted { exposed_name, .. } if exposed_name == "mock__echo")
        {
            saw_route = true;
            break;
        }
    }
    assert!(saw_route);
}

#[tokio::test]
async fn gateway_resources_route_to_downstream() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![stdio_server("mock", command)],
        },
        Arc::new(NoCredentials),
        None,
    )
    .unwrap();

    let resources = gateway.list_resources().await.unwrap();
    assert_eq!(
        resources[0].exposed_uri,
        "tamtri://gateway/mock/mock_report"
    );
    let result = gateway
        .read_resource("tamtri://gateway/mock/mock_report")
        .await
        .unwrap();
    assert_eq!(result.contents[0]["text"], "mock resource");
    assert_eq!(result.contents[0]["uri"], "mock://report");
}

#[tokio::test]
async fn gateway_prompts_route_to_downstream() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![stdio_server("mock", command)],
        },
        Arc::new(NoCredentials),
        None,
    )
    .unwrap();

    let prompts = gateway.list_prompts().await.unwrap();
    assert_eq!(prompts[0].exposed_name, "mock__summarize");
    let result = gateway
        .get_prompt("mock__summarize", json!({"topic": "tamtri"}))
        .await
        .unwrap();
    assert_eq!(result.description.as_deref(), Some("Summarize something"));
    assert_eq!(result.messages[0]["role"], "user");
}

#[tokio::test]
async fn agent_facing_server_exposes_resources_and_prompts() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let gateway = Arc::new(
        McpGateway::new(
            GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![stdio_server("mock", command)],
            },
            Arc::new(NoCredentials),
            None,
        )
        .unwrap(),
    );
    let responses = Arc::new(Mutex::new(Vec::new()));
    let transport = AgentTransport {
        incoming: VecDeque::from(vec![
            IncomingMessage::Request(JsonRpcRequest::new(
                RequestId::Number(1),
                "resources/list",
                None,
            )),
            IncomingMessage::Request(JsonRpcRequest::new(
                RequestId::Number(2),
                "prompts/list",
                None,
            )),
        ]),
        responses: Arc::clone(&responses),
        expected_responses: 2,
    };

    serve_gateway_transport(Box::new(transport), gateway)
        .await
        .unwrap();

    let responses = responses.lock().await;
    assert_eq!(
        responses[0].result.as_ref().unwrap()["resources"][0]["uri"],
        "tamtri://gateway/mock/mock_report"
    );
    assert_eq!(
        responses[1].result.as_ref().unwrap()["prompts"][0]["name"],
        "mock__summarize"
    );
}

#[tokio::test]
async fn credential_injection_redacts_events() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let mut server = stdio_server("mock", command);
    server.credentials = vec![CredentialBinding {
        credential_ref: "keychain://mock".to_string(),
        target: CredentialTarget::EnvVar {
            name: "MOCK_TOKEN".to_string(),
        },
    }];
    let (tx, mut rx) = mpsc::unbounded_channel();
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![server],
        },
        Arc::new(StaticCredentials(HashMap::from([(
            "keychain://mock".to_string(),
            "super-secret".to_string(),
        )]))),
        Some(tx),
    )
    .unwrap();

    gateway.list_tools().await.unwrap();
    let event = rx.recv().await.unwrap();
    assert!(matches!(
        event,
        GatewayEvent::CredentialInjected {
            ref credential_ref,
            ref target_kind,
            ..
        } if credential_ref == "keychain://mock" && target_kind == "env_var"
    ));
    assert!(!format!("{event:?}").contains("super-secret"));
}
