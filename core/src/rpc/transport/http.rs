use std::sync::Arc;

use futures_util::StreamExt;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use tokio::sync::{Mutex, mpsc};

use crate::rpc::jsonrpc::{IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::rpc::transport::Transport;
use crate::{CoreError, Result};

const MCP_SESSION_HEADER: &str = "mcp-session-id";

pub struct HttpTransport {
    endpoint: reqwest::Url,
    client: reqwest::Client,
    headers: HeaderMap,
    session_id: Arc<Mutex<Option<HeaderValue>>>,
    tx: mpsc::Sender<Result<IncomingMessage>>,
    rx: mpsc::Receiver<Result<IncomingMessage>>,
}

impl HttpTransport {
    pub fn new(endpoint: &str, headers: &[(String, String)]) -> Result<Self> {
        let endpoint = reqwest::Url::parse(endpoint)
            .map_err(|err| CoreError::Protocol(format!("invalid HTTP endpoint: {err}")))?;
        let mut header_map = HeaderMap::new();
        for (name, value) in headers {
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|err| CoreError::Protocol(format!("invalid header name: {err}")))?;
            let value = HeaderValue::from_str(value)
                .map_err(|err| CoreError::Protocol(format!("invalid header value: {err}")))?;
            header_map.insert(name, value);
        }
        let (tx, rx) = mpsc::channel(128);
        Ok(Self {
            endpoint,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|err| {
                    CoreError::Protocol(format!("failed to build HTTP client: {err}"))
                })?,
            headers: header_map,
            session_id: Arc::new(Mutex::new(None)),
            tx,
            rx,
        })
    }

    fn spawn_send<T>(&self, message: T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let message = serde_json::to_value(message)?;
        let endpoint = self.endpoint.clone();
        let client = self.client.clone();
        let headers = self.headers.clone();
        let session_id = Arc::clone(&self.session_id);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            match send_http_message(client, endpoint, headers, session_id, message).await {
                Ok(messages) => {
                    for message in messages {
                        let _ = tx.send(Ok(message)).await;
                    }
                }
                Err(err) => {
                    let _ = tx.send(Err(err)).await;
                }
            }
        });
        Ok(())
    }
}

#[async_trait::async_trait]
impl Transport for HttpTransport {
    async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()> {
        self.spawn_send(req.clone())
    }

    async fn send_notification(&mut self, note: &JsonRpcNotification) -> Result<()> {
        self.spawn_send(note.clone())
    }

    async fn send_response(&mut self, resp: &JsonRpcResponse) -> Result<()> {
        self.spawn_send(resp.clone())
    }

    async fn recv(&mut self) -> Result<IncomingMessage> {
        self.rx.recv().await.ok_or(CoreError::TransportClosed)?
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

async fn send_http_message(
    client: reqwest::Client,
    endpoint: reqwest::Url,
    mut headers: HeaderMap,
    session_id: Arc<Mutex<Option<HeaderValue>>>,
    message: serde_json::Value,
) -> Result<Vec<IncomingMessage>> {
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    if let Some(value) = session_id.lock().await.clone() {
        headers.insert(HeaderName::from_static(MCP_SESSION_HEADER), value);
    }

    let response = client
        .post(endpoint)
        .headers(headers)
        .json(&message)
        .send()
        .await
        .map_err(|err| CoreError::Protocol(format!("HTTP MCP request failed: {err}")))?;

    if let Some(value) = response.headers().get(MCP_SESSION_HEADER).cloned() {
        *session_id.lock().await = Some(value);
    }

    let status = response.status();
    if !status.is_success() {
        return Err(CoreError::Protocol(format!(
            "HTTP MCP server returned status {}",
            status.as_u16()
        )));
    }
    if status == StatusCode::ACCEPTED || status == StatusCode::NO_CONTENT {
        return Ok(Vec::new());
    }

    let is_sse = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/event-stream"));

    if is_sse {
        parse_sse_response(response).await
    } else {
        let value = response
            .json::<serde_json::Value>()
            .await
            .map_err(|err| CoreError::Protocol(format!("invalid HTTP MCP JSON: {err}")))?;
        messages_from_json_value(value)
    }
}

async fn parse_sse_response(response: reqwest::Response) -> Result<Vec<IncomingMessage>> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut messages = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|err| CoreError::Protocol(format!("HTTP MCP stream failed: {err}")))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(frame_end) = buffer.find("\n\n") {
            let frame = buffer[..frame_end].to_string();
            buffer.replace_range(..frame_end + 2, "");
            if let Some(data) = sse_data(&frame) {
                messages.push(IncomingMessage::from_line(&data)?);
            }
        }
    }
    if let Some(data) = sse_data(&buffer) {
        messages.push(IncomingMessage::from_line(&data)?);
    }
    Ok(messages)
}

fn sse_data(frame: &str) -> Option<String> {
    let data = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() { None } else { Some(data) }
}

fn messages_from_json_value(value: serde_json::Value) -> Result<Vec<IncomingMessage>> {
    match value {
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|item| IncomingMessage::from_line(&item.to_string()))
            .collect(),
        other => Ok(vec![IncomingMessage::from_line(&other.to_string())?]),
    }
}
