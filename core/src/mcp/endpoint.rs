use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::conversation::McpServerRef;
use crate::mcp::gateway::{GatewayEvent, McpGateway};
use crate::mcp::server::handle_gateway_request;
use crate::rpc::jsonrpc::{
    IncomingMessage, JsonRpcNotification, JsonRpcResponse, METHOD_NOT_FOUND,
};
use crate::{CoreError, Result};

pub struct GatewayEndpoint {
    mcp_ref: McpServerRef,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl GatewayEndpoint {
    pub fn mcp_ref(&self) -> McpServerRef {
        self.mcp_ref.clone()
    }

    pub fn stdio_mcp_ref(&self, command: String) -> McpServerRef {
        McpServerRef {
            id: "tamtri-gateway".to_string(),
            name: "Tamtri Gateway".to_string(),
            transport: "stdio".to_string(),
            endpoint: format!("{} {}", command, self.mcp_ref.endpoint),
        }
    }

    pub fn http_endpoint(&self) -> &str {
        &self.mcp_ref.endpoint
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for GatewayEndpoint {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

pub async fn start_loopback_gateway(gateway: Arc<McpGateway>) -> Result<GatewayEndpoint> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let endpoint = format!("http://{addr}/mcp");
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, _peer)) => {
                            let gateway = Arc::clone(&gateway);
                            tokio::spawn(async move {
                                let _ = handle_http_connection(stream, gateway).await;
                            });
                        }
                        Err(err) => {
                            tracing::warn!("tamtri gateway HTTP accept failed: {err}");
                            break;
                        }
                    }
                }
            }
        }
    });
    Ok(GatewayEndpoint {
        mcp_ref: McpServerRef {
            id: "tamtri-gateway".to_string(),
            name: "Tamtri Gateway".to_string(),
            transport: "http".to_string(),
            endpoint,
        },
        shutdown_tx: Some(shutdown_tx),
        task: Some(task),
    })
}

async fn handle_http_connection(mut stream: TcpStream, gateway: Arc<McpGateway>) -> Result<()> {
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0_u8; 1024];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Err(CoreError::TransportClosed);
        }
        buffer.extend_from_slice(&chunk[..n]);
        if let Some(index) = find_header_end(&buffer) {
            break index;
        }
        if buffer.len() > 64 * 1024 {
            return Err(CoreError::Protocol(
                "gateway HTTP request headers too large".to_string(),
            ));
        }
    };

    let header_text = std::str::from_utf8(&buffer[..header_end])
        .map_err(|err| CoreError::Protocol(format!("invalid HTTP headers: {err}")))?;
    let Some(request_line) = header_text.lines().next() else {
        return write_http_response(&mut stream, 400, "text/plain", b"bad request").await;
    };
    if request_line.starts_with("GET /mcp ") {
        return write_sse_stream(&mut stream, gateway).await;
    }
    if !request_line.starts_with("POST /mcp ") {
        return write_http_response(&mut stream, 404, "text/plain", b"not found").await;
    }
    let content_length = content_length(header_text)?;
    if content_length > 1024 * 1024 {
        return write_http_response(&mut stream, 413, "text/plain", b"payload too large").await;
    }
    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let mut chunk = [0_u8; 8192];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Err(CoreError::TransportClosed);
        }
        buffer.extend_from_slice(&chunk[..n]);
    }
    let body = std::str::from_utf8(&buffer[body_start..body_start + content_length])
        .map_err(|err| CoreError::Protocol(format!("invalid HTTP body: {err}")))?;

    match IncomingMessage::from_line(body) {
        Ok(IncomingMessage::Request(req)) => {
            let id = req.id;
            let result = handle_gateway_request(&gateway, &req.method, req.params).await;
            let response = match result {
                Ok(value) => JsonRpcResponse::success(id, value),
                Err(error) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(error),
                },
            };
            let bytes = serde_json::to_vec(&response)?;
            write_http_response(&mut stream, 200, "application/json", &bytes).await
        }
        Ok(IncomingMessage::Notification(note)) => {
            if matches!(
                note.method.as_str(),
                "notifications/cancelled" | "notifications/cancelledRequest" | "$/cancelRequest"
            ) {
                gateway.agent_cancelled(note.params.unwrap_or_else(|| serde_json::json!({})));
            }
            tracing::debug!("agent-facing MCP HTTP notification {}", note.method);
            write_http_response(&mut stream, 202, "text/plain", b"").await
        }
        Ok(IncomingMessage::Response(_)) => {
            let response = JsonRpcResponse::error(
                crate::rpc::jsonrpc::RequestId::Number(0),
                METHOD_NOT_FOUND,
                "unexpected response at gateway server",
            );
            let bytes = serde_json::to_vec(&response)?;
            write_http_response(&mut stream, 400, "application/json", &bytes).await
        }
        Err(err) => {
            let error = serde_json::json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": {"code": -32700, "message": err.to_string()}
            });
            let bytes = serde_json::to_vec(&error)?;
            write_http_response(&mut stream, 400, "application/json", &bytes).await
        }
    }
}

async fn write_sse_stream(stream: &mut TcpStream, gateway: Arc<McpGateway>) -> Result<()> {
    let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(b": tamtri gateway stream\n\n").await?;
    stream.flush().await?;

    let mut events = gateway.subscribe();
    loop {
        let event = events
            .recv()
            .await
            .map_err(|_| CoreError::TransportClosed)?;
        let Some(notification) = gateway_event_notification(event) else {
            continue;
        };
        let bytes = serde_json::to_vec(&notification)?;
        stream.write_all(b"event: message\n").await?;
        stream.write_all(b"data: ").await?;
        stream.write_all(&bytes).await?;
        stream.write_all(b"\n\n").await?;
        stream.flush().await?;
    }
}

fn gateway_event_notification(event: GatewayEvent) -> Option<JsonRpcNotification> {
    match event {
        GatewayEvent::Progress { params, .. } => Some(JsonRpcNotification::new(
            "notifications/progress",
            Some(params),
        )),
        GatewayEvent::Log { params, .. } => Some(JsonRpcNotification::new(
            "notifications/message",
            Some(params),
        )),
        GatewayEvent::Cancellation { params, .. } => Some(JsonRpcNotification::new(
            "notifications/cancelled",
            Some(params),
        )),
        _ => None,
    }
}

async fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    };
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.shutdown().await?;
    Ok(())
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(headers: &str) -> Result<usize> {
    for line in headers.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value
                .trim()
                .parse()
                .map_err(|err| CoreError::Protocol(format!("invalid content-length: {err}")));
        }
    }
    Err(CoreError::Protocol(
        "gateway HTTP request missing content-length".to_string(),
    ))
}
