use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use chrono::Utc;
use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::config::{
    GatewayScope, GatewayServerConfig, GatewayTransport, OAuthConfig,
};
use tamtri_core::mcp::gateway::MemoryCredentials;
use tamtri_core::mcp::oauth::{
    OAuthConnectionStatus, OAuthResolveOutcome, StoredOAuthBundle, oauth_connection_status,
    parse_stored_oauth, resolve_oauth_access_token, serialize_stored_oauth,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener as AsyncTcpListener;

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

#[test]
fn oauth_pkce_flow_stores_token_reference_only() {
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

    let servers = core.list_gateway_servers().expect("servers");
    assert_eq!(servers[0].oauth_status, "connected");
}

#[tokio::test]
async fn oauth_refresh_success_updates_keychain() {
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
    assert!(matches!(outcome, OAuthResolveOutcome::AccessToken(_)));
    let updated = parse_stored_oauth(&credentials.get_stored(&token_ref).unwrap().unwrap()).unwrap();
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
