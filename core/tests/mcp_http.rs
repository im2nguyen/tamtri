use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Value, json};
use tamtri_core::mcp::{McpClient, McpClientConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

#[tokio::test]
async fn streamable_http_json_response() {
    let (url, _seen_rx) = spawn_http_fixture(false, false).await;
    let client = McpClient::connect_http(&url, &[], McpClientConfig::default())
        .await
        .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools[0].name, "http_echo");

    let result = client
        .call_tool("http_echo", json!({"message": "hello"}), None)
        .await
        .unwrap();
    assert_eq!(result.content[0]["text"], "hello from json");
}

#[tokio::test]
async fn streamable_http_preserves_session_header() {
    let (url, mut seen_rx) = spawn_http_fixture(false, false).await;
    let client = McpClient::connect_http(&url, &[], McpClientConfig::default())
        .await
        .unwrap();

    let _ = client.list_tools().await.unwrap();

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
async fn streamable_http_error_status() {
    let (url, _seen_rx) = spawn_http_fixture(false, true).await;
    let result = McpClient::connect_http(&url, &[], McpClientConfig::default()).await;
    let Err(err) = result else {
        panic!("expected connect to fail with HTTP 500");
    };
    assert!(
        err.to_string().contains("500"),
        "expected HTTP error status in connect error, got: {err}"
    );
}

#[tokio::test]
async fn streamable_http_sse_response() {
    let (url, _seen_rx) = spawn_http_fixture(true, false).await;
    let client = McpClient::connect_http(&url, &[], McpClientConfig::default())
        .await
        .unwrap();

    let result = client
        .call_tool("http_echo", json!({"message": "hello"}), None)
        .await
        .unwrap();
    assert_eq!(result.content[0]["text"], "hello from sse");
}

#[tokio::test]
async fn remote_http_server_uses_oauth_header_without_logging_value() {
    use tamtri_core::config::{GatewayConfig, GatewayScope, GatewayServerConfig, GatewayTransport, OAuthConfig};
    use tamtri_core::mcp::gateway::{GatewayEvent, McpGateway, MemoryCredentials};
    use tamtri_core::mcp::oauth::{StoredOAuthBundle, serialize_stored_oauth};

    let expected_token = "secret-access-token-123";
    let (url, mut seen_rx) = spawn_http_fixture(false, false).await;
    let credentials = Arc::new(MemoryCredentials::default());
    let token_ref = "keychain://remote-oauth".to_string();
    let bundle = StoredOAuthBundle {
        access_token: expected_token.to_string(),
        refresh_token: None,
        expires_at: None,
        reauth_required: false,
    };
    credentials
        .set(token_ref.clone(), serialize_stored_oauth(&bundle).unwrap())
        .unwrap();

    let server = GatewayServerConfig {
        id: "remote".to_string(),
        display_name: "Remote".to_string(),
        enabled: true,
        scope: GatewayScope::User,
        transport: GatewayTransport::StreamableHttp {
            endpoint: url.clone(),
            headers: Vec::new(),
        },
        timeout_secs: Some(30),
        credentials: Vec::new(),
        oauth: Some(OAuthConfig {
            issuer: None,
            authorization_endpoint: None,
            token_endpoint: Some("https://example.com/token".to_string()),
            client_id: "tamtri-test".to_string(),
            scopes: vec!["mcp".to_string()],
            token_ref: token_ref.clone(),
        }),
    };

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 30,
            servers: vec![server],
        },
        credentials,
        Some(event_tx),
    )
    .unwrap();

    let tools = gateway.list_tools().await.unwrap();
    let exposed = tools
        .iter()
        .find(|tool| tool.original_name == "http_echo")
        .map(|tool| tool.exposed_name.clone())
        .expect("http_echo tool");

    let _ = gateway
        .call_tool(&exposed, json!({"message": "hello"}))
        .await
        .unwrap();

    // Assert HTTP server saw the Authorization header, and it matches.
    let headers = tokio::time::timeout(std::time::Duration::from_secs(2), seen_rx.recv())
        .await
        .unwrap()
        .expect("seen headers");
    let expected_header = format!("Bearer {expected_token}");
    assert_eq!(
        headers.get("authorization").map(|value| value.as_str()),
        Some(expected_header.as_str())
    );

    // Assert gateway emitted injection events but never logged the token value.
    let mut saw_injected = false;
    let mut serialized_events = String::new();
    while let Ok(Some(event)) =
        tokio::time::timeout(std::time::Duration::from_millis(200), event_rx.recv()).await
    {
        if matches!(event, GatewayEvent::CredentialInjected { .. }) {
            saw_injected = true;
        }
        serialized_events.push_str(&serde_json::to_string(&event).unwrap());
    }
    assert!(saw_injected, "expected credential injection event");
    assert!(
        !serialized_events.contains(expected_token),
        "token value leaked into gateway event serialization"
    );
}

async fn spawn_http_fixture(
    sse_tools_call: bool,
    error_status: bool,
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
                let _ = handle_connection(socket, seen_tx, sse_tools_call, error_status).await;
            });
        }
    });
    (format!("http://{addr}/mcp"), seen_rx)
}

async fn handle_connection(
    mut socket: TcpStream,
    seen_tx: Arc<mpsc::Sender<HashMap<String, String>>>,
    sse_tools_call: bool,
    error_status: bool,
) -> std::io::Result<()> {
    let (headers, body) = read_request(&mut socket).await?;
    let _ = seen_tx.send(headers).await;
    let message: Value = serde_json::from_slice(&body).unwrap();
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" if error_status => {
            write_json(
                &mut socket,
                500,
                &[],
                json!({
                    "jsonrpc": "2.0",
                    "id": message["id"].clone(),
                    "error": {"code": -32000, "message": "server unavailable"}
                }),
            )
            .await
        }
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
        500 => "Internal Server Error",
        _ => "OK",
    }
}
