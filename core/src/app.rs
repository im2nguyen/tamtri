use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::{Builder, Runtime};
use url::Url;
use uuid::Uuid;

use crate::artifact::{detect_mime, verify_inline_artifact, ArtifactSnapshot, ArtifactSnapshotter, verify_attachment};
use crate::config::{
    load_app_config, replace_gateway_servers, GatewayScope, GatewayServerConfig, GatewayTransport,
};
use crate::conversation::reduce::TurnReducer;
use crate::conversation::{
    ContentBlock, Conversation, ElicitationAction, Id, McpServerRef, Message, Role, WorkingDir,
};
use crate::harness::acp::{AcpAdapter, AgentLaunchSpec};
use crate::harness::{
    ContextSeed, ConversationContext, HarnessAdapter, HarnessEvent, RunControl, TurnInput,
};
use crate::mcp::elicitation::{audit_safe_elicitation_url, sanitize_transcript_data};
use crate::mcp::oauth::{
    PkceChallenge, build_authorization_url, exchange_authorization_code, generate_pkce,
    oauth_connection_status, oauth_status_label, parse_stored_oauth, serialize_stored_oauth,
    stored_oauth_from_token_response, validate_callback_url,
};
use crate::mcp::url_handoff::validate_handoff_url;
use crate::mcp::endpoint::{GatewayEndpoint, start_loopback_gateway};
use crate::mcp::gateway::{GatewayEvent, McpGateway, MemoryCredentials};
use crate::vault::events::{Event, EventKind};
use crate::vault::fs::FilesystemVault;
use crate::vault::{ConversationSummary, ConversationVault};
use crate::debug_log::debug_log;
use crate::{CoreError, Result};

#[uniffi::export(foreign)]
pub trait ConversationObserver: Send + Sync {
    fn on_event(&self, event: UiEvent);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct UiEvent {
    pub conversation_id: String,
    pub kind: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ConversationSummaryDto {
    pub id: String,
    pub title: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ConversationDto {
    pub id: String,
    pub title: String,
    pub active_harness_id: Option<String>,
    pub model_id: Option<String>,
    pub transcript_json: String,
}

#[derive(Debug, Clone)]
struct CachedConversationDto {
    updated_at: DateTime<Utc>,
    dto: ConversationDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct WorkdirFileDto {
    pub relative_path: String,
    pub size: u64,
    pub mime_type: Option<String>,
    pub modified_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct WorkdirFileContentDto {
    pub mime_type: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct GatewayEnvVarDto {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct GatewayServerDto {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub scope: String,
    pub transport: String,
    pub stdio_command: String,
    pub stdio_args: Vec<String>,
    pub stdio_env: Vec<GatewayEnvVarDto>,
    pub http_endpoint: String,
    pub credential_refs: Vec<String>,
    pub missing_credential_refs: Vec<String>,
    pub oauth_status: String,
    pub oauth_token_ref: String,
    pub oauth_client_id: String,
    pub oauth_authorization_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct OAuthHandoffDto {
    pub server_id: String,
    pub authorization_url: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct OAuthCompletionDto {
    pub server_id: String,
    pub oauth_status: String,
}

type FfiResult<T> = std::result::Result<T, TamtriError>;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum TamtriError {
    #[error("{message}")]
    Core { message: String },
}

impl From<CoreError> for TamtriError {
    fn from(err: CoreError) -> Self {
        ffi_err(err)
    }
}

#[derive(Clone)]
struct PendingOAuthFlow {
    server_id: String,
    pkce: PkceChallenge,
    redirect_uri: String,
    token_ref: String,
}

#[derive(Clone)]
struct ActiveRun {
    control: RunControl,
    gateway: Arc<McpGateway>,
    gateway_blocks: Arc<Mutex<Vec<ContentBlock>>>,
}

#[derive(uniffi::Object)]
pub struct TamtriCore {
    vault: Arc<FilesystemVault>,
    runtime: Runtime,
    adapters: Arc<Mutex<HashMap<String, Arc<dyn HarnessAdapter>>>>,
    active_runs: Arc<Mutex<HashMap<Id, ActiveRun>>>,
    credentials: Arc<MemoryCredentials>,
    observer: Arc<dyn ConversationObserver>,
    conversation_cache: Arc<Mutex<HashMap<Id, CachedConversationDto>>>,
    pending_oauth: Arc<Mutex<HashMap<String, PendingOAuthFlow>>>,
}

const CONVERSATION_CACHE_LIMIT: usize = 32;

#[uniffi::export]
impl TamtriCore {
    #[uniffi::constructor]
    pub fn new(vault_path: String, observer: Arc<dyn ConversationObserver>) -> FfiResult<Self> {
        Self::new_inner(vault_path.into(), observer).map_err(ffi_err)
    }
}

impl TamtriCore {
    pub fn new_inner(vault_path: PathBuf, observer: Arc<dyn ConversationObserver>) -> Result<Self> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        Ok(Self {
            vault: Arc::new(FilesystemVault::new(vault_path)?),
            runtime,
            adapters: Arc::new(Mutex::new(HashMap::new())),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            credentials: Arc::new(MemoryCredentials::default()),
            observer,
            conversation_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_oauth: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn invalidate_conversation_cache(&self, id: Id) {
        if let Ok(mut cache) = self.conversation_cache.lock() {
            cache.remove(&id);
        }
    }

    fn store_conversation_cache(&self, updated_at: DateTime<Utc>, dto: ConversationDto) {
        let Ok(id) = parse_id(&dto.id) else {
            return;
        };
        let Ok(mut cache) = self.conversation_cache.lock() else {
            return;
        };
        if cache.len() >= CONVERSATION_CACHE_LIMIT
            && let Some(oldest) = cache.keys().next().copied()
        {
            cache.remove(&oldest);
        }
        cache.insert(id, CachedConversationDto { updated_at, dto });
    }

    fn register_acp_agent_spec(&self, spec: AgentLaunchSpec) -> Result<()> {
        let adapter = Arc::new(AcpAdapter::new(spec.clone()));
        self.adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?
            .insert(spec.id, adapter);
        Ok(())
    }
}

#[uniffi::export]
impl TamtriCore {
    pub fn register_acp_agent(
        &self,
        id: String,
        display_name: String,
        command: String,
        args: Vec<String>,
    ) -> FfiResult<()> {
        self.register_acp_agent_spec(AgentLaunchSpec {
            id,
            display_name,
            command,
            args,
            env: Vec::new(),
        })
        .map_err(ffi_err)
    }

    pub fn list_conversations(&self) -> FfiResult<Vec<ConversationSummaryDto>> {
        self.vault
            .list()?
            .into_iter()
            .map(summary_to_dto)
            .collect::<Result<Vec<_>>>()
            .map_err(ffi_err)
    }

    pub fn load_conversation(&self, id: String) -> FfiResult<ConversationDto> {
        self.load_conversation_inner(&id).map_err(ffi_err)
    }

    pub fn create_conversation(
        &self,
        title: String,
        harness_id: String,
        model_id: String,
    ) -> FfiResult<ConversationDto> {
        self.create_conversation_inner(&title, &harness_id, &model_id)
            .map_err(ffi_err)
    }

    pub fn fork_conversation(
        &self,
        id: String,
        harness_id: String,
        model_id: String,
    ) -> FfiResult<ConversationDto> {
        self.fork_conversation_inner(&id, &harness_id, &model_id)
            .map_err(ffi_err)
    }

    pub fn delete_conversation(&self, id: String) -> FfiResult<()> {
        let id = parse_id(&id)?;
        self.invalidate_conversation_cache(id);
        self.vault.delete(id).map_err(ffi_err)
    }

    pub fn send_message(&self, conversation_id: String, text: String) -> FfiResult<()> {
        self.send_message_inner(&conversation_id, &text)
            .map_err(ffi_err)
    }

    pub fn respond_permission(
        &self,
        conversation_id: String,
        request_id: String,
        option_id: String,
    ) -> FfiResult<()> {
        self.respond_permission_inner(&conversation_id, &request_id, &option_id)
            .map_err(ffi_err)
    }

    pub fn respond_elicitation(
        &self,
        conversation_id: String,
        request_id: String,
        action: String,
        data_json: Option<String>,
    ) -> FfiResult<()> {
        self.respond_elicitation_inner(&conversation_id, &request_id, &action, data_json.as_deref())
            .map_err(ffi_err)
    }

    pub fn cancel_run(&self, conversation_id: String) -> FfiResult<()> {
        self.cancel_run_inner(&conversation_id).map_err(ffi_err)
    }

    pub fn list_gateway_servers(&self) -> FfiResult<Vec<GatewayServerDto>> {
        self.list_gateway_servers_inner().map_err(ffi_err)
    }

    pub fn set_gateway_credential(&self, credential_ref: String, value: String) -> FfiResult<()> {
        self.credentials.set(credential_ref, value).map_err(ffi_err)
    }

    pub fn save_gateway_servers(&self, servers: Vec<GatewayServerDto>) -> FfiResult<()> {
        self.save_gateway_servers_inner(&servers).map_err(ffi_err)
    }

    pub fn start_oauth_flow(
        &self,
        server_id: String,
        redirect_uri: String,
    ) -> FfiResult<OAuthHandoffDto> {
        self.start_oauth_flow_inner(&server_id, &redirect_uri)
            .map_err(ffi_err)
    }

    pub fn complete_oauth_callback(
        &self,
        callback_url: String,
    ) -> FfiResult<OAuthCompletionDto> {
        self.complete_oauth_callback_inner(&callback_url)
            .map_err(ffi_err)
    }

    pub fn export_gateway_credential(&self, credential_ref: String) -> FfiResult<Option<String>> {
        self.credentials
            .get_stored(&credential_ref)
            .map_err(ffi_err)
    }

    pub fn copy_file_to_workdir(
        &self,
        conversation_id: String,
        source_path: String,
    ) -> FfiResult<String> {
        self.copy_file_to_workdir_inner(&conversation_id, source_path.into())
            .map_err(ffi_err)
    }

    pub fn list_workdir_files(
        &self,
        conversation_id: String,
    ) -> FfiResult<Vec<WorkdirFileDto>> {
        self.list_workdir_files_inner(&conversation_id)
            .map_err(ffi_err)
    }

    pub fn conversation_workdir_path(&self, conversation_id: String) -> FfiResult<String> {
        self.conversation_workdir_path_inner(&conversation_id)
            .map_err(ffi_err)
    }

    pub fn read_workdir_file(
        &self,
        conversation_id: String,
        relative_path: String,
    ) -> FfiResult<WorkdirFileContentDto> {
        self.read_workdir_file_inner(&conversation_id, &relative_path)
            .map_err(ffi_err)
    }

    pub fn read_attachment_verified(
        &self,
        conversation_id: String,
        path: String,
        size: u64,
        sha256: String,
    ) -> FfiResult<Vec<u8>> {
        self.read_attachment_verified_inner(&conversation_id, &path, size, &sha256)
            .map_err(ffi_err)
    }

    pub fn verified_attachment_path(
        &self,
        conversation_id: String,
        path: String,
        size: u64,
        sha256: String,
    ) -> FfiResult<String> {
        self.verified_attachment_path_inner(&conversation_id, &path, size, &sha256)
            .map_err(ffi_err)
    }

    pub fn verify_artifact_inline(
        &self,
        size: u64,
        sha256: String,
        inline_content: String,
    ) -> FfiResult<()> {
        verify_inline_artifact(size, &sha256, &inline_content).map_err(ffi_err)
    }

    pub fn log_artifact_navigation_blocked(
        &self,
        conversation_id: String,
        url: String,
    ) -> FfiResult<()> {
        let id = parse_id(&conversation_id)?;
        self.vault
            .append_event(
                id,
                &Event::new(
                    EventKind::ArtifactNavigationBlocked,
                    json!({ "url": url }),
                ),
            )
            .map_err(ffi_err)
    }
}

impl TamtriCore {
    pub fn load_conversation_inner(&self, id: &str) -> Result<ConversationDto> {
        let id = parse_id(id)?;
        let updated_at = self.vault.meta_updated_at(id)?;
        if let Ok(cache) = self.conversation_cache.lock()
            && let Some(cached) = cache.get(&id)
            && cached.updated_at == updated_at
        {
            debug_log(format!("[tamtri] load_conversation cache hit {id}"));
            return Ok(cached.dto.clone());
        }

        let started = std::time::Instant::now();
        let conversation = self.vault.load(id)?;
        let dto = conversation_to_dto(&conversation)?;
        self.store_conversation_cache(conversation.updated_at, dto.clone());
        debug_log(format!(
            "[tamtri] load_conversation cache miss {id} {:?} messages={}",
            started.elapsed(),
            conversation.messages.len()
        ));
        Ok(dto)
    }

    pub fn create_conversation_inner(
        &self,
        title: &str,
        harness_id: &str,
        model_id: &str,
    ) -> Result<ConversationDto> {
        let mut conversation = Conversation::new(title);
        conversation.active_harness_id = Some(harness_id.to_string());
        conversation.model_id = Some(model_id.to_string());
        self.vault.create(&conversation)?;
        let dto = conversation_to_dto(&conversation)?;
        self.store_conversation_cache(conversation.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn fork_conversation_inner(
        &self,
        id: &str,
        harness_id: &str,
        model_id: &str,
    ) -> Result<ConversationDto> {
        let id = parse_id(id)?;
        let mut fork = self.vault.load(id)?.fork();
        fork.active_harness_id = Some(harness_id.to_string());
        fork.model_id = Some(model_id.to_string());
        self.vault.create(&fork)?;
        let dto = conversation_to_dto(&fork)?;
        self.store_conversation_cache(fork.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn send_message_inner(&self, conversation_id: &str, text: &str) -> Result<()> {
        let id = parse_id(conversation_id)?;
        if self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .contains_key(&id)
        {
            return Err(CoreError::ConversationBusy(id));
        }

        let mut conversation = self.vault.load(id)?;
        let harness_id = conversation
            .active_harness_id
            .clone()
            .ok_or_else(|| CoreError::Protocol("conversation has no active harness".to_string()))?;
        let model_id = conversation
            .model_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let adapter = self.adapter(&harness_id)?;
        let conversation_dir = self.vault.conversation_folder(id)?;
        let workdir_path = self
            .vault
            .conversation_workdir(id)?
            .unwrap_or_else(|| self.vault.root().join("conversations"));
        let app_config = load_app_config(self.vault.root())?;
        let (gateway_event_tx, gateway_event_rx) = tokio::sync::mpsc::unbounded_channel();
        let gateway = Arc::new(McpGateway::new(
            app_config.gateway,
            self.credentials.clone(),
            Some(gateway_event_tx),
        )?);
        let gateway_endpoint = self
            .runtime
            .block_on(start_loopback_gateway(Arc::clone(&gateway)))?;
        let user_message = Message {
            id: Id::now_v7(),
            role: Role::User,
            harness_id: None,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            created_at: Utc::now(),
        };
        self.vault.append_message(id, &user_message)?;
        self.invalidate_conversation_cache(id);
        conversation.messages.push(user_message.clone());
        let ctx = ConversationContext {
            seed: ContextSeed::FreshTranscript {
                messages: conversation.messages.clone(),
            },
            working_dir: WorkingDir::VaultLocal,
            working_dir_path: workdir_path.clone(),
            roots: conversation.roots.clone(),
            mcp_servers: vec![gateway_mcp_ref(&gateway_endpoint)],
            model_id,
        };

        self.vault.append_event(
            id,
            &Event::new(EventKind::TurnStarted, json!({ "harness_id": harness_id })),
        )?;

        let vault = Arc::clone(&self.vault);
        let active_runs = Arc::clone(&self.active_runs);
        let observer = Arc::clone(&self.observer);
        let conversation_cache = Arc::clone(&self.conversation_cache);
        let gateway_blocks = Arc::new(Mutex::new(Vec::<ContentBlock>::new()));
        let gateway_for_run = Arc::clone(&gateway);
        self.runtime.spawn(async move {
            let gateway_vault = Arc::clone(&vault);
            let gateway_observer = Arc::clone(&observer);
            let gateway_blocks_for_events = Arc::clone(&gateway_blocks);
            let mut gateway_event_rx = gateway_event_rx;
            let gateway_event_task = tokio::spawn(async move {
                while let Some(event) = gateway_event_rx.recv().await {
                    record_gateway_content_block(
                        &gateway_blocks_for_events,
                        &event,
                    );
                    let _ = append_event_for_gateway_event(&gateway_vault, id, &event);
                    observer_emit_gateway(&gateway_observer, id, &event);
                }
            });
            let run = adapter.run(ctx, TurnInput { user_message }).await;
            match run {
                Ok(mut run) => {
                    if let Ok(mut runs) = active_runs.lock() {
                        runs.insert(
                            id,
                            ActiveRun {
                                control: run.control.clone(),
                                gateway: Arc::clone(&gateway_for_run),
                                gateway_blocks: Arc::clone(&gateway_blocks),
                            },
                        );
                    }
                    let mut reducer = TurnReducer::new(harness_id.clone());
                    while let Some(event) = run.events.recv().await {
                        emit(&observer, id, &event);
                        let _ = append_event_for_harness_event(&vault, id, &event);
                        let _ = reducer.apply(&event);
                        if matches!(event, HarnessEvent::TurnEnded { .. }) {
                            let reduced = reducer.finish();
                            let mut message = reduced.message;
                            let snapshotter =
                                ArtifactSnapshotter::new(&workdir_path, &conversation_dir);
                            let mut snapshotted = HashSet::new();
                            for change in &reduced.file_changes {
                                match snapshotter.snapshot_file_changed(&change.diff) {
                                    Ok(Some(snapshot)) => {
                                        snapshotted.insert(snapshot.attachment_path.clone());
                                        append_artifact_snapshot(
                                            &vault,
                                            id,
                                            snapshot,
                                            Some(&change.tool_call_id),
                                            &mut message,
                                        );
                                    }
                                    Ok(None) => {}
                                    Err(err) => {
                                        let _ = vault.append_event(
                                            id,
                                            &Event::new(
                                                EventKind::Error,
                                                json!({ "message": err.to_string() }),
                                            ),
                                        );
                                    }
                                }
                            }
                            match snapshotter.snapshot_renderable_files() {
                                Ok(snapshots) => {
                                    for snapshot in snapshots {
                                        if snapshotted.insert(snapshot.attachment_path.clone()) {
                                            append_artifact_snapshot(
                                                &vault,
                                                id,
                                                snapshot,
                                                None,
                                                &mut message,
                                            );
                                        }
                                    }
                                }
                                Err(err) => {
                                    let _ = vault.append_event(
                                        id,
                                        &Event::new(
                                            EventKind::Error,
                                            json!({ "message": err.to_string() }),
                                        ),
                                    );
                                }
                            }
                            if !message.content.is_empty() || !gateway_blocks.lock().unwrap().is_empty()
                            {
                                message
                                    .content
                                    .extend(gateway_blocks.lock().unwrap().drain(..));
                                let _ = vault.append_message(id, &message);
                                if let Ok(mut cache) = conversation_cache.lock() {
                                    cache.remove(&id);
                                }
                                observer.on_event(UiEvent {
                                    conversation_id: id.to_string(),
                                    kind: "message_committed".to_string(),
                                    payload_json: serde_json::to_string(&message)
                                        .unwrap_or_else(|_| "{}".to_string()),
                                });
                            }
                            break;
                        }
                    }
                    if let Ok(mut runs) = active_runs.lock() {
                        runs.remove(&id);
                    }
                }
                Err(err) => {
                    let _ = vault.append_event(
                        id,
                        &Event::new(EventKind::Error, json!({ "message": err.to_string() })),
                    );
                    observer.on_event(UiEvent {
                        conversation_id: id.to_string(),
                        kind: "error".to_string(),
                        payload_json: json!({ "message": err.to_string() }).to_string(),
                    });
                }
            }
            gateway_endpoint.shutdown().await;
            gateway_for_run.cancel_pending_elicitations().await;
            gateway_event_task.abort();
        });
        Ok(())
    }

    pub fn respond_permission_inner(
        &self,
        conversation_id: &str,
        request_id: &str,
        option_id: &str,
    ) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let control = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .map(|run| run.control.clone())
            .ok_or(CoreError::NotFound(id))?;
        self.runtime
            .block_on(control.respond_permission(request_id, option_id))
    }

    pub fn respond_elicitation_inner(
        &self,
        conversation_id: &str,
        request_id: &str,
        action: &str,
        data_json: Option<&str>,
    ) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let action = parse_elicitation_action(action)?;
        let data = match data_json {
            Some(raw) if !raw.trim().is_empty() => Some(serde_json::from_str(raw)?),
            _ => None,
        };
        let run = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .cloned()
            .ok_or(CoreError::NotFound(id))?;
        self.runtime
            .block_on(run.gateway.respond_elicitation(request_id, action.clone(), data.clone()))?;
        let response_data = data.map(|value| sanitize_transcript_data(&value));
        run.gateway_blocks
            .lock()
            .map_err(|_| CoreError::Protocol("gateway block lock poisoned".to_string()))?
            .push(ContentBlock::ElicitationResponse {
                request_id: request_id.to_string(),
                action,
                data: response_data,
            });
        Ok(())
    }

    pub fn cancel_run_inner(&self, conversation_id: &str) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let run = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .cloned()
            .ok_or(CoreError::NotFound(id))?;
        self.runtime.block_on(async {
            run.gateway.cancel_pending_elicitations().await;
            run.control.cancel().await
        })
    }

    pub fn list_gateway_servers_inner(&self) -> Result<Vec<GatewayServerDto>> {
        let config = load_app_config(self.vault.root())?;
        config
            .gateway
            .servers
            .into_iter()
            .map(|server| gateway_server_to_dto(&server, &self.credentials))
            .collect()
    }

    pub fn save_gateway_servers_inner(&self, servers: &[GatewayServerDto]) -> Result<()> {
        let config = load_app_config(self.vault.root())?;
        let existing_by_id = config
            .gateway
            .servers
            .into_iter()
            .map(|server| (server.id.clone(), server))
            .collect::<HashMap<_, _>>();
        let gateway_servers = servers
            .iter()
            .map(|server| gateway_server_from_dto(server, existing_by_id.get(&server.id)))
            .collect::<Result<Vec<_>>>()?;
        replace_gateway_servers(self.vault.root(), gateway_servers)
    }

    pub fn start_oauth_flow_inner(
        &self,
        server_id: &str,
        redirect_uri: &str,
    ) -> Result<OAuthHandoffDto> {
        let config = load_app_config(self.vault.root())?;
        let server = config
            .gateway
            .servers
            .iter()
            .find(|server| server.id == server_id)
            .ok_or_else(|| CoreError::Protocol(format!("unknown gateway server: {server_id}")))?;
        let oauth = server.oauth.as_ref().ok_or_else(|| {
            CoreError::Protocol(format!("gateway server {server_id} has no oauth config"))
        })?;
        let pkce = generate_pkce();
        let state = Uuid::now_v7().to_string();
        let authorization_url =
            build_authorization_url(oauth, redirect_uri, &pkce, &state)?;
        self.pending_oauth
            .lock()
            .map_err(|_| CoreError::Protocol("oauth flow lock poisoned".to_string()))?
            .insert(
                state.clone(),
                PendingOAuthFlow {
                    server_id: server_id.to_string(),
                    pkce,
                    redirect_uri: redirect_uri.to_string(),
                    token_ref: oauth.token_ref.clone(),
                },
            );
        Ok(OAuthHandoffDto {
            server_id: server_id.to_string(),
            authorization_url,
            state,
            redirect_uri: redirect_uri.to_string(),
        })
    }

    pub fn complete_oauth_callback_inner(
        &self,
        callback_url: &str,
    ) -> Result<OAuthCompletionDto> {
        let state = Url::parse(callback_url)
            .map_err(|err| CoreError::Protocol(format!("invalid callback URL: {err}")))?
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .ok_or_else(|| CoreError::Protocol("oauth callback missing state".to_string()))?;
        let pending = self
            .pending_oauth
            .lock()
            .map_err(|_| CoreError::Protocol("oauth flow lock poisoned".to_string()))?
            .remove(&state)
            .ok_or_else(|| CoreError::Protocol("unknown oauth flow state".to_string()))?;
        let config = load_app_config(self.vault.root())?;
        let server = config
            .gateway
            .servers
            .iter()
            .find(|server| server.id == pending.server_id)
            .ok_or_else(|| {
                CoreError::Protocol(format!("unknown gateway server: {}", pending.server_id))
            })?;
        let oauth = server.oauth.as_ref().ok_or_else(|| {
            CoreError::Protocol(format!(
                "gateway server {} has no oauth config",
                pending.server_id
            ))
        })?;
        let (code, _) = validate_callback_url(callback_url, &state)?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| CoreError::Protocol(format!("oauth http client failed: {err}")))?;
        let tokens = std::thread::scope(|scope| -> Result<crate::mcp::oauth::TokenEndpointResponse> {
            let handle = scope.spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|err| CoreError::Protocol(format!("oauth runtime failed: {err}")))?;
                rt.block_on(exchange_authorization_code(
                    &client,
                    oauth,
                    &code,
                    &pending.redirect_uri,
                    &pending.pkce,
                ))
            });
            handle
                .join()
                .map_err(|_| CoreError::Protocol("oauth exchange thread panicked".to_string()))?
        })?;
        let bundle = stored_oauth_from_token_response(&tokens);
        let serialized = serialize_stored_oauth(&bundle)?;
        self.credentials
            .set(pending.token_ref.clone(), serialized)?;
        let status = oauth_status_label(oauth_connection_status(
            Some(oauth),
            true,
            bundle.expires_at,
            false,
        ));
        Ok(OAuthCompletionDto {
            server_id: pending.server_id,
            oauth_status: status.to_string(),
        })
    }

    pub fn copy_file_to_workdir_inner(
        &self,
        conversation_id: &str,
        source_path: PathBuf,
    ) -> Result<String> {
        let id = parse_id(conversation_id)?;
        let workdir = self
            .vault
            .conversation_workdir(id)?
            .ok_or_else(|| CoreError::Protocol("conversation has no workdir".to_string()))?;
        fs::create_dir_all(&workdir)?;
        let filename = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| CoreError::Protocol("source file has no filename".to_string()))?;
        let safe_name = safe_workdir_filename(filename);
        fs::copy(&source_path, workdir.join(&safe_name))?;
        Ok(safe_name)
    }

    pub fn list_workdir_files_inner(&self, conversation_id: &str) -> Result<Vec<WorkdirFileDto>> {
        let id = parse_id(conversation_id)?;
        let workdir = self
            .vault
            .conversation_workdir(id)?
            .ok_or_else(|| CoreError::Protocol("conversation has no workdir".to_string()))?;
        if !workdir.exists() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        collect_workdir_files(&workdir, &workdir, &mut files)?;
        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(files)
    }

    pub fn conversation_workdir_path_inner(&self, conversation_id: &str) -> Result<String> {
        let id = parse_id(conversation_id)?;
        let workdir = self
            .vault
            .conversation_workdir(id)?
            .ok_or_else(|| CoreError::Protocol("conversation has no workdir".to_string()))?;
        Ok(workdir.to_string_lossy().into_owned())
    }

    pub fn read_workdir_file_inner(
        &self,
        conversation_id: &str,
        relative_path: &str,
    ) -> Result<WorkdirFileContentDto> {
        let id = parse_id(conversation_id)?;
        let workdir = self
            .vault
            .conversation_workdir(id)?
            .ok_or_else(|| CoreError::Protocol("conversation has no workdir".to_string()))?;
        let path = resolve_workdir_relative_path(&workdir, relative_path)?;
        let bytes = fs::read(&path)?;
        let mime_type = detect_mime(&path, &bytes);
        Ok(WorkdirFileContentDto { mime_type, data: bytes })
    }

    pub fn read_attachment_verified_inner(
        &self,
        conversation_id: &str,
        path: &str,
        size: u64,
        sha256: &str,
    ) -> Result<Vec<u8>> {
        let id = parse_id(conversation_id)?;
        let conversation_dir = self.vault.conversation_folder(id)?;
        let attachment = verify_attachment(&conversation_dir, path, size, sha256)?;
        Ok(fs::read(attachment)?)
    }

    pub fn verified_attachment_path_inner(
        &self,
        conversation_id: &str,
        path: &str,
        size: u64,
        sha256: &str,
    ) -> Result<String> {
        let id = parse_id(conversation_id)?;
        let conversation_dir = self.vault.conversation_folder(id)?;
        let attachment = verify_attachment(&conversation_dir, path, size, sha256)?;
        Ok(attachment.to_string_lossy().to_string())
    }

    fn adapter(&self, harness_id: &str) -> Result<Arc<dyn HarnessAdapter>> {
        self.adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?
            .get(harness_id)
            .cloned()
            .ok_or_else(|| CoreError::Protocol(format!("unknown harness: {harness_id}")))
    }
}

fn emit(observer: &Arc<dyn ConversationObserver>, conversation_id: Id, event: &HarnessEvent) {
    observer.on_event(UiEvent {
        conversation_id: conversation_id.to_string(),
        kind: event_kind(event).to_string(),
        payload_json: serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string()),
    });
}

fn append_event_for_harness_event(
    vault: &FilesystemVault,
    id: Id,
    event: &HarnessEvent,
) -> Result<()> {
    let (kind, payload) = match event {
        HarnessEvent::ToolCallStarted {
            id, name, input, ..
        } => (
            EventKind::ToolCallStarted,
            json!({ "id": id, "name": name, "input": input }),
        ),
        HarnessEvent::ToolCallProgress {
            id,
            status,
            content,
        } => (
            EventKind::ToolCallCompleted,
            json!({ "id": id, "status": status, "content": content }),
        ),
        HarnessEvent::PermissionRequested {
            request_id,
            action,
            detail,
            options,
        } => (
            EventKind::PermissionRequested,
            json!({ "request_id": request_id, "action": action, "detail": detail, "options": options }),
        ),
        HarnessEvent::PermissionResolved {
            request_id,
            option_id,
        } => (
            EventKind::PermissionResolved,
            json!({ "request_id": request_id, "option_id": option_id }),
        ),
        HarnessEvent::TurnEnded { reason } => (EventKind::TurnEnded, json!({ "reason": reason })),
        HarnessEvent::Error { message } => (EventKind::Error, json!({ "message": message })),
        _ => return Ok(()),
    };
    vault.append_event(id, &Event::new(kind, payload))
}

fn append_artifact_snapshot(
    vault: &FilesystemVault,
    id: Id,
    snapshot: ArtifactSnapshot,
    tool_call_id: Option<&str>,
    message: &mut Message,
) {
    let mut payload = json!({
        "original_path": snapshot.original_path.to_string_lossy(),
        "attachment_path": snapshot.attachment_path,
        "mime_type": snapshot.mime_type,
        "size": snapshot.size,
        "sha256": snapshot.sha256,
    });
    if let Some(tool_call_id) = tool_call_id {
        payload["tool_call_id"] = json!(tool_call_id);
    }
    let _ = vault.append_event(id, &Event::new(EventKind::ArtifactSnapshotted, payload));
    message.content.push(snapshot.block);
}

fn append_event_for_gateway_event(
    vault: &FilesystemVault,
    id: Id,
    event: &GatewayEvent,
) -> Result<()> {
    let (kind, payload) = match event {
        GatewayEvent::ServerConnected { server_id } => (
            EventKind::GatewayServerConnected,
            json!({ "server_id": server_id }),
        ),
        GatewayEvent::ToolRouted {
            server_id,
            exposed_name,
            original_name,
        } => (
            EventKind::GatewayToolRouted,
            json!({
                "server_id": server_id,
                "exposed_name": exposed_name,
                "original_name": original_name
            }),
        ),
        GatewayEvent::CredentialInjected {
            server_id,
            credential_ref,
            target_kind,
        } => (
            EventKind::GatewayCredentialInjected,
            json!({
                "server_id": server_id,
                "credential_ref": credential_ref,
                "target_kind": target_kind
            }),
        ),
        GatewayEvent::Progress { server_id, params } => (
            EventKind::GatewayProgress,
            json!({ "server_id": server_id, "params": sanitize_gateway_params(params) }),
        ),
        GatewayEvent::Log { server_id, params } => (
            EventKind::GatewayLog,
            json!({ "server_id": server_id, "params": sanitize_gateway_params(params) }),
        ),
        GatewayEvent::Cancellation { server_id, params } => (
            EventKind::GatewayCancellation,
            json!({ "server_id": server_id, "params": sanitize_gateway_params(params) }),
        ),
        GatewayEvent::DownstreamError { server_id, message } => (
            EventKind::GatewayDownstreamError,
            json!({ "server_id": server_id, "message": message }),
        ),
        GatewayEvent::ElicitationRequested {
            server_id,
            request_id,
            mode,
            message,
            schema,
            url,
            origin_tool_call_id,
        } => (
            EventKind::ElicitationRequested,
            json!({
                "server_id": server_id,
                "request_id": request_id,
                "mode": mode,
                "message": message,
                "schema": schema,
                "url": url.as_ref().map(|value| audit_safe_elicitation_url(value)),
                "url_origin": url.as_ref().and_then(|value| {
                    validate_handoff_url(value).ok().map(|validated| validated.origin)
                }),
                "origin_tool_call_id": origin_tool_call_id,
            }),
        ),
        GatewayEvent::ElicitationResolved {
            server_id,
            request_id,
            action,
        } => (
            EventKind::ElicitationResolved,
            json!({
                "server_id": server_id,
                "request_id": request_id,
                "action": action,
            }),
        ),
        GatewayEvent::OAuthHandoffStarted {
            server_id,
            credential_ref,
        } => (
            EventKind::OAuthHandoffStarted,
            json!({
                "server_id": server_id,
                "credential_ref": credential_ref,
            }),
        ),
        GatewayEvent::OAuthHandoffCompleted {
            server_id,
            credential_ref,
            status,
        } => (
            EventKind::OAuthHandoffCompleted,
            json!({
                "server_id": server_id,
                "credential_ref": credential_ref,
                "status": status,
            }),
        ),
        GatewayEvent::OAuthRefreshFailed {
            server_id,
            credential_ref,
        } => (
            EventKind::OAuthRefreshFailed,
            json!({
                "server_id": server_id,
                "credential_ref": credential_ref,
            }),
        ),
    };
    vault.append_event(id, &Event::new(kind, payload))
}

fn sanitize_gateway_params(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut clean = serde_json::Map::new();
            for (key, value) in map {
                let normalized = key.to_ascii_lowercase();
                if normalized == "progresstoken" || normalized == "progress_token" {
                    clean.insert("progress_ref".to_string(), sanitize_gateway_params(value));
                } else if normalized.contains("secret")
                    || normalized.contains("token")
                    || normalized.contains("password")
                    || normalized.contains("api_key")
                {
                    clean.insert(
                        "redacted_field".to_string(),
                        serde_json::Value::String("[redacted]".to_string()),
                    );
                } else {
                    clean.insert(key.clone(), sanitize_gateway_params(value));
                }
            }
            serde_json::Value::Object(clean)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(sanitize_gateway_params).collect())
        }
        _ => value.clone(),
    }
}

fn observer_emit_gateway(
    observer: &Arc<dyn ConversationObserver>,
    conversation_id: Id,
    event: &GatewayEvent,
) {
    observer.on_event(UiEvent {
        conversation_id: conversation_id.to_string(),
        kind: gateway_event_kind(event).to_string(),
        payload_json: serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string()),
    });
}

fn event_kind(event: &HarnessEvent) -> &'static str {
    match event {
        HarnessEvent::TextDelta { .. } => "text_delta",
        HarnessEvent::ThoughtDelta { .. } => "thought_delta",
        HarnessEvent::ToolCallStarted { .. } => "tool_call_started",
        HarnessEvent::ToolCallProgress { .. } => "tool_call_progress",
        HarnessEvent::FileChanged { .. } => "file_changed",
        HarnessEvent::PermissionRequested { .. } => "permission_requested",
        HarnessEvent::PermissionResolved { .. } => "permission_resolved",
        HarnessEvent::TerminalOutput { .. } => "terminal_output",
        HarnessEvent::PlanUpdated { .. } => "plan_updated",
        HarnessEvent::ModeChanged { .. } => "mode_changed",
        HarnessEvent::Error { .. } => "error",
        HarnessEvent::TurnEnded { .. } => "turn_ended",
    }
}

fn gateway_event_kind(event: &GatewayEvent) -> &'static str {
    match event {
        GatewayEvent::ServerConnected { .. } => "gateway_server_connected",
        GatewayEvent::ToolRouted { .. } => "gateway_tool_routed",
        GatewayEvent::CredentialInjected { .. } => "gateway_credential_injected",
        GatewayEvent::Progress { .. } => "gateway_progress",
        GatewayEvent::Log { .. } => "gateway_log",
        GatewayEvent::Cancellation { .. } => "gateway_cancellation",
        GatewayEvent::DownstreamError { .. } => "gateway_downstream_error",
        GatewayEvent::ElicitationRequested { .. } => "elicitation_requested",
        GatewayEvent::ElicitationResolved { .. } => "elicitation_resolved",
        GatewayEvent::OAuthHandoffStarted { .. } => "oauth_handoff_started",
        GatewayEvent::OAuthHandoffCompleted { .. } => "oauth_handoff_completed",
        GatewayEvent::OAuthRefreshFailed { .. } => "oauth_refresh_failed",
    }
}

fn record_gateway_content_block(blocks: &Mutex<Vec<ContentBlock>>, event: &GatewayEvent) {
    let GatewayEvent::ElicitationRequested {
        request_id,
        server_id,
        origin_tool_call_id,
        mode,
        message,
        schema,
        url,
        ..
    } = event
    else {
        return;
    };
    let Ok(mut blocks) = blocks.lock() else {
        return;
    };
    blocks.push(ContentBlock::ElicitationRequest {
        request_id: request_id.clone(),
        server_id: Some(server_id.clone()),
        origin_tool_call_id: origin_tool_call_id.clone(),
        mode: mode.clone(),
        message: message.clone(),
        schema: schema.clone(),
        url: url.as_ref().map(|value| audit_safe_elicitation_url(value)),
    });
}

fn parse_elicitation_action(action: &str) -> Result<ElicitationAction> {
    match action {
        "accept" => Ok(ElicitationAction::Accept),
        "decline" => Ok(ElicitationAction::Decline),
        "cancel" => Ok(ElicitationAction::Cancel),
        _ => Err(CoreError::Protocol(format!(
            "unknown elicitation action: {action}"
        ))),
    }
}

fn gateway_server_to_dto(
    server: &GatewayServerConfig,
    credentials: &MemoryCredentials,
) -> Result<GatewayServerDto> {
    let credential_refs = server
        .credentials
        .iter()
        .map(|credential| credential.credential_ref.clone())
        .collect::<Vec<_>>();
    let missing_credential_refs = credential_refs
        .iter()
        .filter_map(|credential_ref| match credentials.contains(credential_ref) {
            Ok(true) => None,
            Ok(false) | Err(_) => Some(credential_ref.clone()),
        })
        .collect::<Vec<_>>();
    let (transport, stdio_command, stdio_args, stdio_env, http_endpoint) =
        match &server.transport {
            GatewayTransport::Stdio {
                command,
                args,
                env,
            } => (
                "stdio".to_string(),
                command.clone(),
                args.clone(),
                env.iter()
                    .map(|(name, value)| GatewayEnvVarDto {
                        name: name.clone(),
                        value: value.clone(),
                    })
                    .collect(),
                String::new(),
            ),
            GatewayTransport::StreamableHttp { endpoint, .. } => (
                "streamable_http".to_string(),
                String::new(),
                Vec::new(),
                Vec::new(),
                endpoint.clone(),
            ),
        };
    let oauth_status = server
        .oauth
        .as_ref()
        .map(|oauth| {
            let stored = credentials
                .get_stored(&oauth.token_ref)
                .ok()
                .flatten()
                .and_then(|raw| parse_stored_oauth(&raw).ok());
            let (has_access, expires_at, reauth_required) = stored
                .as_ref()
                .map(|bundle| {
                    (
                        !bundle.access_token.is_empty(),
                        bundle.expires_at,
                        bundle.reauth_required,
                    )
                })
                .unwrap_or((false, None, false));
            oauth_status_label(oauth_connection_status(
                Some(oauth),
                has_access,
                expires_at,
                reauth_required,
            ))
            .to_string()
        })
        .unwrap_or_else(|| "not_configured".to_string());
    let (oauth_token_ref, oauth_client_id, oauth_authorization_endpoint) = server
        .oauth
        .as_ref()
        .map(|oauth| {
            (
                oauth.token_ref.clone(),
                oauth.client_id.clone(),
                oauth
                    .authorization_endpoint
                    .clone()
                    .unwrap_or_default(),
            )
        })
        .unwrap_or_default();
    Ok(GatewayServerDto {
        id: server.id.clone(),
        display_name: server.display_name.clone(),
        enabled: server.enabled,
        scope: serde_json::to_string(&server.scope)?
            .trim_matches('"')
            .to_string(),
        transport,
        stdio_command,
        stdio_args,
        stdio_env,
        http_endpoint,
        credential_refs,
        missing_credential_refs,
        oauth_status,
        oauth_token_ref,
        oauth_client_id,
        oauth_authorization_endpoint,
    })
}

fn gateway_server_from_dto(
    server: &GatewayServerDto,
    existing: Option<&GatewayServerConfig>,
) -> Result<GatewayServerConfig> {
    if server.id.trim().is_empty() {
        return Err(CoreError::Protocol(
            "gateway server id cannot be empty".to_string(),
        ));
    }
    if server.display_name.trim().is_empty() {
        return Err(CoreError::Protocol(
            "gateway server display name cannot be empty".to_string(),
        ));
    }
    let scope = parse_gateway_scope(&server.scope)?;
    let transport = match server.transport.as_str() {
        "stdio" => {
            if server.stdio_command.trim().is_empty() {
                return Err(CoreError::Protocol(
                    "stdio transport requires a command path".to_string(),
                ));
            }
            GatewayTransport::Stdio {
                command: server.stdio_command.clone(),
                args: server.stdio_args.clone(),
                env: server
                    .stdio_env
                    .iter()
                    .map(|entry| (entry.name.clone(), entry.value.clone()))
                    .collect(),
            }
        }
        "streamable_http" => {
            if server.http_endpoint.trim().is_empty() {
                return Err(CoreError::Protocol(
                    "streamable_http transport requires an endpoint URL".to_string(),
                ));
            }
            GatewayTransport::StreamableHttp {
                endpoint: server.http_endpoint.clone(),
                headers: Vec::new(),
            }
        }
        other => {
            return Err(CoreError::Protocol(format!(
                "unknown gateway transport: {other}"
            )));
        }
    };
    Ok(GatewayServerConfig {
        id: server.id.clone(),
        display_name: server.display_name.clone(),
        enabled: server.enabled,
        scope,
        transport,
        timeout_secs: existing.and_then(|existing| existing.timeout_secs),
        credentials: existing
            .map(|existing| existing.credentials.clone())
            .unwrap_or_default(),
        oauth: existing.and_then(|existing| existing.oauth.clone()),
    })
}

fn parse_gateway_scope(scope: &str) -> Result<GatewayScope> {
    match scope {
        "system" => Ok(GatewayScope::System),
        "user" => Ok(GatewayScope::User),
        "project" => Ok(GatewayScope::Project),
        other => Err(CoreError::Protocol(format!(
            "unknown gateway scope: {other}"
        ))),
    }
}

fn gateway_mcp_ref(endpoint: &GatewayEndpoint) -> McpServerRef {
    match gateway_stdio_helper_path() {
        Some(helper) => endpoint.stdio_mcp_ref(helper),
        None => endpoint.mcp_ref(),
    }
}

fn gateway_stdio_helper_path() -> Option<String> {
    if let Ok(path) = std::env::var("TAMTRI_GATEWAY_STDIO_HELPER")
        && std::path::Path::new(&path).is_file()
    {
        return Some(path);
    }
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let binary_name = if cfg!(windows) {
        "tamtri-gateway-stdio.exe"
    } else {
        "tamtri-gateway-stdio"
    };
    let candidates = [
        exe_dir.join(binary_name),
        exe_dir.join("..").join(binary_name),
        std::env::current_dir()
            .ok()?
            .join("target/debug")
            .join(binary_name),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().to_string())
}

fn collect_workdir_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    files: &mut Vec<WorkdirFileDto>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_workdir_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|err| CoreError::Protocol(format!("workdir listing escaped root: {err}")))?;
            let metadata = entry.metadata()?;
            let size = metadata.len();
            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            let mime_type = fs::read(&path)
                .ok()
                .and_then(|bytes| detect_mime(&path, &bytes));
            files.push(WorkdirFileDto {
                relative_path: relative.to_string_lossy().into_owned(),
                size,
                mime_type,
                modified_at,
            });
        }
    }
    Ok(())
}

fn resolve_workdir_relative_path(workdir: &std::path::Path, relative_path: &str) -> Result<PathBuf> {
    let path = std::path::Path::new(relative_path);
    if path.is_absolute() {
        return Err(CoreError::MalformedVault(
            "workdir path must be relative".to_string(),
        ));
    }
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(CoreError::MalformedVault(
                "workdir path must not contain traversal".to_string(),
            ));
        }
    }
    Ok(workdir.join(path))
}

fn safe_workdir_filename(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if cleaned.is_empty() {
        "attachment".to_string()
    } else {
        cleaned
    }
}

fn parse_id(id: &str) -> Result<Id> {
    id.parse()
        .map_err(|err| CoreError::MalformedVault(format!("invalid conversation id: {err}")))
}

fn summary_to_dto(summary: ConversationSummary) -> Result<ConversationSummaryDto> {
    Ok(ConversationSummaryDto {
        id: summary.id.to_string(),
        title: summary.title,
        updated_at: summary.updated_at.to_rfc3339(),
    })
}

fn ffi_err(err: CoreError) -> TamtriError {
    TamtriError::Core {
        message: err.to_string(),
    }
}

fn conversation_to_dto(conversation: &Conversation) -> Result<ConversationDto> {
    let transcript_json = serde_json::to_string(&conversation.messages)?;
    Ok(ConversationDto {
        id: conversation.id.to_string(),
        title: conversation.title.clone(),
        active_harness_id: conversation.active_harness_id.clone(),
        model_id: conversation.model_id.clone(),
        transcript_json,
    })
}
