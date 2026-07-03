//! Integration tests for the public `McpClient` surface.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tamtri_core::mcp::jsonrpc::{
    IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
};
use tamtri_core::mcp::{McpClient, McpClientConfig, McpClientEvent};
use tamtri_core::rpc::transport::Transport;
use tamtri_core::{CoreError, Result};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

struct TrackingTransport {
    incoming: VecDeque<IncomingMessage>,
    responses: Arc<Mutex<Vec<JsonRpcResponse>>>,
    requests_sent: usize,
    responses_delivered: usize,
}

impl TrackingTransport {
    fn new(incoming: Vec<IncomingMessage>, responses: Arc<Mutex<Vec<JsonRpcResponse>>>) -> Self {
        Self {
            incoming: incoming.into(),
            responses,
            requests_sent: 0,
            responses_delivered: 0,
        }
    }
}

#[async_trait]
impl Transport for TrackingTransport {
    async fn send_request(&mut self, _req: &JsonRpcRequest) -> Result<()> {
        self.requests_sent += 1;
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
        if matches!(self.incoming.front(), Some(IncomingMessage::Response(_)))
            && self.responses_delivered >= self.requests_sent
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
            return Err(CoreError::TransportClosed);
        }
        if let Some(message) = self.incoming.pop_front() {
            if matches!(message, IncomingMessage::Response(_)) {
                self.responses_delivered += 1;
            }
            return Ok(message);
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
        Err(CoreError::TransportClosed)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

struct MockTransport {
    incoming: VecDeque<IncomingMessage>,
    never_recv: bool,
    requests_sent: usize,
    responses_delivered: usize,
}

impl MockTransport {
    fn new(incoming: Vec<IncomingMessage>) -> Self {
        Self {
            incoming: incoming.into(),
            never_recv: false,
            requests_sent: 0,
            responses_delivered: 0,
        }
    }

    fn never_recv() -> Self {
        Self {
            incoming: VecDeque::new(),
            never_recv: true,
            requests_sent: 0,
            responses_delivered: 0,
        }
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn send_request(&mut self, _req: &JsonRpcRequest) -> Result<()> {
        self.requests_sent += 1;
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

    async fn recv(&mut self) -> Result<IncomingMessage> {
        if self.never_recv {
            tokio::time::sleep(Duration::from_secs(60)).await;
            return Err(CoreError::TransportClosed);
        }
        if matches!(self.incoming.front(), Some(IncomingMessage::Response(_)))
            && self.responses_delivered >= self.requests_sent
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
            return Err(CoreError::TransportClosed);
        }
        if let Some(message) = self.incoming.pop_front() {
            if matches!(message, IncomingMessage::Response(_)) {
                self.responses_delivered += 1;
            }
            return Ok(message);
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
        Err(CoreError::TransportClosed)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

fn tools_page(name: &str) -> serde_json::Value {
    json!({
        "tools": [{
            "name": name,
            "description": name,
            "inputSchema": {"type": "object"}
        }]
    })
}

#[tokio::test]
async fn mcp_client_concurrent_requests_correlate() {
    let client = McpClient::connect_test(
        Box::new(MockTransport::new(vec![
            IncomingMessage::Response(JsonRpcResponse::success(
                RequestId::Number(2),
                tools_page("b"),
            )),
            IncomingMessage::Response(JsonRpcResponse::success(
                RequestId::Number(1),
                tools_page("a"),
            )),
        ])),
        McpClientConfig::default(),
        None,
    );

    let (a, b) = tokio::join!(client.list_tools(), client.list_tools());

    let a_tools = a.unwrap();
    let b_tools = b.unwrap();
    assert_eq!(a_tools[0].name, "a");
    assert_eq!(b_tools[0].name, "b");
}

#[tokio::test]
async fn mcp_client_inbound_progress_while_pending() {
    let (events_tx, mut events_rx) = mpsc::unbounded_channel();
    let client = McpClient::connect_test(
        Box::new(MockTransport::new(vec![
            IncomingMessage::Notification(JsonRpcNotification::new(
                "notifications/progress",
                Some(json!({"progress": 0.5, "message": "halfway"})),
            )),
            IncomingMessage::Response(JsonRpcResponse::success(
                RequestId::Number(1),
                json!({"tools": []}),
            )),
        ])),
        McpClientConfig::default(),
        Some(events_tx),
    );

    let list = client.list_tools();
    let (event, result) = tokio::join!(events_rx.recv(), list);
    assert!(matches!(
        event,
        Some(McpClientEvent::Progress { params }) if params["message"] == "halfway"
    ));
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn ping_while_downstream_call_pending() {
    let responses = Arc::new(Mutex::new(Vec::new()));
    let client = McpClient::connect_test(
        Box::new(TrackingTransport::new(
            vec![
                IncomingMessage::Request(JsonRpcRequest::new(
                    RequestId::String("srv-ping".to_string()),
                    "ping",
                    None,
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({"tools": []}),
                )),
            ],
            Arc::clone(&responses),
        )),
        McpClientConfig::default(),
        None,
    );

    // tools/list returns before the inbound driver must answer the queued ping, so wait
    // briefly for the auto-response the driver sends.
    client.list_tools().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if !responses.lock().await.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("ping response should be sent while downstream call was pending");

    let sent = responses.lock().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].id, RequestId::String("srv-ping".to_string()));
    assert_eq!(sent[0].result, Some(json!({})));
}

#[tokio::test]
async fn mcp_client_timeout_removes_pending() {
    tokio::time::pause();

    let client = McpClient::connect_test(
        Box::new(MockTransport::never_recv()),
        McpClientConfig {
            init_timeout: Duration::from_millis(100),
            call_timeout: Duration::from_millis(100),
        },
        None,
    );

    let first = client.list_tools();
    tokio::time::advance(Duration::from_millis(200)).await;
    assert!(matches!(
        first.await,
        Err(CoreError::Timeout { method }) if method == "tools/list"
    ));
    assert!(matches!(
        client.list_tools().await,
        Err(CoreError::TransportClosed)
    ));
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
