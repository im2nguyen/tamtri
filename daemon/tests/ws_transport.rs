//! End-to-end transport tests: bind the real WebSocket server, connect with a
//! tungstenite client, and exercise the handshake, a dispatch round-trip, and
//! token auth rejection.

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tamtri_core::daemon::Daemon;
use tamtri_daemon::server;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

struct Harness {
    addr: SocketAddr,
    token: String,
    daemon: Arc<Daemon>,
    server: JoinHandle<()>,
    _dir: TempDir,
}

async fn start() -> Harness {
    let dir = TempDir::new().expect("temp vault dir");
    let vault = dir.path().join("vault");
    let daemon = Arc::new(Daemon::new(vault.to_string_lossy().to_string()).expect("build daemon"));
    let token = "test-token".to_string();

    let listener = server::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");

    let server = tokio::spawn({
        let daemon = Arc::clone(&daemon);
        let token = token.clone();
        async move {
            let _ = server::serve(listener, daemon, token).await;
        }
    });

    Harness {
        addr,
        token,
        daemon,
        server,
        _dir: dir,
    }
}

impl Harness {
    async fn shutdown(self) {
        self.server.abort();
        // The core owns its own runtime; drop it off the async executor.
        tokio::task::spawn_blocking(move || drop(self.daemon))
            .await
            .expect("drop daemon off the async executor");
    }

    fn url(&self) -> String {
        format!("ws://{}/ws?token={}", self.addr, self.token)
    }
}

async fn send_request(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    id: i64,
    method: &str,
    params: Value,
) -> Value {
    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .expect("send request");

    // Skip any event notifications; return the first response matching `id`.
    loop {
        let message = socket.next().await.expect("some frame").expect("frame ok");
        if let Message::Text(text) = message {
            let value: Value = serde_json::from_str(text.as_str()).expect("valid json");
            if value.get("id").and_then(Value::as_i64) == Some(id) {
                return value;
            }
        }
    }
}

#[tokio::test]
async fn hello_returns_server_info() {
    let harness = start().await;
    let (mut socket, _) = connect_async(harness.url()).await.expect("connect");

    let response = send_request(
        &mut socket,
        1,
        "hello",
        json!({
            "client_id": "test",
            "client_type": "cli",
            "protocol_version": "1.0",
        }),
    )
    .await;

    let result = &response["result"];
    assert!(result["server_id"].as_str().is_some());
    assert_eq!(result["protocol_version"], json!("1.0"));
    assert!(result.get("features").is_some());

    socket.close(None).await.ok();
    harness.shutdown().await;
}

#[tokio::test]
async fn create_and_list_over_the_wire() {
    let harness = start().await;
    let (mut socket, _) = connect_async(harness.url()).await.expect("connect");

    let created = send_request(
        &mut socket,
        1,
        "conversation.create",
        json!({ "title": "Wire test", "harness_id": "mock-acp", "model_id": "mock" }),
    )
    .await;
    let id = created["result"]["id"]
        .as_str()
        .expect("created id")
        .to_string();

    let listed = send_request(&mut socket, 2, "conversation.list", json!(null)).await;
    let ids: Vec<&str> = listed["result"]
        .as_array()
        .expect("list array")
        .iter()
        .filter_map(|row| row["id"].as_str())
        .collect();
    assert!(ids.contains(&id.as_str()));

    socket.close(None).await.ok();
    harness.shutdown().await;
}

#[tokio::test]
async fn unknown_method_returns_error_over_the_wire() {
    let harness = start().await;
    let (mut socket, _) = connect_async(harness.url()).await.expect("connect");

    let response = send_request(&mut socket, 1, "does.not.exist", json!(null)).await;
    assert_eq!(response["error"]["code"], json!(-32601));

    socket.close(None).await.ok();
    harness.shutdown().await;
}

#[tokio::test]
async fn bad_token_is_rejected() {
    let harness = start().await;
    let bad_url = format!("ws://{}/ws?token=wrong", harness.addr);
    let result = connect_async(bad_url).await;
    assert!(result.is_err(), "connection with a bad token must be rejected");

    harness.shutdown().await;
}
