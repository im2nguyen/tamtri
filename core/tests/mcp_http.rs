use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Value, json};
use tamtri_core::mcp::{McpClient, McpClientConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

#[tokio::test]
async fn streamable_http_json_response_and_session_header() {
    let (url, mut seen_rx) = spawn_http_fixture(false).await;
    let client = McpClient::connect_http(&url, &[], McpClientConfig::default())
        .await
        .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools[0].name, "http_echo");

    let mut saw_session_header = false;
    for _ in 0..4 {
        let Some(headers) = tokio::time::timeout(std::time::Duration::from_secs(2), seen_rx.recv())
            .await
            .unwrap()
        else {
            break;
        };
        if headers
            .get("mcp-session-id")
            .is_some_and(|value| value == "session-123")
        {
            saw_session_header = true;
            break;
        }
    }
    assert!(saw_session_header);
}

#[tokio::test]
async fn streamable_http_sse_response() {
    let (url, _seen_rx) = spawn_http_fixture(true).await;
    let client = McpClient::connect_http(&url, &[], McpClientConfig::default())
        .await
        .unwrap();

    let result = client
        .call_tool("http_echo", json!({"message": "hello"}), None)
        .await
        .unwrap();
    assert_eq!(result.content[0]["text"], "hello from sse");
}

async fn spawn_http_fixture(
    sse_tools_call: bool,
) -> (String, mpsc::Receiver<HashMap<String, String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (seen_tx, seen_rx) = mpsc::channel(16);
    let seen_tx = Arc::new(seen_tx);
    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                break;
            };
            let seen_tx = Arc::clone(&seen_tx);
            tokio::spawn(async move {
                let _ = handle_connection(socket, seen_tx, sse_tools_call).await;
            });
        }
    });
    (format!("http://{addr}/mcp"), seen_rx)
}

async fn handle_connection(
    mut socket: TcpStream,
    seen_tx: Arc<mpsc::Sender<HashMap<String, String>>>,
    sse_tools_call: bool,
) -> std::io::Result<()> {
    let (headers, body) = read_request(&mut socket).await?;
    let _ = seen_tx.send(headers).await;
    let message: Value = serde_json::from_slice(&body).unwrap();
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => {
            write_json(
                &mut socket,
                200,
                &[("Mcp-Session-Id", "session-123")],
                json!({
                    "jsonrpc": "2.0",
                    "id": message["id"].clone(),
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": {"tools": {"listChanged": false}},
                        "serverInfo": {"name": "mock-http", "version": "0.1.0"}
                    }
                }),
            )
            .await
        }
        "notifications/initialized" => write_empty(&mut socket, 202).await,
        "tools/list" => {
            write_json(
                &mut socket,
                200,
                &[],
                json!({
                    "jsonrpc": "2.0",
                    "id": message["id"].clone(),
                    "result": {
                        "tools": [{
                            "name": "http_echo",
                            "description": "Echo over HTTP",
                            "inputSchema": {"type": "object"}
                        }]
                    }
                }),
            )
            .await
        }
        "tools/call" if sse_tools_call => {
            let progress = json!({
                "jsonrpc": "2.0",
                "method": "notifications/progress",
                "params": {"progressToken": "p1", "progress": 1.0, "message": "working"}
            });
            let response = json!({
                "jsonrpc": "2.0",
                "id": message["id"].clone(),
                "result": {
                    "content": [{"type": "text", "text": "hello from sse"}],
                    "isError": false
                }
            });
            let body = format!("data: {progress}\n\ndata: {response}\n\n");
            write_raw(&mut socket, 200, "text/event-stream", &body, &[]).await
        }
        "tools/call" => {
            write_json(
                &mut socket,
                200,
                &[],
                json!({
                    "jsonrpc": "2.0",
                    "id": message["id"].clone(),
                    "result": {
                        "content": [{"type": "text", "text": "hello from json"}],
                        "isError": false
                    }
                }),
            )
            .await
        }
        _ => {
            write_json(
                &mut socket,
                200,
                &[],
                json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id").cloned().unwrap_or(Value::Null),
                    "error": {"code": -32601, "message": "method not found"}
                }),
            )
            .await
        }
    }
}

async fn read_request(
    socket: &mut TcpStream,
) -> std::io::Result<(HashMap<String, String>, Vec<u8>)> {
    let mut buffer = Vec::new();
    loop {
        let mut chunk = [0; 1024];
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(header_end) = find_header_end(&buffer) {
            let header_text = String::from_utf8_lossy(&buffer[..header_end]);
            let headers = parse_headers(&header_text);
            let content_length = headers
                .get("content-length")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let body_start = header_end + 4;
            while buffer.len() < body_start + content_length {
                let read = socket.read(&mut chunk).await?;
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            return Ok((
                headers,
                buffer[body_start..body_start + content_length].to_vec(),
            ));
        }
    }
    Ok((HashMap::new(), Vec::new()))
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_headers(header_text: &str) -> HashMap<String, String> {
    header_text
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect()
}

async fn write_empty(socket: &mut TcpStream, status: u16) -> std::io::Result<()> {
    let status_text = status_text(status);
    socket
        .write_all(
            format!("HTTP/1.1 {status} {status_text}\r\nContent-Length: 0\r\n\r\n").as_bytes(),
        )
        .await
}

async fn write_json(
    socket: &mut TcpStream,
    status: u16,
    headers: &[(&str, &str)],
    value: Value,
) -> std::io::Result<()> {
    write_raw(
        socket,
        status,
        "application/json",
        &value.to_string(),
        headers,
    )
    .await
}

async fn write_raw(
    socket: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
    headers: &[(&str, &str)],
) -> std::io::Result<()> {
    let status_text = status_text(status);
    let mut response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.push_str(body);
    socket.write_all(response.as_bytes()).await
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        _ => "OK",
    }
}
