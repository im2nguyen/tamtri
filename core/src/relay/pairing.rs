//! Pairing offer for remote clients (QR / deep-link URL fragment).

use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use super::crypto::{KeyPair, encode_base64};

/// Default relay endpoint. Override with `TAMTRI_RELAY_ENDPOINT` on the daemon.
pub const DEFAULT_RELAY_ENDPOINT: &str = "relay.tamtri.dev:443";

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayEndpoint {
    pub endpoint: String,
    pub use_tls: bool,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionOffer {
    pub v: u32,
    pub server_id: String,
    pub daemon_public_key_b64: String,
    pub relay: RelayEndpoint,
}

pub fn relay_endpoint_from_env() -> RelayEndpoint {
    let endpoint = std::env::var("TAMTRI_RELAY_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_RELAY_ENDPOINT.to_string());
    RelayEndpoint {
        use_tls: !endpoint.contains(":80"),
        endpoint,
    }
}

pub fn build_pairing_offer(server_id: impl Into<String>, keypair: &KeyPair) -> ConnectionOffer {
    ConnectionOffer {
        v: 1,
        server_id: server_id.into(),
        daemon_public_key_b64: encode_base64(&keypair.public_key_bytes()),
        relay: relay_endpoint_from_env(),
    }
}
