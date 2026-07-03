use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::sync::Mutex;

use crate::mcp::jsonrpc::{
    IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, METHOD_NOT_FOUND,
    RequestId,
};
use crate::mcp::protocol::{
    CallToolParams, CallToolResult, ClientCapabilities, Implementation, InitializeParams,
    InitializeResult, ListToolsParams, ListToolsResult, MCP_PROTOCOL_VERSION, ServerCapabilities,
    Tool,
};
use crate::rpc::transport::Transport;
use crate::rpc::transport::stdio::StdioTransport;
use crate::{CoreError, Result};

pub struct McpClientConfig {
    pub init_timeout: Duration,
    pub call_timeout: Duration,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            init_timeout: Duration::from_secs(30),
            call_timeout: Duration::from_secs(300),
        }
    }
}

pub struct McpClient {
    inner: Mutex<ClientInner>,
    config: McpClientConfig,
    server_info: Option<Implementation>,
    server_capabilities: Option<ServerCapabilities>,
}

struct ClientInner {
    transport: Box<dyn Transport>,
    next_id: i64,
    poisoned: bool,
}

impl McpClient {
    pub async fn connect_stdio(
        command: &str,
        args: &[String],
        env: &[(String, String)],
        config: McpClientConfig,
    ) -> Result<Self> {
        let transport = StdioTransport::spawn(command, args, env).await?;
        let mut client = Self::with_transport(Box::new(transport), config);
        client.initialize().await?;
        client.send_initialized_notification().await?;
        Ok(client)
    }

    fn with_transport(transport: Box<dyn Transport>, config: McpClientConfig) -> Self {
        Self {
            inner: Mutex::new(ClientInner {
                transport,
                next_id: 1,
                poisoned: false,
            }),
            config,
            server_info: None,
            server_capabilities: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "tamtri-core".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        let result: InitializeResult = self
            .request(
                "initialize",
                Some(serde_json::to_value(params)?),
                self.config.init_timeout,
            )
            .await?;
        if result.protocol_version != MCP_PROTOCOL_VERSION {
            tracing::warn!(
                "MCP server negotiated protocol version {}, client target is {}",
                result.protocol_version,
                MCP_PROTOCOL_VERSION
            );
        }
        self.server_info = Some(result.server_info.clone());
        self.server_capabilities = Some(result.capabilities.clone());
        Ok(result)
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let mut tools = Vec::new();
        let mut cursor = None;
        loop {
            let params = ListToolsParams {
                cursor: cursor.clone(),
            };
            let page: ListToolsResult = self
                .request(
                    "tools/list",
                    Some(serde_json::to_value(params)?),
                    self.config.init_timeout,
                )
                .await?;
            tools.extend(page.tools);
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => return Ok(tools),
            }
        }
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult> {
        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };
        self.request(
            "tools/call",
            Some(serde_json::to_value(params)?),
            self.config.call_timeout,
        )
        .await
    }

    pub async fn close(self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.transport.close().await
    }

    pub fn server_info(&self) -> Option<&Implementation> {
        self.server_info.as_ref()
    }

    pub fn server_capabilities(&self) -> Option<&ServerCapabilities> {
        self.server_capabilities.as_ref()
    }

    async fn send_initialized_notification(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        if inner.poisoned {
            return Err(CoreError::TransportClosed);
        }
        inner
            .transport
            .send_notification(&JsonRpcNotification::new("notifications/initialized", None))
            .await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        timeout: Duration,
    ) -> Result<T> {
        let response = self.request_value(method, params, timeout).await?;
        Ok(serde_json::from_value(response)?)
    }

    async fn request_value(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        let mut inner = self.inner.lock().await;
        if inner.poisoned {
            return Err(CoreError::TransportClosed);
        }
        let id = RequestId::Number(inner.next_id);
        inner.next_id += 1;
        let request = JsonRpcRequest::new(id.clone(), method, params);

        let outcome = tokio::time::timeout(timeout, async {
            inner.transport.send_request(&request).await?;
            read_until_response(&mut *inner.transport, &id).await
        })
        .await;

        match outcome {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(err)) => Err(err),
            Err(_) => {
                inner.poisoned = true;
                let _ = inner.transport.close().await;
                Err(CoreError::Timeout {
                    method: method.to_string(),
                })
            }
        }
    }
}

async fn read_until_response(
    transport: &mut dyn Transport,
    expected_id: &RequestId,
) -> Result<serde_json::Value> {
    loop {
        match transport.recv().await? {
            IncomingMessage::Response(response) if &response.id == expected_id => {
                if let Some(error) = response.error {
                    return Err(CoreError::JsonRpc {
                        code: error.code,
                        message: error.message,
                    });
                }
                return response.result.ok_or_else(|| {
                    CoreError::Protocol("response missing result and error".to_string())
                });
            }
            IncomingMessage::Response(response) => {
                tracing::debug!(
                    "ignoring response for unmatched request id {:?}",
                    response.id
                );
            }
            IncomingMessage::Notification(note) => {
                tracing::debug!("received MCP notification {}", note.method);
            }
            IncomingMessage::Request(req) if req.method == "ping" => {
                transport
                    .send_response(&JsonRpcResponse::success(req.id, json!({})))
                    .await?;
            }
            IncomingMessage::Request(req) => {
                tracing::warn!("unsupported MCP server request {}", req.method);
                transport
                    .send_response(&JsonRpcResponse::error(
                        req.id,
                        METHOD_NOT_FOUND,
                        "method not found",
                    ))
                    .await?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use async_trait::async_trait;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tokio::sync::Mutex as TokioMutex;

    use super::*;
    use crate::conversation::ContentBlock;
    use crate::mcp::bridge::tool_result_block;
    use crate::mcp::jsonrpc::JsonRpcError;
    use crate::mcp::protocol::{Tool, ToolsCapability};

    #[derive(Clone, Debug)]
    enum Sent {
        Request(JsonRpcRequest),
        Notification(JsonRpcNotification),
        Response(JsonRpcResponse),
    }

    struct MockTransport {
        incoming: VecDeque<IncomingMessage>,
        sent: Arc<TokioMutex<Vec<Sent>>>,
        never_recv: bool,
    }

    impl MockTransport {
        fn new(incoming: Vec<IncomingMessage>) -> (Self, Arc<TokioMutex<Vec<Sent>>>) {
            let sent = Arc::new(TokioMutex::new(Vec::new()));
            (
                Self {
                    incoming: incoming.into(),
                    sent: Arc::clone(&sent),
                    never_recv: false,
                },
                sent,
            )
        }

        fn never_recv() -> Self {
            Self {
                incoming: VecDeque::new(),
                sent: Arc::new(TokioMutex::new(Vec::new())),
                never_recv: true,
            }
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()> {
            self.sent.lock().await.push(Sent::Request(req.clone()));
            Ok(())
        }

        async fn send_notification(&mut self, note: &JsonRpcNotification) -> Result<()> {
            self.sent
                .lock()
                .await
                .push(Sent::Notification(note.clone()));
            Ok(())
        }

        async fn send_response(&mut self, resp: &JsonRpcResponse) -> Result<()> {
            self.sent.lock().await.push(Sent::Response(resp.clone()));
            Ok(())
        }

        async fn recv(&mut self) -> Result<IncomingMessage> {
            if self.never_recv {
                tokio::time::sleep(Duration::from_secs(60)).await;
                return Err(CoreError::TransportClosed);
            }
            self.incoming.pop_front().ok_or(CoreError::TransportClosed)
        }

        async fn close(&mut self) -> Result<()> {
            Ok(())
        }
    }

    fn init_response() -> IncomingMessage {
        IncomingMessage::Response(JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "mock", "version": "1.0.0"}
            }),
        ))
    }

    fn client_with(
        incoming: Vec<IncomingMessage>,
        config: McpClientConfig,
    ) -> (McpClient, Arc<TokioMutex<Vec<Sent>>>) {
        let (transport, sent) = MockTransport::new(incoming);
        (McpClient::with_transport(Box::new(transport), config), sent)
    }

    #[tokio::test]
    async fn initialize_handshake() {
        let (mut client, sent) = client_with(vec![init_response()], McpClientConfig::default());
        let result = client.initialize().await.unwrap();
        assert_eq!(result.server_info.name, "mock");
        assert_eq!(
            client.server_capabilities(),
            Some(&ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false)
                })
            })
        );
        let sent = sent.lock().await;
        let Sent::Request(req) = &sent[0] else {
            panic!("expected request");
        };
        assert_eq!(req.method, "initialize");
        assert_eq!(
            req.params.as_ref().unwrap()["protocolVersion"],
            MCP_PROTOCOL_VERSION
        );
        assert_eq!(
            req.params.as_ref().unwrap()["clientInfo"]["name"],
            "tamtri-core"
        );
    }

    #[tokio::test]
    async fn sends_initialized_notification() {
        let (transport, sent) = MockTransport::new(vec![init_response()]);
        let mut client = McpClient::with_transport(Box::new(transport), McpClientConfig::default());
        client.initialize().await.unwrap();
        client.send_initialized_notification().await.unwrap();
        let sent = sent.lock().await;
        assert!(matches!(
            &sent[1],
            Sent::Notification(note) if note.method == "notifications/initialized"
        ));
    }

    #[tokio::test]
    async fn list_tools_single_page() {
        let tool =
            json!({"name": "echo", "description": "Echo", "inputSchema": {"type": "object"}});
        let (client, _) = client_with(
            vec![IncomingMessage::Response(JsonRpcResponse::success(
                RequestId::Number(1),
                json!({"tools": [tool]}),
            ))],
            McpClientConfig::default(),
        );
        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools[0].name, "echo");
    }

    #[tokio::test]
    async fn list_tools_paginates() {
        let (client, sent) = client_with(
            vec![
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({"tools": [{"name": "one", "inputSchema": {}}], "nextCursor": "n"}),
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(2),
                    json!({"tools": [{"name": "two", "inputSchema": {}}]}),
                )),
            ],
            McpClientConfig::default(),
        );
        let tools = client.list_tools().await.unwrap();
        assert_eq!(
            tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            vec!["one", "two"]
        );
        let sent = sent.lock().await;
        let Sent::Request(req) = &sent[1] else {
            panic!("expected second request");
        };
        assert_eq!(req.params.as_ref().unwrap()["cursor"], "n");
    }

    #[tokio::test]
    async fn call_tool_returns_result() {
        let (client, _) = client_with(
            vec![IncomingMessage::Response(JsonRpcResponse::success(
                RequestId::Number(1),
                json!({"content": [{"type": "text", "text": "hi"}], "isError": false}),
            ))],
            McpClientConfig::default(),
        );
        let result = client
            .call_tool("echo", json!({"text": "hi"}))
            .await
            .unwrap();
        assert_eq!(result.content[0]["text"], "hi");
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn call_tool_error_flag() {
        let (client, _) = client_with(
            vec![IncomingMessage::Response(JsonRpcResponse::success(
                RequestId::Number(1),
                json!({"content": [{"type": "text", "text": "bad"}], "isError": true}),
            ))],
            McpClientConfig::default(),
        );
        let result = client.call_tool("echo", json!({})).await.unwrap();
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn ignores_interleaved_notification() {
        let (client, _) = client_with(
            vec![
                IncomingMessage::Notification(JsonRpcNotification::new(
                    "notifications/progress",
                    None,
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({"tools": []}),
                )),
            ],
            McpClientConfig::default(),
        );
        assert!(client.list_tools().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn jsonrpc_error_maps_to_core_error() {
        let (client, _) = client_with(
            vec![IncomingMessage::Response(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: RequestId::Number(1),
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: "nope".to_string(),
                    data: None,
                }),
            })],
            McpClientConfig::default(),
        );
        assert!(matches!(
            client.list_tools().await,
            Err(CoreError::JsonRpc { code: -32000, .. })
        ));
    }

    #[test]
    fn result_maps_to_tool_result_block() {
        let result = CallToolResult {
            content: vec![json!({"type": "text", "text": "hi"})],
            is_error: Some(false),
            structured_content: Some(json!({"ok": true})),
        };
        let block = tool_result_block("call-1", &result);
        let ContentBlock::ToolResult { call_id, output } = block else {
            panic!("expected tool result");
        };
        assert_eq!(call_id, "call-1");
        assert_eq!(output["content"][0]["text"], "hi");
        assert_eq!(output["is_error"], false);
        assert_eq!(output["structured_content"]["ok"], true);
    }

    #[test]
    fn classifies_by_field_presence() {
        assert!(matches!(
            IncomingMessage::from_line(r#"{"jsonrpc":"2.0","id":"srv-1","method":"roots/list"}"#)
                .unwrap(),
            IncomingMessage::Request(JsonRpcRequest {
                id: RequestId::String(id),
                ..
            }) if id == "srv-1"
        ));
        assert!(matches!(
            IncomingMessage::from_line(r#"{"jsonrpc":"2.0","method":"notifications/progress"}"#)
                .unwrap(),
            IncomingMessage::Notification(_)
        ));
        assert!(matches!(
            IncomingMessage::from_line(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#).unwrap(),
            IncomingMessage::Response(_)
        ));
    }

    #[test]
    fn response_with_result_and_error_is_protocol_error() {
        assert!(matches!(
            IncomingMessage::from_line(
                r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":1,"message":"bad"}}"#
            ),
            Err(CoreError::Protocol(_))
        ));
        assert!(matches!(
            IncomingMessage::from_line(r#"{"jsonrpc":"2.0","id":1}"#),
            Err(CoreError::Protocol(_))
        ));
    }

    #[tokio::test]
    async fn answers_ping_while_waiting() {
        let (client, sent) = client_with(
            vec![
                IncomingMessage::Request(JsonRpcRequest::new(
                    RequestId::String("srv-1".to_string()),
                    "ping",
                    None,
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({"tools": []}),
                )),
            ],
            McpClientConfig::default(),
        );
        client.list_tools().await.unwrap();
        let sent = sent.lock().await;
        assert!(matches!(
            &sent[1],
            Sent::Response(resp) if resp.id == RequestId::String("srv-1".to_string()) && resp.result == Some(json!({}))
        ));
    }

    #[tokio::test]
    async fn unknown_server_request_gets_method_not_found() {
        let (client, sent) = client_with(
            vec![
                IncomingMessage::Request(JsonRpcRequest::new(
                    RequestId::String("srv-2".to_string()),
                    "roots/list",
                    None,
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({"tools": []}),
                )),
            ],
            McpClientConfig::default(),
        );
        client.list_tools().await.unwrap();
        let sent = sent.lock().await;
        assert!(matches!(
            &sent[1],
            Sent::Response(resp) if resp.error.as_ref().is_some_and(|err| err.code == METHOD_NOT_FOUND)
        ));
    }

    #[tokio::test]
    async fn request_times_out() {
        let client = McpClient::with_transport(
            Box::new(MockTransport::never_recv()),
            McpClientConfig {
                init_timeout: Duration::from_millis(5),
                call_timeout: Duration::from_millis(5),
            },
        );
        assert!(matches!(
            client.list_tools().await,
            Err(CoreError::Timeout { method }) if method == "tools/list"
        ));
    }

    #[allow(dead_code)]
    fn _tool_type_guard(tool: Tool) -> Tool {
        tool
    }
}
