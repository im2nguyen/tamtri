//! Request parameter shapes and binary-bearing result shapes for the daemon
//! protocol. These are part of the shared schema: the same field names appear on
//! every client. They mirror the `TamtriCore` facade signatures 1:1.
//!
//! Binary payloads (`Vec<u8>`) are carried as base64 strings on the wire rather
//! than JSON number arrays, so attachment and workdir reads stay compact. The
//! dispatcher does the encode; clients decode.

use serde::{Deserialize, Serialize};

use crate::app::{GatewayServerDto, RootDto};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsModels {
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationLoad {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationCreate {
    pub title: String,
    pub harness_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationFork {
    pub id: String,
    pub harness_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDelete {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSendMessage {
    pub conversation_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationFolderPath {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationExportBundle {
    pub conversation_id: String,
    pub dest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationImport {
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCancel {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRespond {
    pub conversation_id: String,
    pub request_id: String,
    pub option_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRespond {
    pub conversation_id: String,
    pub request_id: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancel {
    pub conversation_id: String,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsList {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsAttach {
    pub conversation_id: String,
    pub name: String,
    pub uri: String,
    pub kind: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsRemove {
    pub conversation_id: String,
    pub root_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsSyncRuntime {
    pub conversation_id: String,
    pub roots: Vec<RootDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkdirCopyFile {
    pub conversation_id: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkdirListFiles {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkdirPath {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkdirReadFile {
    pub conversation_id: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentReadVerified {
    pub conversation_id: String,
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentVerifiedPath {
    pub conversation_id: String,
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVerifyInline {
    pub size: u64,
    pub sha256: String,
    pub inline_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactLogNavigationBlocked {
    pub conversation_id: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppResolveTemplate {
    pub conversation_id: String,
    pub server_id: String,
    pub template_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSubmitBridgeRequest {
    pub conversation_id: String,
    pub server_id: String,
    pub app_id: String,
    pub template_ref: String,
    pub request_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRespondBridgeConsent {
    pub conversation_id: String,
    pub request_id: String,
    pub option_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLogNavigationBlocked {
    pub conversation_id: String,
    pub server_id: String,
    pub template_ref: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySetDefaultTimeout {
    pub default_call_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySaveServers {
    pub servers: Vec<GatewayServerDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySetCredential {
    pub credential_ref: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayExportCredential {
    pub credential_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStartOauth {
    pub server_id: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayCompleteOauth {
    pub callback_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConversations {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsWriteBundle {
    pub dest_path: String,
    pub system_info_json: String,
}

// --- Binary-bearing result shapes (base64 on the wire) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkdirFileContent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentContent {
    pub data_base64: String,
}
