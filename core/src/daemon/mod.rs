//! The tamtri daemon: hosts the [`TamtriCore`] facade and serves the wire
//! protocol (see [`crate::protocol`]) to one or more connected clients.
//!
//! This module owns the transport-neutral pieces: constructing the core with a
//! broadcasting observer, fanning out events to every connected client, and
//! dispatching inbound requests onto the facade. The concrete transport
//! (WebSocket/HTTP, relay) layers on top and is added separately.

pub mod dispatch;

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::app::{ConversationObserver, TamtriCore, UiEvent};
use crate::protocol::{EventNotification, Features, PROTOCOL_VERSION, ServerInfo};
use crate::rpc::jsonrpc::JsonRpcError;
use crate::{CoreError, Result};

/// How many events the broadcast channel buffers per subscriber before a slow
/// client starts lagging. A lagging client is told to resync via the
/// authoritative transcript fetch rather than blocking the daemon.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Bridges the core's in-process `ConversationObserver` push callback onto a
/// broadcast channel so every connected client receives the same event stream.
struct BroadcastObserver {
    tx: broadcast::Sender<EventNotification>,
}

impl ConversationObserver for BroadcastObserver {
    fn on_event(&self, event: UiEvent) {
        // A send error only means there are no subscribers right now; the event
        // is still persisted by the core, so clients recover on next fetch.
        let _ = self.tx.send(EventNotification {
            conversation_id: event.conversation_id,
            kind: event.kind,
            payload_json: event.payload_json,
        });
    }
}

/// The headless daemon. Wraps a single [`TamtriCore`] (single writer) and
/// broadcasts its events to any number of client connections (many readers).
pub struct Daemon {
    core: Arc<TamtriCore>,
    events_tx: broadcast::Sender<EventNotification>,
}

impl Daemon {
    /// Construct the daemon over a vault path. Call this off the async executor
    /// (the core builds its own runtime); the WebSocket layer does so on a
    /// blocking thread.
    pub fn new(vault_path: impl Into<String>) -> Result<Self> {
        let vault_path = vault_path.into();
        crate::config::seed_vault_content(std::path::Path::new(&vault_path))?;
        let (events_tx, _rx) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let observer = Arc::new(BroadcastObserver {
            tx: events_tx.clone(),
        });
        let core = TamtriCore::new(vault_path, observer)
            .map_err(|err| CoreError::Protocol(format!("core init failed: {err}")))?;
        let core = Arc::new(core);
        TamtriCore::install_shared(Arc::clone(&core));
        Ok(Self { core, events_tx })
    }

    /// Subscribe a new client connection to the event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<EventNotification> {
        self.events_tx.subscribe()
    }

    /// Shared handle to the core facade.
    pub fn core(&self) -> Arc<TamtriCore> {
        Arc::clone(&self.core)
    }

    /// Identity + capabilities returned in response to a client `hello`.
    /// Feature flags flip on as their workstreams land; today the baseline
    /// protocol is available and no optional capability is advertised yet.
    pub fn server_info(&self) -> ServerInfo {
        ServerInfo {
            server_id: self.core.server_id().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION.to_string(),
            features: Features {
                orchestration: true,
                relay: true,
                native_tools: true,
                session_import: true,
                harness_roster: true,
                provider_usage: true,
                projects: true,
            },
        }
    }

    /// Route one inbound request to the facade.
    pub async fn dispatch(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        dispatch::dispatch(self.core(), method, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::UiEvent;

    #[test]
    fn broadcast_observer_forwards_events_to_subscribers() {
        let (tx, mut rx) = broadcast::channel(8);
        let observer = BroadcastObserver { tx };
        observer.on_event(UiEvent {
            conversation_id: "conv-1".to_string(),
            kind: "text_delta".to_string(),
            payload_json: r#"{"text":"hi"}"#.to_string(),
        });
        let received = rx.try_recv().expect("event forwarded");
        assert_eq!(received.conversation_id, "conv-1");
        assert_eq!(received.kind, "text_delta");
    }
}
