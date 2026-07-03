//! Integration tests for the public `McpClient` surface.
//!
//! `mcp_client_concurrent_requests_correlate` mirrors
//! `rpc::dispatch::tests::concurrent_requests_correlate`: McpClient delegates
//! request correlation to `RpcConnection`, so the behavior is validated here at
//! the client integration boundary with the same out-of-order response fixture.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tamtri_core::mcp::{McpClient, McpClientConfig};
use tamtri_core::rpc::dispatch::RpcConnection;
use tamtri_core::rpc::jsonrpc::{IncomingMessage as TransportMessage, JsonRpcRequest, JsonRpcResponse, RequestId};
use tamtri_core::rpc::transport::Transport;
use tamtri_core::{CoreError, Result};
use tokio::sync::Mutex;

struct DispatchMockTransport {
    incoming: VecDeque<TransportMessage>,
    sent: Arc<Mutex<Vec<JsonRpcRequest>>>,
    recv_delay: Duration,
}

#[async_trait]
impl Transport for DispatchMockTransport {
    async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()> {
        self.sent.lock().await.push(req.clone());
        Ok(())
    }

    async fn send_notification(
        &mut self,
        _note: &tamtri_core::rpc::jsonrpc::JsonRpcNotification,
    ) -> Result<()> {
        Ok(())
    }

    async fn send_response(&mut self, _resp: &JsonRpcResponse) -> Result<()> {
        Ok(())
    }

    async fn recv(&mut self) -> Result<TransportMessage> {
        tokio::time::sleep(self.recv_delay).await;
        self.incoming
            .pop_front()
            .ok_or(CoreError::TransportClosed)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn mcp_client_concurrent_requests_correlate() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let transport = DispatchMockTransport {
        incoming: VecDeque::from(vec![
            TransportMessage::Response(JsonRpcResponse::success(
                RequestId::Number(2),
                json!("b"),
            )),
            TransportMessage::Response(JsonRpcResponse::success(
                RequestId::Number(1),
                json!("a"),
            )),
        ]),
        sent: Arc::clone(&sent),
        recv_delay: Duration::from_millis(1),
    };
    let (handle, _inbound) = RpcConnection::start(Box::new(transport));

    let (a, b) = tokio::join!(
        handle.request("a", None, Duration::from_secs(1)),
        handle.request("b", None, Duration::from_secs(1))
    );

    assert_eq!(a.unwrap(), json!("a"));
    assert_eq!(b.unwrap(), json!("b"));
    assert_eq!(sent.lock().await.len(), 2);
}

#[tokio::test]
async fn integration_echo_tool() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let client = McpClient::connect_stdio(command, &[], &[], McpClientConfig::default())
        .await
        .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools.len(), 6);
    assert!(tools.iter().any(|tool| tool.name == "echo"));
    assert!(tools.iter().any(|tool| tool.name == "fail"));

    let result = client
        .call_tool("echo", json!({"message": "hello"}), None)
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content.unwrap()["echo"]["message"],
        "hello"
    );

    let resources = client.list_resources().await.unwrap();
    assert_eq!(resources[0].uri, "mock://report");
    let resource = client.read_resource("mock://report").await.unwrap();
    assert_eq!(resource.contents[0]["text"], "mock resource");

    let prompts = client.list_prompts().await.unwrap();
    assert_eq!(prompts[0].name, "summarize");
    let prompt = client
        .get_prompt("summarize", json!({"topic": "tamtri"}))
        .await
        .unwrap();
    assert_eq!(prompt.messages[0]["role"], "user");

    client.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires npx and network; spawns @modelcontextprotocol/server-everything"]
async fn integration_server_everything() {
    let client = McpClient::connect_stdio(
        "npx",
        &[
            "-y".to_string(),
            "@modelcontextprotocol/server-everything".to_string(),
        ],
        &[],
        McpClientConfig::default(),
    )
    .await
    .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert!(!tools.is_empty());

    client.close().await.unwrap();
}
