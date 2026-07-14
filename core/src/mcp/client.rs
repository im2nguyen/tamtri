use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use async_trait::async_trait;

use crate::Result;
use crate::mcp::jsonrpc::{JsonRpcError, METHOD_NOT_FOUND};
use crate::mcp::protocol::{
    CallToolParams, CallToolResult, ClientCapabilities, ElicitationCapability, GetPromptParams,
    GetPromptResult, Implementation, InitializeParams, InitializeResult, ListPromptsParams,
    ListPromptsResult, ListResourcesParams, ListResourcesResult, ListToolsParams, ListToolsResult,
    MCP_PROTOCOL_VERSION, Prompt, ReadResourceParams, ReadResourceResult, Resource,
    RootsCapability, ServerCapabilities, Tool,
};
use crate::mcp::roots::RootsHandler;
use crate::rpc::dispatch::{InboundMessage, RpcConnection, RpcHandle};
use crate::rpc::transport::Transport;
use crate::rpc::transport::http::HttpTransport;
use crate::rpc::transport::stdio::StdioTransport;

#[derive(Debug, Clone, PartialEq)]
pub enum McpClientEvent {
    Progress { params: Value },
    Log { params: Value },
    Cancelled { params: Value },
    TaskStatus { params: Value },
}

#[async_trait]
pub trait ElicitationHandler: Send + Sync {
    async fn handle_create(&self, params: Value) -> std::result::Result<Value, JsonRpcError>;
}

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
    handle: RpcHandle,
    inbound_driver: JoinHandle<()>,
    config: McpClientConfig,
    server_info: Option<Implementation>,
    server_capabilities: Option<ServerCapabilities>,
}

impl McpClient {
    pub async fn connect_stdio(
        command: &str,
        args: &[String],
        env: &[(String, String)],
        config: McpClientConfig,
    ) -> Result<Self> {
        let transport = StdioTransport::spawn(command, args, env).await?;
        let mut client = Self::with_transport(Box::new(transport), config, None, None, None);
        client.initialize().await?;
        client.send_initialized_notification().await?;
        Ok(client)
    }

    pub async fn connect_stdio_with_events(
        command: &str,
        args: &[String],
        env: &[(String, String)],
        config: McpClientConfig,
        events: mpsc::UnboundedSender<McpClientEvent>,
        elicitation: Option<Arc<dyn ElicitationHandler>>,
        roots: Option<Arc<dyn RootsHandler>>,
    ) -> Result<Self> {
        let transport = StdioTransport::spawn(command, args, env).await?;
        let mut client = Self::with_transport(
            Box::new(transport),
            config,
            Some(events),
            elicitation,
            roots,
        );
        client.initialize().await?;
        client.send_initialized_notification().await?;
        Ok(client)
    }

    pub async fn connect_http(
        endpoint: &str,
        headers: &[(String, String)],
        config: McpClientConfig,
    ) -> Result<Self> {
        let transport = HttpTransport::new(endpoint, headers)?;
        let mut client = Self::with_transport(Box::new(transport), config, None, None, None);
        client.initialize().await?;
        client.send_initialized_notification().await?;
        Ok(client)
    }

    pub async fn connect_http_with_events(
        endpoint: &str,
        headers: &[(String, String)],
        config: McpClientConfig,
        events: mpsc::UnboundedSender<McpClientEvent>,
        elicitation: Option<Arc<dyn ElicitationHandler>>,
        roots: Option<Arc<dyn RootsHandler>>,
    ) -> Result<Self> {
        let transport = HttpTransport::new(endpoint, headers)?;
        let mut client = Self::with_transport(
            Box::new(transport),
            config,
            Some(events),
            elicitation,
            roots,
        );
        client.initialize().await?;
        client.send_initialized_notification().await?;
        Ok(client)
    }

    /// Hermetic integration-test constructor. Production code should use `connect_stdio` or
    /// `connect_http`.
    pub fn connect_test(
        transport: Box<dyn Transport>,
        config: McpClientConfig,
        events: Option<mpsc::UnboundedSender<McpClientEvent>>,
    ) -> Self {
        Self::with_transport(transport, config, events, None, None)
    }

    fn with_transport(
        transport: Box<dyn Transport>,
        config: McpClientConfig,
        events: Option<mpsc::UnboundedSender<McpClientEvent>>,
        elicitation: Option<Arc<dyn ElicitationHandler>>,
        roots: Option<Arc<dyn RootsHandler>>,
    ) -> Self {
        let (handle, inbound) = RpcConnection::start(transport);
        let inbound_driver = tokio::spawn(run_inbound_driver(
            handle.clone(),
            inbound,
            events,
            elicitation,
            roots,
        ));
        Self {
            handle,
            inbound_driver,
            config,
            server_info: None,
            server_capabilities: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities {
                elicitation: Some(ElicitationCapability {
                    form: Some(serde_json::json!({})),
                    url: Some(serde_json::json!({})),
                }),
                roots: Some(RootsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
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
        meta: Option<serde_json::Value>,
    ) -> Result<CallToolResult> {
        self.call_tool_with_task(name, arguments, None, meta).await
    }

    pub async fn call_tool_with_task(
        &self,
        name: &str,
        arguments: serde_json::Value,
        task: Option<serde_json::Value>,
        meta: Option<serde_json::Value>,
    ) -> Result<CallToolResult> {
        let raw = self.call_tool_raw(name, arguments, task, meta).await?;
        if raw.get("task").is_some() {
            return Ok(CallToolResult {
                content: vec![json!({
                    "type": "text",
                    "text": "Task started"
                })],
                is_error: Some(false),
                structured_content: Some(raw),
            });
        }
        Ok(serde_json::from_value(raw)?)
    }

    pub async fn call_tool_raw(
        &self,
        name: &str,
        arguments: serde_json::Value,
        task: Option<serde_json::Value>,
        meta: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let params = CallToolParams {
            name: name.to_string(),
            arguments,
            task,
            meta,
        };
        self.request_value(
            "tools/call",
            Some(serde_json::to_value(params)?),
            self.config.call_timeout,
        )
        .await
    }

    pub async fn get_task(&self, task_id: &str) -> Result<serde_json::Value> {
        self.request_value(
            "tasks/get",
            Some(json!({ "taskId": task_id })),
            self.config.call_timeout,
        )
        .await
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<serde_json::Value> {
        self.request_value(
            "tasks/cancel",
            Some(json!({ "taskId": task_id })),
            self.config.call_timeout,
        )
        .await
    }

    pub async fn get_task_result(&self, task_id: &str) -> Result<serde_json::Value> {
        self.request_value(
            "tasks/result",
            Some(json!({ "taskId": task_id })),
            self.config.call_timeout,
        )
        .await
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = Vec::new();
        let mut cursor = None;
        loop {
            let params = ListResourcesParams {
                cursor: cursor.clone(),
            };
            let page: ListResourcesResult = self
                .request(
                    "resources/list",
                    Some(serde_json::to_value(params)?),
                    self.config.init_timeout,
                )
                .await?;
            resources.extend(page.resources);
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => return Ok(resources),
            }
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        let params = ReadResourceParams {
            uri: uri.to_string(),
        };
        self.request(
            "resources/read",
            Some(serde_json::to_value(params)?),
            self.config.call_timeout,
        )
        .await
    }

    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        let mut prompts = Vec::new();
        let mut cursor = None;
        loop {
            let params = ListPromptsParams {
                cursor: cursor.clone(),
            };
            let page: ListPromptsResult = self
                .request(
                    "prompts/list",
                    Some(serde_json::to_value(params)?),
                    self.config.init_timeout,
                )
                .await?;
            prompts.extend(page.prompts);
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => return Ok(prompts),
            }
        }
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<GetPromptResult> {
        let params = GetPromptParams {
            name: name.to_string(),
            arguments,
        };
        self.request(
            "prompts/get",
            Some(serde_json::to_value(params)?),
            self.config.call_timeout,
        )
        .await
    }

    pub async fn close(self) -> Result<()> {
        let result = self.handle.close().await;
        self.inbound_driver.abort();
        result
    }

    pub fn server_info(&self) -> Option<&Implementation> {
        self.server_info.as_ref()
    }

    pub fn server_capabilities(&self) -> Option<&ServerCapabilities> {
        self.server_capabilities.as_ref()
    }

    async fn send_initialized_notification(&self) -> Result<()> {
        self.handle.notify("notifications/initialized", None).await
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
        self.handle.request(method, params, timeout).await
    }
}

async fn run_inbound_driver(
    handle: RpcHandle,
    mut inbound: crate::rpc::dispatch::InboundRequests,
    events: Option<mpsc::UnboundedSender<McpClientEvent>>,
    elicitation: Option<Arc<dyn ElicitationHandler>>,
    roots: Option<Arc<dyn RootsHandler>>,
) {
    while let Some(message) = inbound.recv().await {
        match message {
            InboundMessage::Request(req) if req.method == "ping" => {
                let _ = handle.respond(req.id, Ok(json!({}))).await;
            }
            InboundMessage::Request(req) if req.method == "elicitation/create" => {
                let result = if let Some(handler) = &elicitation {
                    handler
                        .handle_create(req.params.unwrap_or_else(|| json!({})))
                        .await
                } else {
                    Err(JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: "elicitation is not supported".to_string(),
                        data: None,
                    })
                };
                let _ = handle.respond(req.id, result).await;
            }
            InboundMessage::Request(req) if req.method == "roots/list" => {
                let result = if let Some(handler) = &roots {
                    handler.handle_list().await
                } else {
                    Err(JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: "roots are not supported".to_string(),
                        data: None,
                    })
                };
                let _ = handle.respond(req.id, result).await;
            }
            InboundMessage::Request(req) if req.method == "sampling/create" => {
                let _ = handle
                    .respond(
                        req.id,
                        Err(JsonRpcError {
                            code: METHOD_NOT_FOUND,
                            message: "sampling is not supported".to_string(),
                            data: None,
                        }),
                    )
                    .await;
            }
            InboundMessage::Request(req) => {
                tracing::warn!("unsupported MCP server request {}", req.method);
                let _ = handle
                    .respond(
                        req.id,
                        Err(JsonRpcError {
                            code: METHOD_NOT_FOUND,
                            message: "method not found".to_string(),
                            data: None,
                        }),
                    )
                    .await;
            }
            InboundMessage::Notification(note) => {
                match note.method.as_str() {
                    "notifications/progress" | "$/progress" => {
                        if let Some(tx) = &events {
                            let _ = tx.send(McpClientEvent::Progress {
                                params: note.params.unwrap_or_else(|| json!({})),
                            });
                        }
                    }
                    "notifications/message" | "notifications/logging/message" => {
                        if let Some(tx) = &events {
                            let _ = tx.send(McpClientEvent::Log {
                                params: note.params.unwrap_or_else(|| json!({})),
                            });
                        }
                    }
                    "notifications/cancelled" | "$/cancelRequest" => {
                        if let Some(tx) = &events {
                            let _ = tx.send(McpClientEvent::Cancelled {
                                params: note.params.unwrap_or_else(|| json!({})),
                            });
                        }
                    }
                    "notifications/tasks/status" => {
                        if let Some(tx) = &events {
                            let _ = tx.send(McpClientEvent::TaskStatus {
                                params: note.params.unwrap_or_else(|| json!({})),
                            });
                        }
                    }
                    _ => {}
                }
                tracing::debug!("received MCP notification {}", note.method);
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
    use crate::CoreError;
    use crate::conversation::ContentBlock;
    use crate::mcp::bridge::{tool_call_block, tool_result_block};
    use crate::mcp::jsonrpc::{
        IncomingMessage, JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
        RequestId,
    };
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
        requests_sent: usize,
        responses_delivered: usize,
    }

    impl MockTransport {
        fn new(incoming: Vec<IncomingMessage>) -> (Self, Arc<TokioMutex<Vec<Sent>>>) {
            let sent = Arc::new(TokioMutex::new(Vec::new()));
            (
                Self {
                    incoming: incoming.into(),
                    sent: Arc::clone(&sent),
                    never_recv: false,
                    requests_sent: 0,
                    responses_delivered: 0,
                },
                sent,
            )
        }

        fn never_recv() -> Self {
            Self {
                incoming: VecDeque::new(),
                sent: Arc::new(TokioMutex::new(Vec::new())),
                never_recv: true,
                requests_sent: 0,
                responses_delivered: 0,
            }
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()> {
            self.requests_sent += 1;
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
            if matches!(self.incoming.front(), Some(IncomingMessage::Response(_)))
                && self.responses_delivered >= self.requests_sent
            {
                tokio::time::sleep(Duration::from_secs(60)).await;
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
        (
            McpClient::with_transport(Box::new(transport), config, None, None, None),
            sent,
        )
    }

    async fn wait_for_sent_len(sent: &Arc<TokioMutex<Vec<Sent>>>, len: usize) -> Vec<Sent> {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let snapshot = sent.lock().await.clone();
                if snapshot.len() >= len {
                    return snapshot;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn initialize_handshake() {
        let (mut client, sent) = client_with(vec![init_response()], McpClientConfig::default());
        let result = client.initialize().await.unwrap();
        assert_eq!(result.server_info.name, "mock");
        assert_eq!(result.server_info.version, "1.0.0");
        assert_eq!(client.server_info().unwrap().name, "mock");
        assert_eq!(client.server_info().unwrap().version, "1.0.0");
        assert_eq!(
            client.server_capabilities(),
            Some(&ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false)
                }),
                ..Default::default()
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
        let mut client = McpClient::with_transport(
            Box::new(transport),
            McpClientConfig::default(),
            None,
            None,
            None,
        );
        client.initialize().await.unwrap();
        client.send_initialized_notification().await.unwrap();
        let sent = wait_for_sent_len(&sent, 2).await;
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
            .call_tool("echo", json!({"text": "hi"}), None)
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
        let result = client.call_tool("echo", json!({}), None).await.unwrap();
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn resources_list_and_read() {
        let (client, sent) = client_with(
            vec![
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({"resources": [{"uri": "mock://one", "name": "One", "mimeType": "text/plain"}]}),
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(2),
                    json!({"contents": [{"uri": "mock://one", "mimeType": "text/plain", "text": "hello"}]}),
                )),
            ],
            McpClientConfig::default(),
        );
        let resources = client.list_resources().await.unwrap();
        assert_eq!(resources[0].uri, "mock://one");
        let contents = client.read_resource("mock://one").await.unwrap();
        assert_eq!(contents.contents[0]["text"], "hello");
        let sent = wait_for_sent_len(&sent, 2).await;
        assert!(matches!(
            &sent[1],
            Sent::Request(req) if req.method == "resources/read" && req.params.as_ref().unwrap()["uri"] == "mock://one"
        ));
    }

    #[tokio::test]
    async fn prompts_list_and_get() {
        let (client, sent) = client_with(
            vec![
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({"prompts": [{"name": "summarize", "description": "Summarize"}]}),
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(2),
                    json!({"description": "Summarize", "messages": [{"role": "user", "content": {"type": "text", "text": "go"}}]}),
                )),
            ],
            McpClientConfig::default(),
        );
        let prompts = client.list_prompts().await.unwrap();
        assert_eq!(prompts[0].name, "summarize");
        let prompt = client
            .get_prompt("summarize", json!({"topic": "tamtri"}))
            .await
            .unwrap();
        assert_eq!(prompt.messages[0]["role"], "user");
        let sent = wait_for_sent_len(&sent, 2).await;
        assert!(matches!(
            &sent[1],
            Sent::Request(req) if req.method == "prompts/get" && req.params.as_ref().unwrap()["name"] == "summarize"
        ));
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
    async fn emits_progress_and_log_notifications() {
        let (events_tx, mut events_rx) = mpsc::unbounded_channel();
        let (transport, _sent) = MockTransport::new(vec![
            IncomingMessage::Notification(JsonRpcNotification::new(
                "notifications/progress",
                Some(json!({"progress": 0.5, "message": "halfway"})),
            )),
            IncomingMessage::Notification(JsonRpcNotification::new(
                "notifications/message",
                Some(json!({"level": "info", "data": "working"})),
            )),
            IncomingMessage::Response(JsonRpcResponse::success(
                RequestId::Number(1),
                json!({"tools": []}),
            )),
        ]);
        let client = McpClient::with_transport(
            Box::new(transport),
            McpClientConfig::default(),
            Some(events_tx),
            None,
            None,
        );
        assert!(client.list_tools().await.unwrap().is_empty());

        assert!(matches!(
            events_rx.recv().await,
            Some(McpClientEvent::Progress { params }) if params["message"] == "halfway"
        ));
        assert!(matches!(
            events_rx.recv().await,
            Some(McpClientEvent::Log { params }) if params["data"] == "working"
        ));
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
    fn tool_call_block_maps_to_content_block() {
        let args = json!({"message": "hello"});
        let block = tool_call_block("call-1", "echo", &args);
        let ContentBlock::ToolCall { id, name, input } = block else {
            panic!("expected tool call");
        };
        assert_eq!(id, "call-1");
        assert_eq!(name, "echo");
        assert_eq!(input, args);
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
        let sent = wait_for_sent_len(&sent, 2).await;
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
        let sent = wait_for_sent_len(&sent, 2).await;
        assert!(matches!(
            &sent[1],
            Sent::Response(resp) if resp.error.as_ref().is_some_and(|err| err.code == METHOD_NOT_FOUND)
        ));
    }

    #[tokio::test]
    async fn request_times_out() {
        tokio::time::pause();
        let client = McpClient::with_transport(
            Box::new(MockTransport::never_recv()),
            McpClientConfig {
                init_timeout: Duration::from_millis(5),
                call_timeout: Duration::from_millis(5),
            },
            None,
            None,
            None,
        );
        let first = client.list_tools();
        tokio::time::advance(Duration::from_millis(10)).await;
        assert!(matches!(
            first.await,
            Err(CoreError::Timeout { method }) if method == "tools/list"
        ));
        assert!(matches!(
            client.list_tools().await,
            Err(CoreError::TransportClosed)
        ));
    }

    #[allow(dead_code)]
    fn _tool_type_guard(tool: Tool) -> Tool {
        tool
    }
}
