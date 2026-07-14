use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::conversation::Id;
use crate::mcp::gateway::{McpGateway, gateway_exposed_resource_uri, gateway_exposed_tool_name};
use crate::mcp::protocol::CallToolResult;
use crate::{CoreError, Result};

pub const APP_BRIDGE_ALLOW_ONCE: &str = "allow_once";
pub const APP_BRIDGE_ALLOW_FOR_CONVERSATION: &str = "allow_for_conversation";
pub const APP_BRIDGE_DENY: &str = "deny";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppBridgeActionKind {
    CallTool,
    ReadResource,
    SetState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppBridgeRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppBridgeAction {
    CallTool { name: String, arguments: Value },
    ReadResource { uri: String },
    SetState { state: Value },
}

impl AppBridgeAction {
    pub fn kind(&self) -> AppBridgeActionKind {
        match self {
            Self::CallTool { .. } => AppBridgeActionKind::CallTool,
            Self::ReadResource { .. } => AppBridgeActionKind::ReadResource,
            Self::SetState { .. } => AppBridgeActionKind::SetState,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppBridgeConsentRequest {
    pub request_id: String,
    pub server_id: String,
    pub app_id: String,
    pub template_ref: String,
    pub rpc_id: Value,
    pub action: AppBridgeAction,
    pub summary: String,
    pub options: Value,
}

pub fn parse_app_bridge_rpc(raw: &str) -> Result<AppBridgeRpcRequest> {
    let request: AppBridgeRpcRequest = serde_json::from_str(raw)
        .map_err(|err| CoreError::Protocol(format!("invalid app bridge JSON-RPC: {err}")))?;
    if request.jsonrpc != "2.0" {
        return Err(CoreError::Protocol(
            "app bridge request must use JSON-RPC 2.0".to_string(),
        ));
    }
    if request.method.trim().is_empty() {
        return Err(CoreError::Protocol(
            "app bridge request missing method".to_string(),
        ));
    }
    Ok(request)
}

pub fn action_from_rpc(request: &AppBridgeRpcRequest) -> Result<AppBridgeAction> {
    match request.method.as_str() {
        "tools/call" => {
            let name = request
                .params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| CoreError::Protocol("tools/call missing name".to_string()))?
                .to_string();
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(AppBridgeAction::CallTool { name, arguments })
        }
        "resources/read" => {
            let uri = request
                .params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| CoreError::Protocol("resources/read missing uri".to_string()))?
                .to_string();
            Ok(AppBridgeAction::ReadResource { uri })
        }
        "app/setState" => {
            let state = request
                .params
                .get("state")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(AppBridgeAction::SetState { state })
        }
        other => Err(CoreError::Protocol(format!(
            "unsupported app bridge method: {other}"
        ))),
    }
}

pub fn consent_summary(
    server_id: &str,
    app_id: &str,
    template_ref: &str,
    action: &AppBridgeAction,
) -> String {
    match action {
        AppBridgeAction::CallTool { name, arguments } => format!(
            "App {app_id} ({template_ref}) on {server_id} wants to call tool {name} with {}",
            summarize_arguments(arguments)
        ),
        AppBridgeAction::ReadResource { uri } => {
            format!("App {app_id} ({template_ref}) on {server_id} wants to read resource {uri}")
        }
        AppBridgeAction::SetState { state } => format!(
            "App {app_id} ({template_ref}) on {server_id} wants to update state ({})",
            summarize_arguments(state)
        ),
    }
}

fn summarize_arguments(value: &Value) -> String {
    let raw = value.to_string();
    if raw.len() <= 240 {
        raw
    } else {
        format!("{}…", &raw[..240])
    }
}

pub fn consent_options() -> Value {
    json!([
        {"id": APP_BRIDGE_DENY, "label": "Deny"},
        {"id": APP_BRIDGE_ALLOW_ONCE, "label": "Allow once"},
        {"id": APP_BRIDGE_ALLOW_FOR_CONVERSATION, "label": "Allow for this conversation"}
    ])
}

pub fn rpc_success(id: &Value, result: Value) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

pub fn rpc_error(id: &Value, code: i32, message: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

fn grant_key(action: &AppBridgeAction) -> String {
    match action {
        AppBridgeAction::CallTool { name, .. } => format!("call_tool:{name}"),
        AppBridgeAction::ReadResource { uri } => format!("read_resource:{uri}"),
        AppBridgeAction::SetState { .. } => "set_state".to_string(),
    }
}

#[derive(Debug)]
pub enum AppBridgeBeginResult {
    NeedsConsent(AppBridgeConsentRequest, oneshot::Receiver<String>),
    AlreadyGranted(PendingAppBridgeExecution),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConversationGrantKey {
    conversation_id: Id,
    server_id: String,
    grant_key: String,
}

struct PendingAppBridge {
    conversation_id: Id,
    server_id: String,
    app_id: String,
    template_ref: String,
    rpc_id: Value,
    action: AppBridgeAction,
    response_tx: oneshot::Sender<String>,
}

#[derive(Default)]
pub struct AppBridgeCoordinator {
    pending: Mutex<HashMap<String, PendingAppBridge>>,
    conversation_grants: Mutex<HashMap<ConversationGrantKey, ()>>,
}

impl AppBridgeCoordinator {
    fn has_grant(&self, conversation_id: Id, server_id: &str, action: &AppBridgeAction) -> bool {
        let key = ConversationGrantKey {
            conversation_id,
            server_id: server_id.to_string(),
            grant_key: grant_key(action),
        };
        self.conversation_grants
            .lock()
            .map(|grants| grants.contains_key(&key))
            .unwrap_or(false)
    }

    fn record_grant(&self, conversation_id: Id, server_id: &str, action: &AppBridgeAction) {
        if let Ok(mut grants) = self.conversation_grants.lock() {
            grants.insert(
                ConversationGrantKey {
                    conversation_id,
                    server_id: server_id.to_string(),
                    grant_key: grant_key(action),
                },
                (),
            );
        }
    }

    pub fn begin_request(
        &self,
        conversation_id: Id,
        server_id: &str,
        app_id: &str,
        template_ref: &str,
        request: &AppBridgeRpcRequest,
    ) -> Result<AppBridgeBeginResult> {
        let action = action_from_rpc(request)?;
        if self.has_grant(conversation_id, server_id, &action) {
            let (response_tx, _response_rx) = oneshot::channel();
            return Ok(AppBridgeBeginResult::AlreadyGranted(
                PendingAppBridgeExecution {
                    rpc_id: request.id.clone(),
                    server_id: server_id.to_string(),
                    action,
                    response_tx,
                },
            ));
        }
        let request_id = Uuid::now_v7().to_string();
        let summary = consent_summary(server_id, app_id, template_ref, &action);
        let consent = AppBridgeConsentRequest {
            request_id: request_id.clone(),
            server_id: server_id.to_string(),
            app_id: app_id.to_string(),
            template_ref: template_ref.to_string(),
            rpc_id: request.id.clone(),
            action: action.clone(),
            summary,
            options: consent_options(),
        };
        let (response_tx, response_rx) = oneshot::channel();
        self.pending
            .lock()
            .map_err(|_| CoreError::Protocol("app bridge lock poisoned".to_string()))?
            .insert(
                request_id,
                PendingAppBridge {
                    conversation_id,
                    server_id: server_id.to_string(),
                    app_id: app_id.to_string(),
                    template_ref: template_ref.to_string(),
                    rpc_id: request.id.clone(),
                    action,
                    response_tx,
                },
            );
        Ok(AppBridgeBeginResult::NeedsConsent(consent, response_rx))
    }

    pub fn resolve_consent(
        &self,
        conversation_id: Id,
        request_id: &str,
        option_id: &str,
    ) -> Result<AppBridgeResolution> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| CoreError::Protocol("app bridge lock poisoned".to_string()))?
            .remove(request_id)
            .ok_or_else(|| CoreError::Protocol("unknown app bridge request".to_string()))?;
        if pending.conversation_id != conversation_id {
            return Err(CoreError::Protocol(
                "app bridge request conversation mismatch".to_string(),
            ));
        }

        let audit = AppBridgeAuditRecord {
            server_id: pending.server_id.clone(),
            app_id: pending.app_id.clone(),
            template_ref: pending.template_ref.clone(),
            action_kind: pending.action.kind(),
            arguments_summary: audit_arguments(&pending.action),
            resolution: option_id.to_string(),
        };

        if option_id == APP_BRIDGE_DENY {
            let response = rpc_error(&pending.rpc_id, -32001, "user denied app bridge action");
            let _ = pending.response_tx.send(response.clone());
            return Ok(AppBridgeResolution::Denied { response, audit });
        }
        if option_id == APP_BRIDGE_ALLOW_ONCE || option_id == APP_BRIDGE_ALLOW_FOR_CONVERSATION {
            if option_id == APP_BRIDGE_ALLOW_FOR_CONVERSATION {
                self.record_grant(conversation_id, &pending.server_id, &pending.action);
            }
            return Ok(AppBridgeResolution::Approved {
                pending: pending.into(),
                audit,
            });
        }
        Err(CoreError::Protocol(format!(
            "unknown app bridge consent option: {option_id}"
        )))
    }

    pub fn pending_count(&self) -> usize {
        self.pending
            .lock()
            .map(|pending| pending.len())
            .unwrap_or(0)
    }
}

pub type SharedAppBridgeCoordinator = Arc<AppBridgeCoordinator>;

pub fn shared_app_bridge_coordinator() -> SharedAppBridgeCoordinator {
    Arc::new(AppBridgeCoordinator::default())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppBridgeAuditRecord {
    pub server_id: String,
    pub app_id: String,
    pub template_ref: String,
    pub action_kind: AppBridgeActionKind,
    pub arguments_summary: String,
    pub resolution: String,
}

pub enum AppBridgeResolution {
    Denied {
        response: String,
        audit: AppBridgeAuditRecord,
    },
    Approved {
        pending: PendingAppBridgeExecution,
        audit: AppBridgeAuditRecord,
    },
}

#[derive(Debug)]
pub struct PendingAppBridgeExecution {
    pub rpc_id: Value,
    pub server_id: String,
    pub action: AppBridgeAction,
    pub response_tx: oneshot::Sender<String>,
}

impl From<PendingAppBridge> for PendingAppBridgeExecution {
    fn from(pending: PendingAppBridge) -> Self {
        Self {
            rpc_id: pending.rpc_id,
            server_id: pending.server_id,
            action: pending.action,
            response_tx: pending.response_tx,
        }
    }
}

pub async fn execute_action(
    gateway: &McpGateway,
    server_id: &str,
    action: &AppBridgeAction,
) -> Result<Value> {
    match action {
        AppBridgeAction::CallTool { name, arguments } => {
            let exposed = gateway_exposed_tool_name(server_id, name);
            let result = gateway
                .call_tool(&exposed, arguments.clone())
                .await
                .map_err(|err| CoreError::Protocol(err.to_string()))?;
            Ok(call_tool_result_to_json(&result))
        }
        AppBridgeAction::ReadResource { uri } => {
            let exposed = gateway_exposed_resource_uri(server_id, uri);
            let result = gateway
                .read_resource(&exposed)
                .await
                .map_err(|err| CoreError::Protocol(err.to_string()))?;
            Ok(json!({
                "contents": result.contents,
            }))
        }
        AppBridgeAction::SetState { state } => Ok(json!({ "state": state })),
    }
}

pub fn finish_execution(pending: PendingAppBridgeExecution, result: Result<Value>) -> String {
    let response = match result {
        Ok(result) => rpc_success(&pending.rpc_id, result),
        Err(err) => rpc_error(&pending.rpc_id, -32002, &err.to_string()),
    };
    let _ = pending.response_tx.send(response.clone());
    response
}

fn audit_arguments(action: &AppBridgeAction) -> String {
    match action {
        AppBridgeAction::CallTool { name, arguments } => {
            format!("tool={name} args={}", summarize_arguments(arguments))
        }
        AppBridgeAction::ReadResource { uri } => format!("uri={uri}"),
        AppBridgeAction::SetState { state } => summarize_arguments(state),
    }
}

fn call_tool_result_to_json(result: &CallToolResult) -> Value {
    json!({
        "content": result.content,
        "is_error": result.is_error,
        "structured_content": result.structured_content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::Id;

    #[test]
    fn app_bridge_action_requires_consent() {
        let coordinator = AppBridgeCoordinator::default();
        let conversation_id = Id::now_v7();
        let request = parse_app_bridge_rpc(
            r#"{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{"name":"echo","arguments":{"value":1}}}"#,
        )
        .unwrap();
        let result = coordinator
            .begin_request(
                conversation_id,
                "fixture",
                "demo-app",
                "demo-template",
                &request,
            )
            .unwrap();
        let (consent, _rx) = match result {
            AppBridgeBeginResult::NeedsConsent(consent, rx) => (consent, rx),
            AppBridgeBeginResult::AlreadyGranted(_) => panic!("expected consent"),
        };
        assert_eq!(
            consent.action,
            AppBridgeAction::CallTool {
                name: "echo".into(),
                arguments: json!({"value": 1}),
            }
        );
        assert!(consent.summary.contains("fixture"));
        assert_eq!(coordinator.pending_count(), 1);
    }

    #[test]
    fn app_bridge_denied_action_not_executed() {
        let coordinator = AppBridgeCoordinator::default();
        let conversation_id = Id::now_v7();
        let request = parse_app_bridge_rpc(
            r#"{"jsonrpc":"2.0","id":"42","method":"tools/call","params":{"name":"echo","arguments":{}}}"#,
        )
        .unwrap();
        let result = coordinator
            .begin_request(
                conversation_id,
                "fixture",
                "demo-app",
                "demo-template",
                &request,
            )
            .unwrap();
        let (consent, mut rx) = match result {
            AppBridgeBeginResult::NeedsConsent(consent, rx) => (consent, rx),
            AppBridgeBeginResult::AlreadyGranted(_) => panic!("expected consent"),
        };
        let (response, audit) = match coordinator
            .resolve_consent(conversation_id, &consent.request_id, APP_BRIDGE_DENY)
            .unwrap()
        {
            AppBridgeResolution::Denied { response, audit } => (response, audit),
            AppBridgeResolution::Approved { .. } => panic!("expected deny"),
        };
        assert!(response.contains("user denied"));
        assert_eq!(audit.resolution, APP_BRIDGE_DENY);
        assert_eq!(coordinator.pending_count(), 0);
        let delivered = rx.try_recv().unwrap();
        assert!(delivered.contains("user denied"));
    }

    #[test]
    fn app_bridge_allow_for_conversation_skips_repeat_consent() {
        let coordinator = AppBridgeCoordinator::default();
        let conversation_id = Id::now_v7();
        let request = parse_app_bridge_rpc(
            r#"{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{"name":"echo","arguments":{}}}"#,
        )
        .unwrap();
        let first = coordinator
            .begin_request(
                conversation_id,
                "fixture",
                "demo-app",
                "demo-template",
                &request,
            )
            .unwrap();
        let (consent, _rx) = match first {
            AppBridgeBeginResult::NeedsConsent(consent, rx) => (consent, rx),
            AppBridgeBeginResult::AlreadyGranted(_) => panic!("expected first consent"),
        };
        match coordinator
            .resolve_consent(
                conversation_id,
                &consent.request_id,
                APP_BRIDGE_ALLOW_FOR_CONVERSATION,
            )
            .unwrap()
        {
            AppBridgeResolution::Approved { audit, .. } => {
                assert_eq!(audit.resolution, APP_BRIDGE_ALLOW_FOR_CONVERSATION);
            }
            AppBridgeResolution::Denied { .. } => panic!("expected approve"),
        }
        let second = coordinator
            .begin_request(
                conversation_id,
                "fixture",
                "demo-app",
                "demo-template",
                &request,
            )
            .unwrap();
        assert!(matches!(second, AppBridgeBeginResult::AlreadyGranted(_)));
    }
}
