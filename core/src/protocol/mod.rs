//! The tamtri daemon wire protocol.
//!
//! This module is the single source of truth for the application-level message
//! vocabulary the shell (and future web/mobile clients) speak to the daemon. It
//! rides the JSON-RPC 2.0 envelope in [`crate::rpc::jsonrpc`]: client-to-daemon
//! calls are requests correlated by id, and daemon-to-client streaming (the old
//! `ConversationObserver` push) arrives as [`method::EVENT`] notifications.
//!
//! Compatibility contract (borrowed from paseo's discipline):
//! - Wire schemas are append-only. Add fields as `Option` with a sensible
//!   default; never flip optional to required, never remove or narrow a field.
//! - New capabilities are advertised in [`ServerInfo::features`] and gated by the
//!   client. An old client keeps working; it just does not light up new features.
//! - Every back-compat shim carries a `COMPAT(name)` comment with the version it
//!   was added in so the full cleanup list is one grep away.

use serde::{Deserialize, Serialize};
use typeshare::typeshare;

pub mod params;

/// Wire protocol version. Bump the minor on additive changes, the major only on
/// a break (which the compatibility contract is designed to avoid).
pub const PROTOCOL_VERSION: &str = "1.0";

/// A 64-bit unsigned wire integer (byte sizes, timeouts). Transparent alias for
/// `u64` so the Rust side stays exact. typeshare hard-rejects the bare `u64`
/// token, so the wire structs use this alias and typeshare.toml maps it to the
/// TypeScript `number` (our u64 values never approach 2^53).
pub type WireU64 = u64;

/// Method names for client-to-daemon requests and daemon-to-client
/// notifications. Names are dotted and namespaced by domain. JSON-RPC correlates
/// a request with its response by `id`, so there is no `.request`/`.response`
/// suffix on the method itself; the pairing is structural.
pub mod method {
    // Handshake
    pub const HELLO: &str = "hello";

    // Streaming push (daemon -> client), params are a [`super::EventNotification`].
    pub const EVENT: &str = "event";

    // Harness roster / models
    pub const AGENTS_LIST: &str = "agents.list";
    pub const AGENTS_MODELS: &str = "agents.models";

    // Conversations
    pub const CONVERSATION_LIST: &str = "conversation.list";
    pub const CONVERSATION_LOAD: &str = "conversation.load";
    pub const CONVERSATION_CREATE: &str = "conversation.create";
    pub const CONVERSATION_FORK: &str = "conversation.fork";
    pub const CONVERSATION_DELETE: &str = "conversation.delete";
    pub const CONVERSATION_SEND_MESSAGE: &str = "conversation.send_message";
    pub const CONVERSATION_FOLDER_PATH: &str = "conversation.folder_path";
    pub const CONVERSATION_EXPORT_BUNDLE: &str = "conversation.export_bundle";
    pub const CONVERSATION_IMPORT: &str = "conversation.import";

    // Run control
    pub const RUN_CANCEL: &str = "run.cancel";
    pub const PERMISSION_RESPOND: &str = "permission.respond";
    pub const ELICITATION_RESPOND: &str = "elicitation.respond";
    pub const TASK_CANCEL: &str = "task.cancel";

    // Roots
    pub const ROOTS_LIST: &str = "roots.list";
    pub const ROOTS_ATTACH: &str = "roots.attach";
    pub const ROOTS_REMOVE: &str = "roots.remove";
    pub const ROOTS_SYNC_RUNTIME: &str = "roots.sync_runtime";

    // Workdir / attachments / artifacts
    pub const WORKDIR_COPY_FILE: &str = "workdir.copy_file";
    pub const WORKDIR_LIST_FILES: &str = "workdir.list_files";
    pub const WORKDIR_PATH: &str = "workdir.path";
    pub const WORKDIR_READ_FILE: &str = "workdir.read_file";
    pub const ATTACHMENT_READ_VERIFIED: &str = "attachment.read_verified";
    pub const ATTACHMENT_VERIFIED_PATH: &str = "attachment.verified_path";
    pub const ARTIFACT_VERIFY_INLINE: &str = "artifact.verify_inline";
    pub const ARTIFACT_LOG_NAVIGATION_BLOCKED: &str = "artifact.log_navigation_blocked";

    // MCP Apps
    pub const APP_RESOLVE_TEMPLATE: &str = "app.resolve_template";
    pub const APP_SUBMIT_BRIDGE_REQUEST: &str = "app.submit_bridge_request";
    pub const APP_RESPOND_BRIDGE_CONSENT: &str = "app.respond_bridge_consent";
    pub const APP_LOG_NAVIGATION_BLOCKED: &str = "app.log_navigation_blocked";
    pub const APP_BRIDGE_BOOTSTRAP_SCRIPT: &str = "app.bridge_bootstrap_script";
    pub const APP_PREPARE_QUIT: &str = "app.prepare_quit";

    // Gateway (MCP servers + credentials + oauth)
    pub const GATEWAY_LIST_SERVERS: &str = "gateway.list_servers";
    pub const GATEWAY_REFRESH_CAPABILITIES: &str = "gateway.refresh_capabilities";
    pub const GATEWAY_LIST_TOOLS: &str = "gateway.list_tools";
    pub const GATEWAY_GET_SETTINGS: &str = "gateway.get_settings";
    pub const GATEWAY_SET_DEFAULT_TIMEOUT: &str = "gateway.set_default_timeout";
    pub const GATEWAY_SAVE_SERVERS: &str = "gateway.save_servers";
    pub const GATEWAY_SET_CREDENTIAL: &str = "gateway.set_credential";
    pub const GATEWAY_EXPORT_CREDENTIAL: &str = "gateway.export_credential";
    pub const GATEWAY_START_OAUTH: &str = "gateway.start_oauth";
    pub const GATEWAY_COMPLETE_OAUTH: &str = "gateway.complete_oauth";

    // Search / health / vault / diagnostics
    pub const SEARCH_CONVERSATIONS: &str = "search.conversations";
    pub const SEARCH_SCOPE_MESSAGE: &str = "search.scope_message";
    pub const HARNESS_HEALTH_LIST: &str = "harness.health_list";
    pub const HARNESS_HEALTH_CHECKLIST: &str = "harness.health_checklist";
    pub const VAULT_ISSUES: &str = "vault.issues";
    pub const VAULT_PATH: &str = "vault.path";
    pub const DIAGNOSTICS_WRITE_BUNDLE: &str = "diagnostics.write_bundle";

    // Relay (remote access)
    pub const RELAY_PAIRING_OFFER: &str = "relay.pairing_offer";

    // Native session import (Workstream B)
    pub const SESSIONS_LIST_NATIVE: &str = "sessions.list_native";
    pub const SESSIONS_IMPORT: &str = "sessions.import";

    // Orchestration (Workstream C)
    pub const RECIPES_LIST: &str = "recipes.list";
    pub const RECIPES_LOAD: &str = "recipes.load";
    pub const ORCHESTRATION_RUN: &str = "orchestration.run";
    pub const ORCHESTRATION_STATUS: &str = "orchestration.status";
    pub const ORCHESTRATION_CANCEL: &str = "orchestration.cancel";
}

/// Client kind, advertised in the [`Hello`] handshake. Mirrors paseo's
/// `clientType` so the daemon can reason about who is connected.
#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientType {
    /// The native macOS shell.
    Desktop,
    /// A browser / PWA client.
    Browser,
    /// A mobile client (iOS/Android).
    Mobile,
    /// The command-line client.
    Cli,
}

/// First frame a client sends after connecting. The daemon replies with
/// [`ServerInfo`].
#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub client_id: String,
    pub client_type: ClientType,
    pub protocol_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
}

/// Capability flags. Additive only. A client checks the flag before using a
/// feature; a `false` (or absent, via `#[serde(default)]`) flag means the daemon
/// does not support it and the client shows an "update the host" affordance
/// rather than degrading.
///
/// COMPAT(features): every field added here is gated at the call site with a
/// `// COMPAT(<feature>): added in v<x>` marker so the cleanup list is greppable.
#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Features {
    /// Daemon exposes the configurable orchestration engine (Workstream C).
    #[serde(default)]
    pub orchestration: bool,
    /// Daemon can hand native tool catalogs to adapters that accept them
    /// (Workstream B).
    #[serde(default)]
    pub native_tools: bool,
    /// Daemon can import native harness sessions into vault conversations
    /// (Workstream B).
    #[serde(default)]
    pub session_import: bool,
    /// Daemon can bridge remote clients through the E2E relay (Workstream A).
    #[serde(default)]
    pub relay: bool,
}

/// Daemon identity + capabilities, returned in response to [`Hello`].
#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_id: String,
    pub version: String,
    pub protocol_version: String,
    #[serde(default)]
    pub features: Features,
}

/// Params for a [`method::EVENT`] notification: the daemon-to-client push that
/// replaces the in-process `ConversationObserver` callback. `payload_json`
/// carries the reduced event content exactly as the shell already parses it.
#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventNotification {
    pub conversation_id: String,
    pub kind: String,
    pub payload_json: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hello_round_trips() {
        let hello = Hello {
            client_id: "shell-1".to_string(),
            client_type: ClientType::Desktop,
            protocol_version: PROTOCOL_VERSION.to_string(),
            app_version: Some("0.1.0".to_string()),
        };
        let value = serde_json::to_value(&hello).expect("serialize hello");
        assert_eq!(value["client_type"], json!("desktop"));
        let back: Hello = serde_json::from_value(value).expect("deserialize hello");
        assert_eq!(back, hello);
    }

    #[test]
    fn server_info_defaults_features_when_absent() {
        // An old daemon that predates `features` still parses in a new client.
        let value = json!({
            "server_id": "daemon-1",
            "version": "0.1.0",
            "protocol_version": PROTOCOL_VERSION,
        });
        let info: ServerInfo = serde_json::from_value(value).expect("deserialize server_info");
        assert_eq!(info.features, Features::default());
        assert!(!info.features.orchestration);
    }

    #[test]
    fn features_are_additive_and_forward_compatible() {
        // A new daemon sends an unknown future flag; an old client ignores it.
        let value = json!({
            "orchestration": true,
            "native_tools": true,
            "some_future_flag": true,
        });
        let features: Features = serde_json::from_value(value).expect("deserialize features");
        assert!(features.orchestration);
        assert!(features.native_tools);
        assert!(!features.session_import);
    }

    #[test]
    fn event_notification_round_trips() {
        let event = EventNotification {
            conversation_id: "conv-1".to_string(),
            kind: "text_delta".to_string(),
            payload_json: r#"{"text":"hi"}"#.to_string(),
        };
        let value = serde_json::to_value(&event).expect("serialize event");
        let back: EventNotification = serde_json::from_value(value).expect("deserialize event");
        assert_eq!(back, event);
    }

    #[test]
    fn method_names_are_unique() {
        let names = [
            method::HELLO,
            method::EVENT,
            method::AGENTS_LIST,
            method::AGENTS_MODELS,
            method::CONVERSATION_LIST,
            method::CONVERSATION_LOAD,
            method::CONVERSATION_CREATE,
            method::CONVERSATION_FORK,
            method::CONVERSATION_DELETE,
            method::CONVERSATION_SEND_MESSAGE,
            method::CONVERSATION_FOLDER_PATH,
            method::CONVERSATION_EXPORT_BUNDLE,
            method::CONVERSATION_IMPORT,
            method::RUN_CANCEL,
            method::PERMISSION_RESPOND,
            method::ELICITATION_RESPOND,
            method::TASK_CANCEL,
            method::ROOTS_LIST,
            method::ROOTS_ATTACH,
            method::ROOTS_REMOVE,
            method::ROOTS_SYNC_RUNTIME,
            method::WORKDIR_COPY_FILE,
            method::WORKDIR_LIST_FILES,
            method::WORKDIR_PATH,
            method::WORKDIR_READ_FILE,
            method::ATTACHMENT_READ_VERIFIED,
            method::ATTACHMENT_VERIFIED_PATH,
            method::ARTIFACT_VERIFY_INLINE,
            method::ARTIFACT_LOG_NAVIGATION_BLOCKED,
            method::APP_RESOLVE_TEMPLATE,
            method::APP_SUBMIT_BRIDGE_REQUEST,
            method::APP_RESPOND_BRIDGE_CONSENT,
            method::APP_LOG_NAVIGATION_BLOCKED,
            method::APP_BRIDGE_BOOTSTRAP_SCRIPT,
            method::APP_PREPARE_QUIT,
            method::GATEWAY_LIST_SERVERS,
            method::GATEWAY_REFRESH_CAPABILITIES,
            method::GATEWAY_LIST_TOOLS,
            method::GATEWAY_GET_SETTINGS,
            method::GATEWAY_SET_DEFAULT_TIMEOUT,
            method::GATEWAY_SAVE_SERVERS,
            method::GATEWAY_SET_CREDENTIAL,
            method::GATEWAY_EXPORT_CREDENTIAL,
            method::GATEWAY_START_OAUTH,
            method::GATEWAY_COMPLETE_OAUTH,
            method::SEARCH_CONVERSATIONS,
            method::SEARCH_SCOPE_MESSAGE,
            method::HARNESS_HEALTH_LIST,
            method::HARNESS_HEALTH_CHECKLIST,
            method::VAULT_ISSUES,
            method::VAULT_PATH,
            method::DIAGNOSTICS_WRITE_BUNDLE,
            method::RELAY_PAIRING_OFFER,
            method::SESSIONS_LIST_NATIVE,
            method::SESSIONS_IMPORT,
        ];
        let mut seen = std::collections::HashSet::new();
        for name in names {
            assert!(seen.insert(name), "duplicate method name: {name}");
        }
    }
}
