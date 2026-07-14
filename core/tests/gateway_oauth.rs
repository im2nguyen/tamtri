use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use chrono::Utc;
use serde_json::json;
use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::config::{
    GatewayConfig, GatewayScope, GatewayServerConfig, GatewayTransport, OAuthConfig,
};
use tamtri_core::mcp::gateway::MemoryCredentials;
use tamtri_core::mcp::oauth::{
    OAuthConnectionStatus, OAuthResolveOutcome, StoredOAuthBundle, oauth_connection_status,
    parse_stored_oauth, resolve_oauth_access_token, serialize_stored_oauth,
};
use tamtri_core::vault::events::EventKind;
use tamtri_core::vault::fs::read_vault_events;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[derive(Default)]
struct NoopObserver;

impl ConversationObserver for NoopObserver {
    fn on_event(&self, _event: UiEvent) {}
}

fn http_oauth_server(endpoint: String, oauth: OAuthConfig) -> GatewayServerConfig {
    GatewayServerConfig {
        id: "remote".to_string(),
        display_name: "Remote".to_string(),
        enabled: true,
        scope: GatewayScope::User,
        transport: GatewayTransport::StreamableHttp {
            endpoint,
            headers: Vec::new(),
        },
        timeout_secs: Some(30),
        credentials: Vec::new(),
        oauth: Some(oauth),
    }
}

fn spawn_oauth_token_server_blocking() -> (String, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let token_endpoint = format!("http://127.0.0.1:{}/token", addr.port());
    let auth_endpoint = format!("http://127.0.0.1:{}/authorize", addr.port());
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let mut socket = stream;
                let mut buf = vec![0u8; 8192];
                let Ok(n) = socket.read(&mut buf) else {
                    return;
                };
                let request = String::from_utf8_lossy(&buf[..n]);
                let body = if request.contains("refresh_token=stale-refresh") {
                    r#"{"error":"invalid_grant"}"#
                } else {
                    r#"{"access_token":"access-new","refresh_token":"refresh-new","expires_in":3600}"#
                };
                let status = if body.contains("invalid_grant") {
                    "400 Bad Request"
                } else {
                    "200 OK"
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes());
            });
        }
    });
    (token_endpoint, auth_endpoint)
}

async fn spawn_oauth_token_server() -> (String, String) {
    spawn_oauth_token_server_blocking()
}

async fn spawn_minimal_http_mcp_server() -> (String, mpsc::Receiver<std::collections::HashMap<String, String>>) {
    let listener = AsyncTcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (seen_tx, seen_rx) = mpsc::channel(16);
    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else { break };
            let seen_tx = seen_tx.clone();
            tokio::spawn(async move {
                let _ = handle_http_mcp_connection(socket, seen_tx).await;
            });
        }
    });
    (format!("http://{addr}/mcp"), seen_rx)
}

async fn handle_http_mcp_connection(
    mut socket: TcpStream,
    seen_tx: mpsc::Sender<std::collections::HashMap<String, String>>,
) -> std::io::Result<()> {
    let mut buffer = Vec::new();
    loop {
        let mut chunk = [0u8; 1024];
        let read = socket.read(&mut chunk).await?;
        if read == 0 { break; }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(header_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            let header_text = String::from_utf8_lossy(&buffer[..header_end]);
            let headers: std::collections::HashMap<String, String> = header_text
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
                .collect();
            let content_length = headers
                .get("content-length")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let body_start = header_end + 4;
            while buffer.len() < body_start + content_length {
                let read = socket.read(&mut chunk).await?;
                if read == 0 { break; }
                buffer.extend_from_slice(&chunk[..read]);
            }
            let body = buffer[body_start..body_start + content_length].to_vec();
            let _ = seen_tx.send(headers).await;
            let message: serde_json::Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
            let method = message.get("method").and_then(serde_json::Value::as_str).unwrap_or("");
            let id = message.get("id").cloned().unwrap_or(serde_json::Value::Null);
            let (status, response_json) = match method {
                "initialize" => (200, json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": {"tools": {"listChanged": false}},
                        "serverInfo": {"name": "mock-http", "version": "0.1.0"}
                    }
                })),
                "notifications/initialized" => {
                    let text = "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n";
                    socket.write_all(text.as_bytes()).await?;
                    return Ok(());
                }
                "tools/list" => (200, json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [{
                            "name": "http_echo",
                            "description": "Echo over HTTP",
                            "inputSchema": {"type": "object"}
                        }]
                    }
                })),
                _ => (200, json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {"code": -32601, "message": "method not found"}
                })),
            };
            let body_text = serde_json::to_string(&response_json).unwrap();
            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                status,
                body_text.len(),
                body_text
            );
            socket.write_all(response.as_bytes()).await?;
            return Ok(());
        }
    }
    Ok(())
}

fn oauth_pkce_flow_stores_token_reference_only_impl() {
    let (token_endpoint, auth_endpoint) = spawn_oauth_token_server_blocking();
    let temp = tempfile::tempdir().unwrap();
    let observer = Arc::new(NoopObserver);
    let core = TamtriCore::new(
        temp.path().to_string_lossy().into_owned(),
        observer,
    )
    .expect("core");

    let oauth = OAuthConfig {
        issuer: None,
        authorization_endpoint: Some(auth_endpoint),
        token_endpoint: Some(token_endpoint),
        client_id: "tamtri-test".to_string(),
        scopes: vec!["mcp".to_string()],
        token_ref: "keychain://remote-oauth".to_string(),
    };
    let server = http_oauth_server("http://127.0.0.1:9/mcp".to_string(), oauth.clone());
    tamtri_core::config::replace_gateway_servers(temp.path(), vec![server]).unwrap();

    let redirect_uri = "http://127.0.0.1:3847/callback";
    let handoff = core
        .start_oauth_flow("remote".to_string(), redirect_uri.to_string())
        .expect("start oauth");
    assert!(handoff.authorization_url.contains("code_challenge="));

    let callback = format!(
        "{redirect_uri}?code=auth-code&state={}",
        handoff.state
    );
    let completion = core
        .complete_oauth_callback(callback)
        .expect("complete oauth");
    assert_eq!(completion.oauth_status, "connected");

    let config_text = std::fs::read_to_string(temp.path().join("config.json")).unwrap();
    assert!(!config_text.contains("access-new"));
    assert!(config_text.contains("keychain://remote-oauth"));

    let events = read_vault_events(temp.path()).expect("read events");
    let kinds: Vec<EventKind> = events.iter().map(|event| event.kind.clone()).collect();
    assert!(
        kinds.contains(&EventKind::OAuthHandoffStarted),
        "expected oauth_handoff_started receipt, saw {kinds:?}"
    );
    assert!(
        kinds.contains(&EventKind::OAuthHandoffCompleted),
        "expected oauth_handoff_completed receipt, saw {kinds:?}"
    );
    let serialized = serde_json::to_string(&events).unwrap();
    assert!(!serialized.contains("access-new"));
    assert!(!serialized.contains("refresh-new"));

    let servers = core.list_gateway_servers().expect("servers");
    assert_eq!(servers[0].oauth_status, "connected");
}

#[test]
fn oauth_pkce_flow_stores_token_reference_only() {
    oauth_pkce_flow_stores_token_reference_only_impl();
}

#[test]
fn oauth_pkce_flow_records_handoff_receipts_and_stores_token_ref_only() {
    oauth_pkce_flow_stores_token_reference_only_impl();
}

async fn oauth_refresh_success_updates_keychain_impl() {
    let (token_endpoint, _) = spawn_oauth_token_server().await;
    let credentials = Arc::new(MemoryCredentials::default());
    let token_ref = "keychain://remote".to_string();
    let bundle = StoredOAuthBundle {
        access_token: "stale-access".to_string(),
        refresh_token: Some("refresh-ok".to_string()),
        expires_at: Some(Utc::now().timestamp() - 10),
        reauth_required: false,
    };
    credentials
        .set(token_ref.clone(), serialize_stored_oauth(&bundle).unwrap())
        .unwrap();

    let oauth = OAuthConfig {
        issuer: None,
        authorization_endpoint: Some("https://auth.example.com/authorize".to_string()),
        token_endpoint: Some(token_endpoint),
        client_id: "tamtri-test".to_string(),
        scopes: vec![],
        token_ref: token_ref.clone(),
    };
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let stored = credentials.get_stored(&token_ref).unwrap().unwrap();
    let (outcome, updated_bundle) = resolve_oauth_access_token(&client, &oauth, &stored)
        .await
        .unwrap();
    if let Some(bundle) = updated_bundle {
        credentials
            .set(token_ref.clone(), serialize_stored_oauth(&bundle).unwrap())
            .unwrap();
    }
    assert!(matches!(outcome, OAuthResolveOutcome::AccessToken(_)));
    let updated = parse_stored_oauth(&credentials.get_stored(&token_ref).unwrap().unwrap()).unwrap();
    assert_eq!(updated.access_token, "access-new");
}

/// OAuth gateway test; `MemoryCredentials` mirrors keychain preload in core tests.
/// Real keychain round-trips live in `KeychainCredentialStoreTests` (Swift).
#[tokio::test]
async fn oauth_refresh_success_updates_keychain() {
    oauth_refresh_success_updates_keychain_impl().await;
}

#[tokio::test]
async fn oauth_refresh_success_updates_persisted_credential_memory_store() {
    oauth_refresh_success_updates_keychain_impl().await;
}

#[tokio::test]
async fn oauth_silent_refresh_emits_credential_updated_event() {
    use tamtri_core::mcp::gateway::{GatewayEvent, McpGateway};

    let (token_endpoint, _auth_endpoint) = spawn_oauth_token_server().await;
    let (mcp_url, mut seen_rx) = spawn_minimal_http_mcp_server().await;

    let credentials = Arc::new(MemoryCredentials::default());
    let token_ref = "keychain://remote".to_string();
    let stale = StoredOAuthBundle {
        access_token: "stale-access".to_string(),
        refresh_token: Some("refresh-ok".to_string()),
        expires_at: Some(Utc::now().timestamp() - 10),
        reauth_required: false,
    };
    credentials
        .set(token_ref.clone(), serialize_stored_oauth(&stale).unwrap())
        .unwrap();

    let oauth = OAuthConfig {
        issuer: None,
        authorization_endpoint: Some("https://auth.example.com/authorize".to_string()),
        token_endpoint: Some(token_endpoint),
        client_id: "tamtri-test".to_string(),
        scopes: vec![],
        token_ref: token_ref.clone(),
    };

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 30,
            servers: vec![http_oauth_server(mcp_url, oauth)],
        },
        credentials.clone(),
        Some(event_tx),
    )
    .unwrap();

    let _ = gateway.list_tools().await.unwrap();

    // Ensure the http server saw an Authorization header.
    let headers = tokio::time::timeout(std::time::Duration::from_secs(2), seen_rx.recv())
        .await
        .unwrap()
        .expect("seen headers");
    assert!(headers.contains_key("authorization"));

    // Ensure gateway emitted a credential-updated event with no token value.
    let mut saw_updated = false;
    let mut serialized = String::new();
    while let Ok(Some(event)) =
        tokio::time::timeout(std::time::Duration::from_millis(200), event_rx.recv()).await
    {
        if matches!(event, GatewayEvent::CredentialUpdated { .. }) {
            saw_updated = true;
        }
        serialized.push_str(&serde_json::to_string(&event).unwrap());
    }
    assert!(saw_updated, "expected CredentialUpdated event");
    assert!(!serialized.contains("access-new"));

    // Confirm the stored bundle is updated.
    let updated_raw = credentials.get_stored(&token_ref).unwrap().unwrap();
    let updated = parse_stored_oauth(&updated_raw).unwrap();
    assert_eq!(updated.access_token, "access-new");
}

#[tokio::test]
async fn oauth_refresh_failure_marks_reauth_required() {
    let listener = AsyncTcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let token_endpoint = format!("http://127.0.0.1:{}/token", addr.port());
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let body = r#"{"error":"invalid_grant"}"#;
                let response = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });

    let credentials = Arc::new(MemoryCredentials::default());
    let token_ref = "keychain://remote".to_string();
    let bundle = StoredOAuthBundle {
        access_token: "stale-access".to_string(),
        refresh_token: Some("stale-refresh".to_string()),
        expires_at: Some(Utc::now().timestamp() - 10),
        reauth_required: false,
    };
    credentials
        .set(token_ref.clone(), serialize_stored_oauth(&bundle).unwrap())
        .unwrap();

    let oauth = OAuthConfig {
        issuer: None,
        authorization_endpoint: None,
        token_endpoint: Some(token_endpoint),
        client_id: "tamtri-test".to_string(),
        scopes: vec![],
        token_ref: token_ref.clone(),
    };
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let stored = credentials.get_stored(&token_ref).unwrap().unwrap();
    let (outcome, updated_bundle) = resolve_oauth_access_token(&client, &oauth, &stored)
        .await
        .unwrap();
    if let Some(bundle) = updated_bundle {
        credentials
            .set(token_ref.clone(), serialize_stored_oauth(&bundle).unwrap())
            .unwrap();
    }
    assert!(matches!(outcome, OAuthResolveOutcome::ReauthRequired));
    let updated = parse_stored_oauth(&credentials.get_stored(&token_ref).unwrap().unwrap()).unwrap();
    assert!(updated.reauth_required);
    assert_eq!(
        oauth_connection_status(Some(&oauth), false, None, true),
        OAuthConnectionStatus::ReauthRequired
    );
}

#[test]
fn oauth_refresh_failed_records_events_jsonl_receipt() {
    let (token_endpoint, _) = spawn_oauth_token_server_blocking();
    let temp = tempfile::tempdir().unwrap();
    let observer = Arc::new(NoopObserver);
    let core = TamtriCore::new(
        temp.path().to_string_lossy().into_owned(),
        observer,
    )
    .expect("core");

    let oauth = OAuthConfig {
        issuer: None,
        authorization_endpoint: None,
        token_endpoint: Some(token_endpoint),
        client_id: "tamtri-test".to_string(),
        scopes: vec![],
        token_ref: "keychain://remote-oauth".to_string(),
    };
    let server = http_oauth_server("http://127.0.0.1:9/mcp".to_string(), oauth);
    tamtri_core::config::replace_gateway_servers(temp.path(), vec![server]).unwrap();

    let bundle = StoredOAuthBundle {
        access_token: "stale-access".to_string(),
        refresh_token: Some("stale-refresh".to_string()),
        expires_at: Some(Utc::now().timestamp() - 10),
        reauth_required: false,
    };
    core.set_gateway_credential(
        "keychain://remote-oauth".to_string(),
        serialize_stored_oauth(&bundle).unwrap(),
    )
    .expect("credential");

    core.refresh_gateway_capabilities().expect("refresh");

    let events = read_vault_events(temp.path()).expect("read events");
    let kinds: Vec<EventKind> = events.iter().map(|event| event.kind.clone()).collect();
    assert!(
        kinds.contains(&EventKind::OAuthRefreshFailed),
        "expected oauth_refresh_failed receipt, saw {kinds:?}"
    );
    let serialized = serde_json::to_string(&events).unwrap();
    assert!(!serialized.contains("stale-access"));
    assert!(!serialized.contains("stale-refresh"));
    assert!(!serialized.contains("access-new"));
}
