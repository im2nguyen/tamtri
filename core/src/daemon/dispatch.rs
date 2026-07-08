//! Maps wire protocol method names onto [`TamtriCore`] facade calls.
//!
//! Every facade method internally calls `runtime.block_on(...)`, so each call is
//! run on a blocking thread via [`tokio::task::spawn_blocking`]; calling them
//! directly on an async worker would panic with a nested-runtime error.

use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::app::{TamtriCore, TamtriError};
use crate::protocol::{method, params};
use crate::rpc::jsonrpc::{JsonRpcError, METHOD_NOT_FOUND};

const CORE_ERROR: i64 = -32000;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

fn rpc_err(code: i64, message: impl Into<String>) -> JsonRpcError {
    JsonRpcError {
        code,
        message: message.into(),
        data: None,
    }
}

fn parse<T: DeserializeOwned>(params: Option<Value>) -> Result<T, JsonRpcError> {
    let value = params.unwrap_or(Value::Null);
    serde_json::from_value(value).map_err(|err| rpc_err(INVALID_PARAMS, format!("invalid params: {err}")))
}

fn to_value<T: Serialize>(value: T) -> Result<Value, JsonRpcError> {
    serde_json::to_value(value).map_err(|err| rpc_err(INTERNAL_ERROR, format!("serialize result: {err}")))
}

/// Run a fallible facade call on a blocking thread and serialize its result.
async fn run<T, F>(f: F) -> Result<Value, JsonRpcError>
where
    F: FnOnce() -> Result<T, TamtriError> + Send + 'static,
    T: Serialize + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(Ok(value)) => to_value(value),
        Ok(Err(err)) => Err(rpc_err(CORE_ERROR, err.to_string())),
        Err(join) => Err(rpc_err(INTERNAL_ERROR, format!("task join error: {join}"))),
    }
}

/// Run an infallible facade call (returns a plain value, not a `Result`).
async fn run_infallible<T, F>(f: F) -> Result<Value, JsonRpcError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Serialize + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(value) => to_value(value),
        Err(join) => Err(rpc_err(INTERNAL_ERROR, format!("task join error: {join}"))),
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Dispatch a single request. `HELLO` is handled by the connection layer, not
/// here.
pub async fn dispatch(
    core: Arc<TamtriCore>,
    method_name: &str,
    params: Option<Value>,
) -> Result<Value, JsonRpcError> {
    match method_name {
        // --- Harness roster / models ---
        method::AGENTS_LIST => {
            let c = Arc::clone(&core);
            run(move || c.list_acp_agents()).await
        }
        method::AGENTS_MODELS => {
            let p: params::AgentsModels = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.list_acp_agent_models(p.agent_id)).await
        }

        // --- Conversations ---
        method::CONVERSATION_LIST => {
            let c = Arc::clone(&core);
            run(move || c.list_conversations()).await
        }
        method::CONVERSATION_LOAD => {
            let p: params::ConversationLoad = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.load_conversation(p.id)).await
        }
        method::CONVERSATION_CREATE => {
            let p: params::ConversationCreate = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.create_conversation(p.title, p.harness_id, p.model_id)).await
        }
        method::CONVERSATION_FORK => {
            let p: params::ConversationFork = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.fork_conversation(p.id, p.harness_id, p.model_id)).await
        }
        method::CONVERSATION_DELETE => {
            let p: params::ConversationDelete = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.delete_conversation(p.id)).await
        }
        method::CONVERSATION_SEND_MESSAGE => {
            let p: params::ConversationSendMessage = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.send_message(p.conversation_id, p.text)).await
        }
        method::CONVERSATION_FOLDER_PATH => {
            let p: params::ConversationFolderPath = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.conversation_folder_path(p.conversation_id)).await
        }
        method::CONVERSATION_EXPORT_BUNDLE => {
            let p: params::ConversationExportBundle = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.export_conversation_bundle(p.conversation_id, p.dest_path)).await
        }
        method::CONVERSATION_IMPORT => {
            let p: params::ConversationImport = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.import_bundle_or_folder_as_new(p.source_path)).await
        }

        // --- Run control ---
        method::RUN_CANCEL => {
            let p: params::RunCancel = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.cancel_run(p.conversation_id)).await
        }
        method::PERMISSION_RESPOND => {
            let p: params::PermissionRespond = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.respond_permission(p.conversation_id, p.request_id, p.option_id)).await
        }
        method::ELICITATION_RESPOND => {
            let p: params::ElicitationRespond = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.respond_elicitation(p.conversation_id, p.request_id, p.action, p.data_json))
                .await
        }
        method::TASK_CANCEL => {
            let p: params::TaskCancel = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.cancel_task(p.conversation_id, p.task_id)).await
        }

        // --- Roots ---
        method::ROOTS_LIST => {
            let p: params::RootsList = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.list_roots(p.conversation_id)).await
        }
        method::ROOTS_ATTACH => {
            let p: params::RootsAttach = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.attach_root(p.conversation_id, p.name, p.uri, p.kind, p.scope)).await
        }
        method::ROOTS_REMOVE => {
            let p: params::RootsRemove = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.remove_root(p.conversation_id, p.root_id)).await
        }
        method::ROOTS_SYNC_RUNTIME => {
            let p: params::RootsSyncRuntime = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.sync_runtime_roots(p.conversation_id, p.roots)).await
        }

        // --- Workdir / attachments / artifacts ---
        method::WORKDIR_COPY_FILE => {
            let p: params::WorkdirCopyFile = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.copy_file_to_workdir(p.conversation_id, p.source_path)).await
        }
        method::WORKDIR_LIST_FILES => {
            let p: params::WorkdirListFiles = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.list_workdir_files(p.conversation_id)).await
        }
        method::WORKDIR_PATH => {
            let p: params::WorkdirPath = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.conversation_workdir_path(p.conversation_id)).await
        }
        method::WORKDIR_READ_FILE => {
            let p: params::WorkdirReadFile = parse(params)?;
            let c = Arc::clone(&core);
            match tokio::task::spawn_blocking(move || {
                c.read_workdir_file(p.conversation_id, p.relative_path)
            })
            .await
            {
                Ok(Ok(dto)) => to_value(params::WorkdirFileContent {
                    mime_type: dto.mime_type,
                    data_base64: base64_encode(&dto.data),
                }),
                Ok(Err(err)) => Err(rpc_err(CORE_ERROR, err.to_string())),
                Err(join) => Err(rpc_err(INTERNAL_ERROR, format!("task join error: {join}"))),
            }
        }
        method::ATTACHMENT_READ_VERIFIED => {
            let p: params::AttachmentReadVerified = parse(params)?;
            let c = Arc::clone(&core);
            match tokio::task::spawn_blocking(move || {
                c.read_attachment_verified(p.conversation_id, p.path, p.size, p.sha256)
            })
            .await
            {
                Ok(Ok(bytes)) => to_value(params::AttachmentContent {
                    data_base64: base64_encode(&bytes),
                }),
                Ok(Err(err)) => Err(rpc_err(CORE_ERROR, err.to_string())),
                Err(join) => Err(rpc_err(INTERNAL_ERROR, format!("task join error: {join}"))),
            }
        }
        method::ATTACHMENT_VERIFIED_PATH => {
            let p: params::AttachmentVerifiedPath = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.verified_attachment_path(p.conversation_id, p.path, p.size, p.sha256)).await
        }
        method::ARTIFACT_VERIFY_INLINE => {
            let p: params::ArtifactVerifyInline = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.verify_artifact_inline(p.size, p.sha256, p.inline_content)).await
        }
        method::ARTIFACT_LOG_NAVIGATION_BLOCKED => {
            let p: params::ArtifactLogNavigationBlocked = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.log_artifact_navigation_blocked(p.conversation_id, p.url)).await
        }

        // --- MCP Apps ---
        method::APP_RESOLVE_TEMPLATE => {
            let p: params::AppResolveTemplate = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.resolve_app_template(p.conversation_id, p.server_id, p.template_ref)).await
        }
        method::APP_SUBMIT_BRIDGE_REQUEST => {
            let p: params::AppSubmitBridgeRequest = parse(params)?;
            let c = Arc::clone(&core);
            run(move || {
                c.submit_app_bridge_request(
                    p.conversation_id,
                    p.server_id,
                    p.app_id,
                    p.template_ref,
                    p.request_json,
                )
            })
            .await
        }
        method::APP_RESPOND_BRIDGE_CONSENT => {
            let p: params::AppRespondBridgeConsent = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.respond_app_bridge_consent(p.conversation_id, p.request_id, p.option_id))
                .await
        }
        method::APP_LOG_NAVIGATION_BLOCKED => {
            let p: params::AppLogNavigationBlocked = parse(params)?;
            let c = Arc::clone(&core);
            run(move || {
                c.log_app_navigation_blocked(p.conversation_id, p.server_id, p.template_ref, p.url)
            })
            .await
        }
        method::APP_BRIDGE_BOOTSTRAP_SCRIPT => {
            let c = Arc::clone(&core);
            run_infallible(move || c.app_bridge_bootstrap_script()).await
        }
        method::APP_PREPARE_QUIT => {
            let c = Arc::clone(&core);
            run(move || c.prepare_for_app_quit()).await
        }

        // --- Gateway (MCP servers + credentials + oauth) ---
        method::GATEWAY_LIST_SERVERS => {
            let c = Arc::clone(&core);
            run(move || c.list_gateway_servers()).await
        }
        method::GATEWAY_REFRESH_CAPABILITIES => {
            let c = Arc::clone(&core);
            run(move || c.refresh_gateway_capabilities()).await
        }
        method::GATEWAY_LIST_TOOLS => {
            let c = Arc::clone(&core);
            run(move || c.list_gateway_tools()).await
        }
        method::GATEWAY_GET_SETTINGS => {
            let c = Arc::clone(&core);
            run(move || c.get_gateway_settings()).await
        }
        method::GATEWAY_SET_DEFAULT_TIMEOUT => {
            let p: params::GatewaySetDefaultTimeout = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.set_gateway_default_timeout(p.default_call_timeout_secs)).await
        }
        method::GATEWAY_SAVE_SERVERS => {
            let p: params::GatewaySaveServers = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.save_gateway_servers(p.servers)).await
        }
        method::GATEWAY_SET_CREDENTIAL => {
            let p: params::GatewaySetCredential = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.set_gateway_credential(p.credential_ref, p.value)).await
        }
        method::GATEWAY_EXPORT_CREDENTIAL => {
            let p: params::GatewayExportCredential = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.export_gateway_credential(p.credential_ref)).await
        }
        method::GATEWAY_START_OAUTH => {
            let p: params::GatewayStartOauth = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.start_oauth_flow(p.server_id, p.redirect_uri)).await
        }
        method::GATEWAY_COMPLETE_OAUTH => {
            let p: params::GatewayCompleteOauth = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.complete_oauth_callback(p.callback_url)).await
        }

        // --- Search / health / vault / diagnostics ---
        method::SEARCH_CONVERSATIONS => {
            let p: params::SearchConversations = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.search_conversations(p.query)).await
        }
        method::SEARCH_SCOPE_MESSAGE => {
            let c = Arc::clone(&core);
            run_infallible(move || c.search_scope_message()).await
        }
        method::HARNESS_HEALTH_LIST => {
            let c = Arc::clone(&core);
            run(move || c.list_harness_health()).await
        }
        method::HARNESS_HEALTH_CHECKLIST => {
            let c = Arc::clone(&core);
            run(move || c.harness_health_checklist()).await
        }
        method::VAULT_ISSUES => {
            let c = Arc::clone(&core);
            run(move || c.vault_issues()).await
        }
        method::VAULT_PATH => {
            let c = Arc::clone(&core);
            run_infallible(move || c.vault_path()).await
        }
        method::DIAGNOSTICS_WRITE_BUNDLE => {
            let p: params::DiagnosticsWriteBundle = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.write_diagnostics_bundle(p.dest_path, p.system_info_json)).await
        }
        method::RELAY_PAIRING_OFFER => {
            let c = Arc::clone(&core);
            run(move || c.relay_pairing_offer()).await
        }
        method::SESSIONS_LIST_NATIVE => {
            let c = Arc::clone(&core);
            run(move || c.list_native_sessions()).await
        }
        method::SESSIONS_IMPORT => {
            let p: params::SessionsImport = parse(params)?;
            let c = Arc::clone(&core);
            run(move || {
                c.import_native_session(p.provider, p.path, p.harness_id, p.model_id)
            })
            .await
        }

        method::RECIPES_LIST => {
            let c = Arc::clone(&core);
            run(move || c.list_recipes()).await
        }
        method::RECIPES_LOAD => {
            let p: params::RecipesLoad = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.load_recipe(p.recipe_id)).await
        }
        method::ORCHESTRATION_RUN => {
            let p: params::OrchestrationRun = parse(params)?;
            let c = Arc::clone(&core);
            run(move || {
                c.orchestration_run(
                    p.recipe_id,
                    p.source_conversation_id,
                    p.inputs_json,
                )
            })
            .await
        }
        method::ORCHESTRATION_STATUS => {
            let p: params::OrchestrationStatus = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.orchestration_status(p.run_id)).await
        }
        method::ORCHESTRATION_CANCEL => {
            let p: params::OrchestrationCancel = parse(params)?;
            let c = Arc::clone(&core);
            run(move || c.orchestration_cancel(p.run_id)).await
        }

        other => Err(rpc_err(METHOD_NOT_FOUND, format!("method not found: {other}"))),
    }
}
