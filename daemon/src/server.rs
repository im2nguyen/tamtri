//! The WebSocket transport for the daemon protocol.
//!
//! One connection per client. Client-to-daemon frames are JSON-RPC requests
//! (text frames); the daemon replies with JSON-RPC responses and pushes
//! `event` notifications from the broadcast stream. Binary payloads (attachment
//! bytes) ride as base64 inside the JSON result, so no binary framing is needed
//! yet.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::broadcast::error::RecvError;

use tamtri_core::daemon::Daemon;
use tamtri_core::protocol::method;
use tamtri_core::rpc::jsonrpc::{
    IncomingMessage, JsonRpcNotification, JsonRpcResponse,
};

#[derive(Clone)]
struct AppState {
    daemon: Arc<Daemon>,
    token: String,
}

#[derive(Debug, Deserialize)]
struct ConnectQuery {
    #[serde(default)]
    token: String,
}

/// Serve the daemon protocol over an already-bound listener until a shutdown
/// signal arrives.
pub async fn serve(
    listener: tokio::net::TcpListener,
    daemon: Arc<Daemon>,
    token: String,
) -> std::io::Result<()> {
    let state = AppState { daemon, token };
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Convenience for callers that want us to bind for them; returns the bound
/// address so the port can be recorded.
pub async fn bind(addr: SocketAddr) -> std::io::Result<tokio::net::TcpListener> {
    tokio::net::TcpListener::bind(addr).await
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<ConnectQuery>,
) -> Response {
    // Constant-time-ish check is overkill for a localhost token, but reject
    // empty/mismatched tokens outright.
    if query.token.is_empty() || query.token != state.token {
        return (StatusCode::UNAUTHORIZED, "invalid or missing token").into_response();
    }
    let daemon = Arc::clone(&state.daemon);
    ws.on_upgrade(move |socket| handle_socket(socket, daemon))
}

async fn handle_socket(socket: WebSocket, daemon: Arc<Daemon>) {
    let (mut sender, mut receiver) = socket.split();
    let mut events = daemon.subscribe();

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(reply) = handle_request(&daemon, text.as_str()).await
                            && sender.send(Message::Text(reply.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    // Ping/pong handled by axum; ignore other frames.
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
            event = events.recv() => {
                match event {
                    Ok(notification) => {
                        let params = serde_json::to_value(&notification).unwrap_or(Value::Null);
                        let note = JsonRpcNotification::new(method::EVENT, Some(params));
                        let line = match serde_json::to_string(&note) {
                            Ok(line) => line,
                            Err(_) => continue,
                        };
                        if sender.send(Message::Text(line.into())).await.is_err() {
                            break;
                        }
                    }
                    // Slow client fell behind. Keep the connection; the client
                    // recovers via an authoritative transcript fetch.
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}

/// Parse one inbound text frame, dispatch it, and produce the response line.
/// Non-request frames (notifications, responses, malformed) are ignored because
/// the client is not expected to send them in v1.
async fn handle_request(daemon: &Daemon, text: &str) -> Option<String> {
    let request = match IncomingMessage::from_line(text) {
        Ok(IncomingMessage::Request(request)) => request,
        _ => return None,
    };

    let id = request.id.clone();
    let result = if request.method == method::HELLO {
        serde_json::to_value(daemon.server_info())
            .map_err(|err| tamtri_core::rpc::jsonrpc::JsonRpcError {
                code: -32603,
                message: format!("serialize server_info: {err}"),
                data: None,
            })
    } else {
        daemon.dispatch(&request.method, request.params).await
    };

    let response = match result {
        Ok(value) => JsonRpcResponse::success(id, value),
        Err(error) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        },
    };
    serde_json::to_string(&response).ok()
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
