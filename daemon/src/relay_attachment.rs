//! Optional outbound WebSocket registration with the E2E relay.
//!
//! When `TAMTRI_RELAY_ENDPOINT` is set (or the default is used), the daemon
//! maintains a long-lived connection so remote clients can route ciphertext to
//! this host. The relay sees only registration metadata and encrypted frames.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tamtri_core::relay::{load_or_create_keypair, relay_endpoint_from_env, encode_base64};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const RETRY_DELAY: Duration = Duration::from_secs(15);

pub fn spawn_if_enabled(server_id: String, tamtri_home: PathBuf) {
    if std::env::var("TAMTRI_RELAY_DISABLE")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"))
    {
        tracing::info!("relay attachment disabled via TAMTRI_RELAY_DISABLE");
        return;
    }

    tokio::spawn(async move {
        relay_attachment_loop(server_id, tamtri_home).await;
    });
}

async fn relay_attachment_loop(server_id: String, tamtri_home: PathBuf) {
    loop {
        match attach_once(&server_id, &tamtri_home).await {
            Ok(()) => tracing::warn!(server_id = %server_id, "relay connection closed; retrying"),
            Err(err) => tracing::debug!(server_id = %server_id, error = %err, "relay attach failed; retrying"),
        }
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

async fn attach_once(server_id: &str, tamtri_home: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = relay_endpoint_from_env();
    let keypair = load_or_create_keypair(tamtri_home)?;
    let scheme = if endpoint.use_tls { "wss" } else { "ws" };
    let url = format!("{scheme}://{}/v1/daemon/register", endpoint.endpoint.trim_end_matches('/'));

    tracing::info!(url = %url, server_id = %server_id, "connecting to relay");
    let (mut ws, _) = connect_async(&url).await?;
    let register = serde_json::json!({
        "type": "register",
        "server_id": server_id,
        "daemon_public_key_b64": encode_base64(&keypair.public_key_bytes()),
        "protocol_version": tamtri_core::protocol::PROTOCOL_VERSION,
    });
    ws.send(Message::Text(register.to_string().into())).await?;

    while let Some(frame) = ws.next().await {
        match frame {
            Ok(Message::Text(text)) => {
                tracing::debug!(target: "tamtri_daemon::relay", frame = %text, "relay frame");
            }
            Ok(Message::Ping(payload)) => {
                ws.send(Message::Pong(payload)).await?;
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}
