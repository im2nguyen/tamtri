use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::oneshot;
use typeshare::typeshare;
use url::Url;
use uuid::Uuid;

use crate::artifact::{
    ArtifactSnapshot, ArtifactSnapshotter, detect_mime, list_renderable_workdir_paths,
    new_renderable_workdir_paths, verify_attachment, verify_inline_artifact,
};
use crate::config::{
    GatewayScope, GatewayServerConfig, GatewayTransport, OAuthConfig, add_agent_to_roster,
    load_app_config, replace_gateway_servers, save_app_config, seed_agent_roster_if_empty,
    set_agent_enabled,
};
use crate::conversation::reduce::TurnReducer;
use crate::conversation::{
    ContentBlock, Conversation, ConversationKind, ElicitationAction, Id, McpServerRef, Message,
    Role, Root, RootKind, RootOrigin, RootScope, WorkingDir, attach_root, remove_root,
    validate_root,
};
use crate::credentials::DurableCredentials;
use crate::debug_log::debug_log;
use crate::diagnostics;
use crate::harness::acp::AgentLaunchSpec;
use crate::harness::health::{
    HarnessHealthStatus, adapter_kind_label, adapter_type_label, health_entries_from_roster,
    it_admin_checklist,
};
use crate::harness::readiness::{
    ReadinessState, RecommendableAgent, diagnose_agent, recommend_agent,
};
use crate::harness::registry::build_adapter;
use crate::harness::usage::{HarnessUsageEntry, list_harness_usage};
use crate::harness::{
    ContextSeed, ConversationContext, HarnessAdapter, HarnessEvent, RunControl, TurnEndReason,
    TurnInput,
};
use crate::mcp::app::{AppTemplate, app_bridge_bootstrap_script, app_sandbox_csp};
use crate::mcp::app_bridge::{
    AppBridgeBeginResult, AppBridgeResolution, SharedAppBridgeCoordinator, execute_action,
    finish_execution, parse_app_bridge_rpc, shared_app_bridge_coordinator,
};
use crate::mcp::capabilities::{FeatureStatus, ServerCapabilityReport};
use crate::mcp::elicitation::{
    audit_safe_elicitation_url, elicitation_request_block, elicitation_response_block,
};
use crate::mcp::endpoint::{GatewayEndpoint, start_loopback_gateway};
use crate::mcp::gateway::{GatewayEvent, McpGateway};
use crate::mcp::oauth::{
    PkceChallenge, build_authorization_url, exchange_authorization_code, generate_pkce,
    oauth_connection_status, oauth_status_label, parse_stored_oauth, serialize_stored_oauth,
    stored_oauth_from_token_response, validate_callback_url,
};
use crate::mcp::protocol::CallToolResult;
use crate::mcp::url_handoff::validate_handoff_url;
use crate::orchestration::events::{orchestration_finished, orchestration_started};
use crate::orchestration::mcp_tools::{
    TOOL_ORCHESTRATION_CANCEL, TOOL_ORCHESTRATION_HANDOFF, TOOL_ORCHESTRATION_RUN,
    TOOL_ORCHESTRATION_STATUS, native_original_name, tool_result_error, tool_result_structured,
};
use crate::orchestration::{
    self, OrchestrationRunDto, OrchestrationRunMeta, OrchestrationRunStatus, RecipeSummary,
};
use crate::project::{Project, ProjectStore, effective_roots, unfiled_project_id};
use crate::search::{SEARCH_SCOPE_MESSAGE, SearchMatchField, search_conversations};
use crate::vault::bundle::{export_conversation_bundle_with_roots, import_bundle_or_folder_as_new};
use crate::vault::events::{Event, EventKind};
use crate::vault::fs::{FilesystemVault, copy_attachments_dir};
use crate::vault::{ConversationSummary, ConversationVault, VaultIssue};
use crate::{CoreError, Result};

pub trait ConversationObserver: Send + Sync {
    fn on_event(&self, event: UiEvent);
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiEvent {
    pub conversation_id: String,
    pub kind: String,
    pub payload_json: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationSummaryDto {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub active_harness_id: Option<String>,
    pub kind: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationDto {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub active_harness_id: Option<String>,
    pub model_id: Option<String>,
    pub forked_from: Option<String>,
    pub kind: String,
    pub transcript_json: String,
}

#[derive(Debug, Clone)]
struct CachedConversationDto {
    updated_at: DateTime<Utc>,
    dto: ConversationDto,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeLoadDto {
    pub recipe_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultIssueDto {
    pub kind: String,
    pub conversation_id: Option<String>,
    pub path: Option<String>,
    pub reason: Option<String>,
    pub winner_path: Option<String>,
    pub loser_paths: Vec<String>,
    pub detail: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootDto {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub kind: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub roots: Vec<RootDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkdirFileDto {
    pub relative_path: String,
    pub size: u64,
    pub mime_type: Option<String>,
    pub modified_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkdirFileContentDto {
    pub mime_type: Option<String>,
    pub data: Vec<u8>,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayEnvVarDto {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessHealthEntryDto {
    pub id: String,
    pub display_name: String,
    pub command: String,
    pub status: String,
    pub install_doc_url: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessProviderEntryDto {
    pub id: String,
    pub display_name: String,
    pub command: String,
    pub status: String,
    pub readiness_state: String,
    pub recovery_action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_message: Option<String>,
    pub install_doc_url: String,
    pub adapter_type: String,
    pub adapter_kind: String,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_count: Option<u32>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picker_order: Option<u32>,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessInstalledCliDto {
    pub id: String,
    pub display_name: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub install_doc_url: String,
    pub in_roster: bool,
    pub auth_ready: bool,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessPickerSettingsDto {
    pub harness_order: Vec<String>,
    pub hidden_harness_ids: Vec<String>,
    pub enable_cli_update_checks: bool,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessReadinessRecommendDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub display_name: String,
    pub readiness_state: String,
    pub recovery_action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub install_doc_url: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessUsageWindowDto {
    pub id: String,
    pub label: String,
    pub utilization_pct: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    pub tone: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessUsageBalanceDto {
    pub id: String,
    pub label: String,
    pub remaining: f64,
    pub unit: String,
    pub tone: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessUsageEntryDto {
    pub provider_id: String,
    pub display_name: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_label: Option<String>,
    #[serde(default)]
    pub windows: Vec<HarnessUsageWindowDto>,
    #[serde(default)]
    pub balances: Vec<HarnessUsageBalanceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub fetched_at: String,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessUsageListDto {
    pub providers: Vec<HarnessUsageEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHitDto {
    pub conversation_id: String,
    pub title: String,
    pub snippet: String,
    pub match_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportWarningDto {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportResultDto {
    pub conversation: ConversationDto,
    pub warnings: Vec<ImportWarningDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRosterEntryDto {
    pub id: String,
    pub display_name: String,
    pub runtime_model_switch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfoDto {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayToolDto {
    pub exposed_name: String,
    pub server_id: String,
    pub original_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewaySettingsDto {
    pub default_call_timeout_secs: u64,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub oauth_token_endpoint: String,
    pub oauth_scopes: Vec<String>,
    pub cap_tools: String,
    pub cap_resources: String,
    pub cap_prompts: String,
    pub cap_elicitation: String,
    pub cap_apps: String,
    pub cap_tasks: String,
    pub cap_roots: String,
    pub cap_sampling: String,
    pub connection_status: String,
    pub last_error: String,
    pub timeout_secs: Option<crate::protocol::WireU64>,
}

#[derive(Debug, Clone)]
struct GatewayServerStatus {
    connection_status: String,
    last_error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthHandoffDto {
    pub server_id: String,
    pub authorization_url: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthCompletionDto {
    pub server_id: String,
    pub oauth_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppTemplateDto {
    pub template_ref: String,
    pub server_id: String,
    pub html: String,
    pub allowed_origins: Vec<String>,
    pub metadata_json: String,
    pub bridge_script: String,
    pub content_security_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppBridgeSubmissionDto {
    pub request_id: String,
    pub needs_consent: bool,
}

type FfiResult<T> = std::result::Result<T, TamtriError>;

#[derive(Debug, thiserror::Error)]
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

type TurnWaiters = Arc<Mutex<HashMap<Id, Vec<oneshot::Sender<TurnEndReason>>>>>;

struct OrchestrationRunHandle {
    cancelled: Arc<AtomicBool>,
}

type OrchestrationRunRegistry = Arc<Mutex<HashMap<String, OrchestrationRunHandle>>>;
type PendingOrchestrationConsents = Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>;

static SHARED_CORE: OnceLock<Arc<TamtriCore>> = OnceLock::new();

const ORCHESTRATION_CONSENT_TIMEOUT_SECS: u64 = 600;

pub struct TamtriCore {
    vault: Arc<FilesystemVault>,
    projects: Arc<ProjectStore>,
    runtime: Runtime,
    adapters: Arc<Mutex<HashMap<String, Arc<dyn HarnessAdapter>>>>,
    active_runs: Arc<Mutex<HashMap<Id, ActiveRun>>>,
    credentials: Arc<DurableCredentials>,
    observer: Arc<dyn ConversationObserver>,
    conversation_cache: Arc<Mutex<HashMap<Id, CachedConversationDto>>>,
    pending_oauth: Arc<Mutex<HashMap<String, PendingOAuthFlow>>>,
    gateway_capability_cache: Arc<Mutex<HashMap<String, ServerCapabilityReport>>>,
    gateway_status_cache: Arc<Mutex<HashMap<String, GatewayServerStatus>>>,
    app_bridge: SharedAppBridgeCoordinator,
    /// Shell-resolved root URIs (security-scoped bookmarks) keyed by conversation id.
    runtime_roots: Arc<Mutex<HashMap<Id, Vec<Root>>>>,
    turn_waiters: TurnWaiters,
    active_orchestration_runs: OrchestrationRunRegistry,
    pending_orchestration_consents: PendingOrchestrationConsents,
    tamtri_home: PathBuf,
    server_id: String,
}

const CONVERSATION_CACHE_LIMIT: usize = 32;

impl TamtriCore {
    pub fn new(vault_path: String, observer: Arc<dyn ConversationObserver>) -> FfiResult<Self> {
        Self::new_inner(vault_path.into(), observer).map_err(ffi_err)
    }
}

impl TamtriCore {
    pub fn new_inner(vault_path: PathBuf, observer: Arc<dyn ConversationObserver>) -> Result<Self> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        seed_agent_roster_if_empty(&vault_path)?;
        crate::harness::discovery::sync_agent_roster_with_discovery(&vault_path)?;
        let vault = Arc::new(FilesystemVault::new(vault_path.clone())?);
        let projects = Arc::new(ProjectStore::new(&vault_path)?);
        let config = load_app_config(&vault_path)?;
        let mut adapters: HashMap<String, Arc<dyn HarnessAdapter>> = HashMap::new();
        for spec in &config.agent_roster {
            adapters.insert(spec.id.clone(), build_adapter(spec));
        }
        let tamtri_home = tamtri_home_for_vault(&vault_path);
        let credentials = Arc::new(DurableCredentials::open(&tamtri_home)?);
        let server_id = crate::relay::load_or_create_server_id(&tamtri_home)?;
        Ok(Self {
            vault,
            projects,
            runtime,
            adapters: Arc::new(Mutex::new(adapters)),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            credentials,
            observer,
            conversation_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_oauth: Arc::new(Mutex::new(HashMap::new())),
            gateway_capability_cache: Arc::new(Mutex::new(HashMap::new())),
            gateway_status_cache: Arc::new(Mutex::new(HashMap::new())),
            app_bridge: shared_app_bridge_coordinator(),
            runtime_roots: Arc::new(Mutex::new(HashMap::new())),
            turn_waiters: Arc::new(Mutex::new(HashMap::new())),
            active_orchestration_runs: Arc::new(Mutex::new(HashMap::new())),
            pending_orchestration_consents: Arc::new(Mutex::new(HashMap::new())),
            tamtri_home,
            server_id,
        })
    }

    pub fn list_native_sessions(
        &self,
    ) -> FfiResult<Vec<crate::harness::sessions::NativeSessionSummary>> {
        Ok(crate::harness::sessions::list_native_sessions())
    }

    pub fn import_native_session(
        &self,
        provider: String,
        path: String,
        harness_id: String,
        model_id: String,
    ) -> FfiResult<ConversationDto> {
        self.import_native_session_inner(&provider, &path, &harness_id, &model_id)
            .map_err(ffi_err)
    }

    pub fn list_recipes(&self) -> FfiResult<Vec<RecipeSummary>> {
        self.list_recipes_inner().map_err(ffi_err)
    }

    pub fn load_recipe(&self, recipe_id: String) -> FfiResult<RecipeLoadDto> {
        self.load_recipe_inner(&recipe_id).map_err(ffi_err)
    }

    pub fn orchestration_run(
        &self,
        recipe_id: String,
        source_conversation_id: String,
        inputs_json: Option<String>,
    ) -> FfiResult<OrchestrationRunDto> {
        self.orchestration_run_inner(&recipe_id, &source_conversation_id, inputs_json.as_deref())
            .map_err(ffi_err)
    }

    pub fn orchestration_status(&self, run_id: String) -> FfiResult<OrchestrationRunDto> {
        self.orchestration_status_inner(&run_id).map_err(ffi_err)
    }

    pub fn orchestration_cancel(&self, run_id: String) -> FfiResult<OrchestrationRunDto> {
        self.orchestration_cancel_inner(&run_id).map_err(ffi_err)
    }

    fn import_native_session_inner(
        &self,
        provider: &str,
        path: &str,
        harness_id: &str,
        model_id: &str,
    ) -> Result<ConversationDto> {
        let conversation =
            crate::harness::sessions::import_native_session(provider, path, harness_id, model_id)?;
        self.vault.create(&conversation)?;
        let dto = conversation_to_dto(&conversation)?;
        self.store_conversation_cache(conversation.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn relay_pairing_offer(&self) -> FfiResult<crate::relay::ConnectionOffer> {
        self.relay_pairing_offer_inner().map_err(ffi_err)
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    fn relay_pairing_offer_inner(&self) -> Result<crate::relay::ConnectionOffer> {
        let keypair = crate::relay::load_or_create_keypair(&self.tamtri_home)?;
        Ok(crate::relay::build_pairing_offer(&self.server_id, &keypair))
    }

    fn record_gateway_server_status(
        &self,
        server_id: &str,
        connection_status: &str,
        last_error: &str,
    ) {
        if let Ok(mut cache) = self.gateway_status_cache.lock() {
            cache.insert(
                server_id.to_string(),
                GatewayServerStatus {
                    connection_status: connection_status.to_string(),
                    last_error: last_error.to_string(),
                },
            );
        }
    }

    fn gateway_server_status(&self, server_id: &str) -> GatewayServerStatus {
        self.gateway_status_cache
            .lock()
            .ok()
            .and_then(|cache| cache.get(server_id).cloned())
            .unwrap_or(GatewayServerStatus {
                connection_status: "unknown".to_string(),
                last_error: String::new(),
            })
    }

    fn drain_gateway_events_to_vault(
        &self,
        gateway_event_rx: &mut tokio::sync::mpsc::UnboundedReceiver<GatewayEvent>,
    ) -> Result<()> {
        while let Ok(event) = gateway_event_rx.try_recv() {
            append_vault_event_for_gateway_event(&self.vault, &event)?;
        }
        Ok(())
    }

    fn list_acp_agents_inner(&self) -> Result<Vec<AgentRosterEntryDto>> {
        let config = load_app_config(self.vault.root())?;
        let enabled_ids: HashSet<&str> = config
            .agent_roster
            .iter()
            .filter(|spec| spec.enabled)
            .map(|spec| spec.id.as_str())
            .collect();
        let mut agents = self
            .adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?
            .iter()
            .filter(|(id, _)| enabled_ids.contains(id.as_str()))
            .map(|(id, adapter)| AgentRosterEntryDto {
                id: id.clone(),
                display_name: adapter.display_name().to_string(),
                runtime_model_switch: adapter.capabilities().runtime_model_switch,
            })
            .collect::<Vec<_>>();
        agents.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        Ok(agents)
    }

    fn list_acp_agent_models_inner(&self, agent_id: &str) -> Result<Vec<ModelInfoDto>> {
        let adapters = self
            .adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?;
        let adapter = adapters
            .get(agent_id)
            .ok_or_else(|| CoreError::Protocol(format!("unknown ACP agent: {agent_id}")))?;
        let models = self
            .runtime
            .block_on(adapter.available_models())?
            .into_iter()
            .map(|model| ModelInfoDto {
                id: model.id,
                display_name: model.display_name,
            })
            .collect();
        Ok(models)
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
        let adapter = build_adapter(&spec);
        self.adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?
            .insert(spec.id, adapter);
        Ok(())
    }
}

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
            adapter: Default::default(),
            enabled: true,
        })
        .map_err(ffi_err)
    }

    pub fn register_acp_agent_with_env(
        &self,
        id: String,
        display_name: String,
        command: String,
        args: Vec<String>,
        env: Vec<GatewayEnvVarDto>,
    ) -> FfiResult<()> {
        self.register_acp_agent_spec(AgentLaunchSpec {
            id,
            display_name,
            command,
            args,
            env: env
                .into_iter()
                .map(|pair| (pair.name, pair.value))
                .collect(),
            adapter: Default::default(),
            enabled: true,
        })
        .map_err(ffi_err)
    }

    pub fn list_acp_agents(&self) -> FfiResult<Vec<AgentRosterEntryDto>> {
        self.list_acp_agents_inner().map_err(ffi_err)
    }

    pub fn list_acp_agent_models(&self, agent_id: String) -> FfiResult<Vec<ModelInfoDto>> {
        self.list_acp_agent_models_inner(&agent_id).map_err(ffi_err)
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

    pub fn list_projects(&self) -> FfiResult<Vec<ProjectDto>> {
        self.list_projects_inner().map_err(ffi_err)
    }

    pub fn create_project(&self, name: String) -> FfiResult<ProjectDto> {
        self.create_project_inner(&name).map_err(ffi_err)
    }

    pub fn update_project(&self, id: String, name: String) -> FfiResult<ProjectDto> {
        self.update_project_inner(&id, &name).map_err(ffi_err)
    }

    pub fn delete_project(&self, id: String) -> FfiResult<()> {
        self.delete_project_inner(&id).map_err(ffi_err)
    }

    pub fn attach_project_root(
        &self,
        project_id: String,
        name: String,
        uri: String,
        kind: String,
        scope: String,
    ) -> FfiResult<RootDto> {
        self.attach_project_root_inner(&project_id, &name, &uri, &kind, &scope)
            .map_err(ffi_err)
    }

    pub fn remove_project_root(&self, project_id: String, root_id: String) -> FfiResult<()> {
        self.remove_project_root_inner(&project_id, &root_id)
            .map_err(ffi_err)
    }

    pub fn move_conversation_to_project(
        &self,
        conversation_id: String,
        project_id: String,
    ) -> FfiResult<ConversationDto> {
        self.move_conversation_to_project_inner(&conversation_id, &project_id)
            .map_err(ffi_err)
    }

    pub fn create_conversation_in_project(
        &self,
        project_id: String,
        title: String,
        harness_id: String,
        model_id: String,
    ) -> FfiResult<ConversationDto> {
        self.create_conversation_in_project_inner(&project_id, &title, &harness_id, &model_id)
            .map_err(ffi_err)
    }

    pub fn set_conversation_model(
        &self,
        id: String,
        model_id: String,
    ) -> FfiResult<ConversationDto> {
        self.set_conversation_model_inner(&id, &model_id)
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

    pub fn prepare_for_app_quit(&self) -> FfiResult<()> {
        self.prepare_for_app_quit_inner().map_err(ffi_err)
    }

    pub fn cancel_task(&self, conversation_id: String, task_id: String) -> FfiResult<()> {
        self.cancel_task_inner(&conversation_id, &task_id)
            .map_err(ffi_err)
    }

    pub fn list_gateway_servers(&self) -> FfiResult<Vec<GatewayServerDto>> {
        self.list_gateway_servers_inner().map_err(ffi_err)
    }

    pub fn refresh_gateway_capabilities(&self) -> FfiResult<Vec<GatewayServerDto>> {
        self.refresh_gateway_capabilities_inner().map_err(ffi_err)
    }

    pub fn list_gateway_tools(&self) -> FfiResult<Vec<GatewayToolDto>> {
        self.list_gateway_tools_inner().map_err(ffi_err)
    }

    pub fn get_gateway_settings(&self) -> FfiResult<GatewaySettingsDto> {
        self.get_gateway_settings_inner().map_err(ffi_err)
    }

    pub fn set_gateway_default_timeout(&self, default_call_timeout_secs: u64) -> FfiResult<()> {
        self.set_gateway_default_timeout_inner(default_call_timeout_secs)
            .map_err(ffi_err)
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

    pub fn complete_oauth_callback(&self, callback_url: String) -> FfiResult<OAuthCompletionDto> {
        self.complete_oauth_callback_inner(&callback_url)
            .map_err(ffi_err)
    }

    pub fn export_gateway_credential(&self, credential_ref: String) -> FfiResult<Option<String>> {
        self.credentials
            .get_stored(&credential_ref)
            .map_err(ffi_err)
    }

    pub fn list_roots(&self, conversation_id: String) -> FfiResult<Vec<RootDto>> {
        self.list_roots_inner(&conversation_id).map_err(ffi_err)
    }

    pub fn attach_root(
        &self,
        conversation_id: String,
        name: String,
        uri: String,
        kind: String,
        scope: String,
    ) -> FfiResult<RootDto> {
        self.attach_root_inner(&conversation_id, &name, &uri, &kind, &scope)
            .map_err(ffi_err)
    }

    pub fn remove_root(&self, conversation_id: String, root_id: String) -> FfiResult<()> {
        self.remove_root_inner(&conversation_id, &root_id)
            .map_err(ffi_err)
    }

    pub fn sync_runtime_roots(
        &self,
        conversation_id: String,
        roots: Vec<RootDto>,
    ) -> FfiResult<()> {
        self.sync_runtime_roots_inner(&conversation_id, roots)
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

    pub fn write_workdir_file(
        &self,
        conversation_id: String,
        filename: String,
        data: Vec<u8>,
    ) -> FfiResult<String> {
        self.write_workdir_file_inner(&conversation_id, &filename, &data)
            .map_err(ffi_err)
    }

    pub fn list_workdir_files(&self, conversation_id: String) -> FfiResult<Vec<WorkdirFileDto>> {
        self.list_workdir_files_inner(&conversation_id)
            .map_err(ffi_err)
    }

    pub fn conversation_workdir_path(&self, conversation_id: String) -> FfiResult<String> {
        self.conversation_workdir_path_inner(&conversation_id)
            .map_err(ffi_err)
    }

    pub fn conversation_folder_path(&self, conversation_id: String) -> FfiResult<String> {
        self.conversation_folder_path_inner(&conversation_id)
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

    pub fn resolve_artifact_path(
        &self,
        conversation_id: String,
        path: String,
    ) -> FfiResult<String> {
        self.resolve_artifact_path_inner(&conversation_id, &path)
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
                &Event::new(EventKind::ArtifactNavigationBlocked, json!({ "url": url })),
            )
            .map_err(ffi_err)
    }

    pub fn resolve_app_template(
        &self,
        conversation_id: String,
        server_id: String,
        template_ref: String,
    ) -> FfiResult<Option<AppTemplateDto>> {
        self.resolve_app_template_inner(&conversation_id, &server_id, &template_ref)
            .map_err(ffi_err)
    }

    pub fn submit_app_bridge_request(
        &self,
        conversation_id: String,
        server_id: String,
        app_id: String,
        template_ref: String,
        request_json: String,
    ) -> FfiResult<AppBridgeSubmissionDto> {
        self.submit_app_bridge_request_inner(
            &conversation_id,
            &server_id,
            &app_id,
            &template_ref,
            &request_json,
        )
        .map_err(ffi_err)
    }

    pub fn respond_app_bridge_consent(
        &self,
        conversation_id: String,
        request_id: String,
        option_id: String,
    ) -> FfiResult<()> {
        self.respond_app_bridge_consent_inner(&conversation_id, &request_id, &option_id)
            .map_err(ffi_err)
    }

    pub fn log_app_navigation_blocked(
        &self,
        conversation_id: String,
        server_id: String,
        template_ref: String,
        url: String,
    ) -> FfiResult<()> {
        let id = parse_id(&conversation_id)?;
        self.vault
            .append_event(
                id,
                &Event::new(
                    EventKind::AppNavigationBlocked,
                    json!({
                        "server_id": server_id,
                        "template_ref": template_ref,
                        "url": url,
                    }),
                ),
            )
            .map_err(ffi_err)
    }

    pub fn app_bridge_bootstrap_script(&self) -> String {
        app_bridge_bootstrap_script("tamtriAppBridge")
    }

    pub fn export_conversation_bundle(
        &self,
        conversation_id: String,
        dest_path: String,
    ) -> FfiResult<()> {
        let id = parse_id(&conversation_id)?;
        let conversation = self.vault.load(id)?;
        let roots = self.effective_roots_for_conversation(&conversation)?;
        export_conversation_bundle_with_roots(
            &self.vault,
            &conversation,
            roots,
            PathBuf::from(dest_path).as_path(),
        )
        .map_err(ffi_err)
    }

    pub fn import_bundle_or_folder_as_new(
        &self,
        source_path: String,
    ) -> FfiResult<ImportResultDto> {
        self.import_bundle_or_folder_as_new_inner(source_path.into())
            .map_err(ffi_err)
    }

    pub fn search_conversations(&self, query: String) -> FfiResult<Vec<SearchHitDto>> {
        self.search_conversations_inner(&query).map_err(ffi_err)
    }

    pub fn search_scope_message(&self) -> String {
        SEARCH_SCOPE_MESSAGE.to_string()
    }

    pub fn list_harness_health(&self) -> FfiResult<Vec<HarnessHealthEntryDto>> {
        self.list_harness_health_inner().map_err(ffi_err)
    }

    pub fn list_harness_providers(&self) -> FfiResult<Vec<HarnessProviderEntryDto>> {
        self.list_harness_providers_inner().map_err(ffi_err)
    }

    pub fn harness_roster_set_enabled(&self, agent_id: String, enabled: bool) -> FfiResult<()> {
        self.harness_roster_set_enabled_inner(&agent_id, enabled)
            .map_err(ffi_err)
    }

    pub fn harness_roster_add(&self, spec: AgentLaunchSpec) -> FfiResult<()> {
        self.harness_roster_add_inner(spec).map_err(ffi_err)
    }

    pub fn list_harness_usage(&self) -> FfiResult<HarnessUsageListDto> {
        self.list_harness_usage_inner().map_err(ffi_err)
    }

    pub fn list_harness_installed_clis(&self) -> FfiResult<Vec<HarnessInstalledCliDto>> {
        self.list_harness_installed_clis_inner().map_err(ffi_err)
    }

    pub fn harness_picker_settings_get(&self) -> FfiResult<HarnessPickerSettingsDto> {
        self.harness_picker_settings_get_inner().map_err(ffi_err)
    }

    pub fn harness_picker_settings_set(
        &self,
        harness_order: Vec<String>,
        hidden_harness_ids: Vec<String>,
        enable_cli_update_checks: bool,
    ) -> FfiResult<()> {
        self.harness_picker_settings_set_inner(
            harness_order,
            hidden_harness_ids,
            enable_cli_update_checks,
        )
        .map_err(ffi_err)
    }

    pub fn harness_discovery_sync(&self) -> FfiResult<()> {
        self.harness_discovery_sync_inner().map_err(ffi_err)
    }

    pub fn harness_health_checklist(&self) -> FfiResult<String> {
        self.harness_health_checklist_inner().map_err(ffi_err)
    }

    pub fn vault_issues(&self) -> FfiResult<Vec<VaultIssueDto>> {
        self.vault_issues_inner().map_err(ffi_err)
    }

    pub fn vault_path(&self) -> String {
        self.vault.root().display().to_string()
    }

    pub fn write_diagnostics_bundle(
        &self,
        dest_path: String,
        system_info_json: String,
    ) -> FfiResult<String> {
        self.write_diagnostics_bundle_inner(dest_path.into(), &system_info_json)
            .map_err(ffi_err)
    }
}

impl TamtriCore {
    pub fn load_conversation_inner(&self, id: &str) -> Result<ConversationDto> {
        let id = parse_id(id)?;
        let conversation = self.vault.load(id)?;
        if let Ok(cache) = self.conversation_cache.lock()
            && let Some(cached) = cache.get(&id)
            && cached.updated_at == conversation.updated_at
        {
            debug_log(format!("[tamtri] load_conversation cache hit {id}"));
            return Ok(cached.dto.clone());
        }

        let started = std::time::Instant::now();
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
        let source = self.vault.load(id)?;
        if source.kind == ConversationKind::Example {
            return Err(CoreError::ExampleImmutable(id));
        }
        let mut fork = source.fork();
        fork.active_harness_id = Some(harness_id.to_string());
        fork.model_id = Some(model_id.to_string());
        self.vault.create(&fork)?;
        let dto = conversation_to_dto(&fork)?;
        self.store_conversation_cache(fork.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn list_projects_inner(&self) -> Result<Vec<ProjectDto>> {
        Ok(self.projects.list()?.iter().map(project_to_dto).collect())
    }

    fn effective_roots_for_conversation(&self, conversation: &Conversation) -> Result<Vec<Root>> {
        let Some(project_id) = conversation.project_id else {
            return Ok(conversation.roots.clone());
        };
        let project = match self.projects.load(project_id) {
            Ok(project) => project,
            Err(CoreError::ProjectNotFound(_)) => return Ok(conversation.roots.clone()),
            Err(err) => return Err(err),
        };
        Ok(effective_roots(&project.roots, &conversation.roots))
    }

    pub fn create_project_inner(&self, name: &str) -> Result<ProjectDto> {
        let project = Project::new(name)?;
        self.projects.create(&project)?;
        Ok(project_to_dto(&project))
    }

    pub fn update_project_inner(&self, id: &str, name: &str) -> Result<ProjectDto> {
        let project = self.projects.update_name(parse_id(id)?, name.to_string())?;
        Ok(project_to_dto(&project))
    }

    pub fn delete_project_inner(&self, id: &str) -> Result<()> {
        let project_id = parse_id(id)?;
        let project = self.projects.load(project_id)?;
        if project_id == unfiled_project_id() {
            return Err(CoreError::UnfiledProjectImmutable);
        }
        let project_roots = project.roots.clone();
        for summary in self.vault.list()? {
            if summary.project_id != Some(project_id) {
                continue;
            }
            let mut conversation = self.vault.load(summary.id)?;
            let merged = effective_roots(&project_roots, &conversation.roots)
                .into_iter()
                .map(|mut root| {
                    if root.origin == RootOrigin::Project {
                        root.origin = RootOrigin::ProjectSnapshot;
                    }
                    root
                })
                .collect();
            conversation.roots = merged;
            conversation.project_id = None;
            conversation.touch();
            self.vault.save_meta(&conversation)?;
            self.invalidate_conversation_cache(conversation.id);
        }
        self.projects.delete(project_id)
    }

    pub fn attach_project_root_inner(
        &self,
        project_id: &str,
        name: &str,
        uri: &str,
        kind: &str,
        scope: &str,
    ) -> Result<RootDto> {
        let root = self.projects.attach_root(
            parse_id(project_id)?,
            name.to_string(),
            uri.to_string(),
            parse_root_kind(kind)?,
            parse_root_scope(scope)?,
        )?;
        Ok(root_to_dto(&root))
    }

    pub fn remove_project_root_inner(&self, project_id: &str, root_id: &str) -> Result<()> {
        self.projects.remove_root(parse_id(project_id)?, root_id)?;
        Ok(())
    }

    pub fn move_conversation_to_project_inner(
        &self,
        conversation_id: &str,
        project_id: &str,
    ) -> Result<ConversationDto> {
        let conversation_id = parse_id(conversation_id)?;
        let project_id = parse_id(project_id)?;
        self.projects.load(project_id)?;
        let mut conversation = self.vault.load(conversation_id)?;
        conversation.project_id = (project_id != unfiled_project_id()).then_some(project_id);
        conversation.touch();
        self.vault.save_meta(&conversation)?;
        self.invalidate_conversation_cache(conversation_id);
        let dto = conversation_to_dto(&conversation)?;
        self.store_conversation_cache(conversation.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn create_conversation_in_project_inner(
        &self,
        project_id: &str,
        title: &str,
        harness_id: &str,
        model_id: &str,
    ) -> Result<ConversationDto> {
        let project_id = parse_id(project_id)?;
        self.projects.load(project_id)?;
        let mut conversation = Conversation::new(title);
        conversation.project_id = (project_id != unfiled_project_id()).then_some(project_id);
        conversation.active_harness_id = Some(harness_id.to_string());
        conversation.model_id = Some(model_id.to_string());
        self.vault.create(&conversation)?;
        let dto = conversation_to_dto(&conversation)?;
        self.store_conversation_cache(conversation.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn set_conversation_model_inner(
        &self,
        id: &str,
        model_id: &str,
    ) -> Result<ConversationDto> {
        let id = parse_id(id)?;
        let trimmed = model_id.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Protocol("model_id required".to_string()));
        }

        let mut conversation = self.vault.load(id)?;
        if conversation.kind == ConversationKind::Example {
            return Err(CoreError::ExampleImmutable(id));
        }
        if self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .contains_key(&id)
        {
            return Err(CoreError::ConversationBusy(id));
        }

        let harness_id = conversation
            .active_harness_id
            .clone()
            .ok_or_else(|| CoreError::Protocol("conversation has no active harness".to_string()))?;
        let adapter = self.adapter(&harness_id)?;
        if !adapter.capabilities().runtime_model_switch {
            return Err(CoreError::Protocol(format!(
                "harness {harness_id} does not support runtime model switching"
            )));
        }

        conversation.model_id = Some(trimmed.to_string());
        conversation.touch();
        self.vault.save_meta(&conversation)?;
        self.invalidate_conversation_cache(id);
        let dto = conversation_to_dto(&conversation)?;
        self.store_conversation_cache(conversation.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn copy_example_conversation(&self, id: String) -> FfiResult<ConversationDto> {
        self.copy_example_conversation_inner(&id).map_err(ffi_err)
    }

    pub fn copy_example_conversation_inner(&self, id: &str) -> Result<ConversationDto> {
        let id = parse_id(id)?;
        let source = self.vault.load(id)?;
        if source.kind != ConversationKind::Example {
            return Err(CoreError::Protocol(
                "conversation is not an example".to_string(),
            ));
        }
        let mut copy = source.fork();
        copy.kind = ConversationKind::User;
        copy.title = source
            .title
            .strip_prefix("Example: ")
            .unwrap_or(&source.title)
            .to_string();
        copy.active_harness_id = None;
        copy.model_id = None;
        let source_dir = self.vault.conversation_folder(id)?;
        self.vault.create(&copy)?;
        let copy_dir = self.vault.conversation_folder(copy.id)?;
        copy_attachments_dir(
            &source_dir.join("attachments"),
            &copy_dir.join("attachments"),
        )?;
        let dto = conversation_to_dto(&copy)?;
        self.store_conversation_cache(copy.updated_at, dto.clone());
        Ok(dto)
    }

    pub fn harness_readiness_recommend(&self) -> FfiResult<HarnessReadinessRecommendDto> {
        self.harness_readiness_recommend_inner().map_err(ffi_err)
    }

    pub fn send_message_inner(&self, conversation_id: &str, text: &str) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let conversation = self.vault.load(id)?;
        if conversation.kind == ConversationKind::Example {
            return Err(CoreError::ExampleImmutable(id));
        }
        if self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .contains_key(&id)
        {
            return Err(CoreError::ConversationBusy(id));
        }

        let mut conversation = conversation;
        let harness_id = conversation
            .active_harness_id
            .clone()
            .ok_or_else(|| CoreError::Protocol("conversation has no active harness".to_string()))?;
        let model_id = conversation
            .model_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let effective_roots = self.effective_roots_for_conversation(&conversation)?;
        let adapter = self.adapter(&harness_id)?;
        let harness_display_name = adapter.display_name().to_string();
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
        self.runtime
            .block_on(gateway.set_agent_context(id.to_string(), true));
        self.runtime.block_on(gateway.set_roots(roots_for_gateway(
            &self.runtime_roots,
            id,
            &effective_roots,
        )));
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
        let tool_catalog = if adapter.capabilities().native_tools {
            self.runtime
                .block_on(gateway.list_tools())
                .map(crate::harness::tools::from_gateway_tools)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let ctx = ConversationContext {
            seed: ContextSeed::FreshTranscript {
                messages: conversation.messages.clone(),
            },
            working_dir: WorkingDir::VaultLocal,
            working_dir_path: workdir_path.clone(),
            roots: effective_roots,
            mcp_servers: vec![gateway_mcp_ref(&gateway_endpoint)],
            model_id,
            native_session: conversation.native_session.clone(),
            tool_catalog,
        };

        self.vault.append_event(
            id,
            &Event::new(EventKind::TurnStarted, json!({ "harness_id": harness_id })),
        )?;
        self.observer.on_event(UiEvent {
            conversation_id: id.to_string(),
            kind: "turn_started".to_string(),
            payload_json: json!({ "harness_id": harness_id }).to_string(),
        });

        let vault = Arc::clone(&self.vault);
        let active_runs = Arc::clone(&self.active_runs);
        let observer = Arc::clone(&self.observer);
        let conversation_cache = Arc::clone(&self.conversation_cache);
        let gateway_blocks = Arc::new(Mutex::new(Vec::<ContentBlock>::new()));
        let gateway_for_run = Arc::clone(&gateway);
        let runtime_roots = Arc::clone(&self.runtime_roots);
        let turn_waiters = Arc::clone(&self.turn_waiters);
        let harness_id_for_run = harness_id.clone();
        let harness_display_for_run = harness_display_name.clone();
        self.runtime.spawn(async move {
            let gateway_vault = Arc::clone(&vault);
            let gateway_observer = Arc::clone(&observer);
            let gateway_blocks_for_events = Arc::clone(&gateway_blocks);
            let mut gateway_event_rx = gateway_event_rx;
            let gateway_event_task = tokio::spawn(async move {
                while let Some(event) = gateway_event_rx.recv().await {
                    record_gateway_content_block(&gateway_blocks_for_events, &event);
                    let _ = append_event_for_gateway_event(&gateway_vault, id, &event);
                    observer_emit_gateway(&gateway_observer, id, &event);
                }
            });
            let workdir_baseline = list_renderable_workdir_paths(&workdir_path).unwrap_or_default();
            let run = adapter.run(ctx, TurnInput { user_message }).await;
            match run {
                Ok(mut run) => {
                    let _ = vault.append_event(
                        id,
                        &Event::new(
                            EventKind::HarnessSpawned,
                            json!({
                                "harness_id": harness_id_for_run,
                                "display_name": harness_display_for_run,
                            }),
                        ),
                    );
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
                    let mut reducer = TurnReducer::new(harness_id_for_run.clone());
                    while let Some(event) = run.events.recv().await {
                        emit(&observer, id, &event, &harness_display_for_run);
                        let _ = append_event_for_harness_event(
                            &vault,
                            id,
                            &event,
                            Some(&harness_display_for_run),
                        );
                        let _ = reducer.apply(&event);
                        if let HarnessEvent::NativeSessionBound {
                            provider,
                            session_id,
                            cwd,
                        } = &event
                        {
                            let _ = persist_native_session_link(
                                &vault,
                                id,
                                provider,
                                session_id,
                                cwd.as_deref(),
                            );
                        }
                        if let HarnessEvent::TurnEnded { reason } = &event {
                            signal_turn_completed(&turn_waiters, id, reason.clone());
                            if matches!(reason, TurnEndReason::Cancelled) {
                                break;
                            }
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
                            let mut extra_paths = reduced.referenced_paths.clone();
                            extra_paths.retain(|path| {
                                !reduced
                                    .file_changes
                                    .iter()
                                    .any(|change| &change.diff.path == path)
                            });
                            match snapshotter
                                .snapshot_referenced_paths(extra_paths.iter().map(String::as_str))
                            {
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
                            match new_renderable_workdir_paths(&workdir_path, &workdir_baseline) {
                                Ok(new_paths) => {
                                    let already_tracked: HashSet<String> = reduced
                                        .file_changes
                                        .iter()
                                        .map(|change| change.diff.path.clone())
                                        .chain(reduced.referenced_paths.iter().cloned())
                                        .collect();
                                    let paths_to_snapshot = new_paths
                                        .iter()
                                        .map(String::as_str)
                                        .filter(|path| !already_tracked.contains(*path));
                                    match snapshotter.snapshot_referenced_paths(paths_to_snapshot) {
                                        Ok(snapshots) => {
                                            for snapshot in snapshots {
                                                if snapshotted
                                                    .insert(snapshot.attachment_path.clone())
                                                {
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
                            let has_gateway_blocks = gateway_blocks
                                .lock()
                                .map(|blocks| !blocks.is_empty())
                                .unwrap_or(false);
                            if !message.content.is_empty() || has_gateway_blocks {
                                if let Ok(mut blocks) = gateway_blocks.lock() {
                                    message.content.extend(blocks.drain(..));
                                }
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
                    let _ = vault.append_event(
                        id,
                        &Event::new(
                            EventKind::HarnessExited,
                            json!({ "harness_id": harness_id_for_run }),
                        ),
                    );
                    if let Ok(mut runs) = active_runs.lock() {
                        runs.remove(&id);
                    }
                    if let Ok(mut roots) = runtime_roots.lock() {
                        roots.remove(&id);
                    }
                }
                Err(err) => {
                    signal_turn_completed(&turn_waiters, id, TurnEndReason::Failed);
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
            gateway_for_run.disconnect_all_clients().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
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
        if let Ok(mut pending) = self.pending_orchestration_consents.lock()
            && let Some(tx) = pending.remove(request_id)
        {
            let allow = !matches!(option_id, "deny" | "reject" | "decline");
            let _ = tx.send(allow);
            self.observer.on_event(UiEvent {
                conversation_id: conversation_id.to_string(),
                kind: "permission_resolved".to_string(),
                payload_json: json!({
                    "type": "permission_resolved",
                    "request_id": request_id,
                    "option_id": option_id,
                })
                .to_string(),
            });
            return Ok(());
        }

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
        self.runtime.block_on(run.gateway.respond_elicitation(
            request_id,
            action.clone(),
            data.clone(),
        ))?;
        run.gateway_blocks
            .lock()
            .map_err(|_| CoreError::Protocol("gateway block lock poisoned".to_string()))?
            .push(elicitation_response_block(
                request_id.to_string(),
                action,
                data,
            ));
        Ok(())
    }

    pub fn resolve_app_template_inner(
        &self,
        conversation_id: &str,
        server_id: &str,
        template_ref: &str,
    ) -> Result<Option<AppTemplateDto>> {
        let id = parse_id(conversation_id)?;
        let gateway = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .map(|run| Arc::clone(&run.gateway));
        let Some(gateway) = gateway else {
            return Ok(None);
        };
        let template = self
            .runtime
            .block_on(gateway.cached_app_template(server_id, template_ref));
        Ok(template.map(app_template_to_dto))
    }

    pub fn submit_app_bridge_request_inner(
        &self,
        conversation_id: &str,
        server_id: &str,
        app_id: &str,
        template_ref: &str,
        request_json: &str,
    ) -> Result<AppBridgeSubmissionDto> {
        let id = parse_id(conversation_id)?;
        if self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .is_none()
        {
            return Err(CoreError::NotFound(id));
        }
        let request = parse_app_bridge_rpc(request_json)?;
        let gateway = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .map(|run| Arc::clone(&run.gateway))
            .ok_or(CoreError::NotFound(id))?;
        match self
            .app_bridge
            .begin_request(id, server_id, app_id, template_ref, &request)?
        {
            AppBridgeBeginResult::AlreadyGranted(pending) => {
                let execution = self.runtime.block_on(execute_action(
                    gateway.as_ref(),
                    &pending.server_id,
                    &pending.action,
                ));
                let response = finish_execution(pending, execution);
                self.observer.on_event(UiEvent {
                    conversation_id: id.to_string(),
                    kind: "app_bridge_resolved".to_string(),
                    payload_json: json!({
                        "request_id": null,
                        "response_json": response,
                        "auto_granted": true,
                    })
                    .to_string(),
                });
                Ok(AppBridgeSubmissionDto {
                    request_id: String::new(),
                    needs_consent: false,
                })
            }
            AppBridgeBeginResult::NeedsConsent(consent, _response_rx) => {
                self.vault.append_event(
                    id,
                    &Event::new(
                        EventKind::AppBridgeConsentRequested,
                        json!({
                            "request_id": consent.request_id,
                            "server_id": consent.server_id,
                            "app_id": consent.app_id,
                            "template_ref": consent.template_ref,
                            "action": consent.action,
                            "summary": consent.summary,
                            "options": consent.options,
                        }),
                    ),
                )?;
                self.observer.on_event(UiEvent {
                    conversation_id: id.to_string(),
                    kind: "app_bridge_consent_requested".to_string(),
                    payload_json: serde_json::to_string(&consent)
                        .unwrap_or_else(|_| "{}".to_string()),
                });
                Ok(AppBridgeSubmissionDto {
                    request_id: consent.request_id,
                    needs_consent: true,
                })
            }
        }
    }

    pub fn respond_app_bridge_consent_inner(
        &self,
        conversation_id: &str,
        request_id: &str,
        option_id: &str,
    ) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let gateway = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .map(|run| Arc::clone(&run.gateway))
            .ok_or(CoreError::NotFound(id))?;
        let resolution = self.app_bridge.resolve_consent(id, request_id, option_id)?;
        let (response, audit) = match resolution {
            AppBridgeResolution::Denied { response, audit } => (response, audit),
            AppBridgeResolution::Approved { pending, audit } => {
                let execution = self.runtime.block_on(execute_action(
                    gateway.as_ref(),
                    &pending.server_id,
                    &pending.action,
                ));
                let response = finish_execution(pending, execution);
                (response, audit)
            }
        };
        self.vault.append_event(
            id,
            &Event::new(
                EventKind::AppBridgeConsentResolved,
                json!({
                    "request_id": request_id,
                    "server_id": audit.server_id,
                    "app_id": audit.app_id,
                    "template_ref": audit.template_ref,
                    "action_kind": audit.action_kind,
                    "arguments_summary": audit.arguments_summary,
                    "resolution": audit.resolution,
                }),
            ),
        )?;
        self.observer.on_event(UiEvent {
            conversation_id: id.to_string(),
            kind: "app_bridge_resolved".to_string(),
            payload_json: json!({
                "request_id": request_id,
                "response_json": response,
            })
            .to_string(),
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
            run.gateway
                .agent_cancelled(serde_json::json!({"requestId": "user-cancel"}));
            run.control.cancel().await
        })
    }

    pub fn prepare_for_app_quit_inner(&self) -> Result<()> {
        let runs: Vec<_> = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .values()
            .map(|run| Arc::clone(&run.gateway))
            .collect();
        self.runtime.block_on(async {
            for gateway in runs {
                gateway.cancel_pending_elicitations().await;
            }
        });
        Ok(())
    }

    pub fn cancel_task_inner(&self, conversation_id: &str, task_id: &str) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let run = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .cloned()
            .ok_or(CoreError::NotFound(id))?;
        self.runtime.block_on(run.gateway.cancel_task(task_id))
    }

    pub fn list_gateway_servers_inner(&self) -> Result<Vec<GatewayServerDto>> {
        let config = load_app_config(self.vault.root())?;
        let cache = self
            .gateway_capability_cache
            .lock()
            .map_err(|_| CoreError::Protocol("capability cache lock poisoned".to_string()))?;
        config
            .gateway
            .servers
            .into_iter()
            .map(|server| {
                gateway_server_to_dto(
                    &server,
                    &self.credentials,
                    cache.get(&server.id),
                    &self.gateway_server_status(&server.id),
                )
            })
            .collect()
    }

    pub fn list_gateway_tools_inner(&self) -> Result<Vec<GatewayToolDto>> {
        let config = load_app_config(self.vault.root())?;
        let gateway = McpGateway::new(config.gateway.clone(), self.credentials.clone(), None)?;
        let tools = self
            .runtime
            .block_on(async { gateway.list_tools().await })?;
        Ok(tools
            .into_iter()
            .map(|tool| GatewayToolDto {
                exposed_name: tool.exposed_name,
                server_id: tool.server_id,
                original_name: tool.original_name,
            })
            .collect())
    }

    pub fn get_gateway_settings_inner(&self) -> Result<GatewaySettingsDto> {
        let config = load_app_config(self.vault.root())?;
        Ok(GatewaySettingsDto {
            default_call_timeout_secs: config.gateway.default_call_timeout_secs,
        })
    }

    pub fn set_gateway_default_timeout_inner(&self, default_call_timeout_secs: u64) -> Result<()> {
        if default_call_timeout_secs == 0 {
            return Err(CoreError::Protocol(
                "gateway default timeout must be greater than zero".to_string(),
            ));
        }
        let mut config = load_app_config(self.vault.root())?;
        config.gateway.default_call_timeout_secs = default_call_timeout_secs;
        save_app_config(self.vault.root(), &config)
    }

    pub fn refresh_gateway_capabilities_inner(&self) -> Result<Vec<GatewayServerDto>> {
        let config = load_app_config(self.vault.root())?;
        let (gateway_event_tx, mut gateway_event_rx) = tokio::sync::mpsc::unbounded_channel();
        let gateway = McpGateway::new(
            config.gateway.clone(),
            self.credentials.clone(),
            Some(gateway_event_tx),
        )?;
        for server in config
            .gateway
            .servers
            .iter()
            .filter(|server| server.enabled)
        {
            let server_id = server.id.clone();
            let outcome = self
                .runtime
                .block_on(async { gateway.check_server_connection(server).await });
            self.drain_gateway_events_to_vault(&mut gateway_event_rx)?;
            match outcome {
                Ok(()) => self.record_gateway_server_status(&server_id, "connected", ""),
                Err(err) => {
                    self.record_gateway_server_status(&server_id, "error", &err.to_string())
                }
            }
        }
        let reports = self
            .runtime
            .block_on(async { gateway.probe_server_capabilities().await })?;
        self.drain_gateway_events_to_vault(&mut gateway_event_rx)?;
        {
            let mut cache = self
                .gateway_capability_cache
                .lock()
                .map_err(|_| CoreError::Protocol("capability cache lock poisoned".to_string()))?;
            for report in reports {
                cache.insert(report.server_id.clone(), report);
            }
        }
        self.list_gateway_servers_inner()
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
        let authorization_url = build_authorization_url(oauth, redirect_uri, &pkce, &state)?;
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
        self.vault.append_vault_event(&Event::new(
            EventKind::OAuthHandoffStarted,
            json!({
                "server_id": server_id,
                "credential_ref": oauth.token_ref.clone(),
            }),
        ))?;
        Ok(OAuthHandoffDto {
            server_id: server_id.to_string(),
            authorization_url,
            state,
            redirect_uri: redirect_uri.to_string(),
        })
    }

    pub fn complete_oauth_callback_inner(&self, callback_url: &str) -> Result<OAuthCompletionDto> {
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
        let tokens = std::thread::scope(
            |scope| -> Result<crate::mcp::oauth::TokenEndpointResponse> {
                let handle = scope.spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|err| {
                            CoreError::Protocol(format!("oauth runtime failed: {err}"))
                        })?;
                    rt.block_on(exchange_authorization_code(
                        &client,
                        oauth,
                        &code,
                        &pending.redirect_uri,
                        &pending.pkce,
                    ))
                });
                handle.join().map_err(|_| {
                    CoreError::Protocol("oauth exchange thread panicked".to_string())
                })?
            },
        )?;
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
        self.vault.append_vault_event(&Event::new(
            EventKind::OAuthHandoffCompleted,
            json!({
                "server_id": pending.server_id,
                "credential_ref": pending.token_ref,
                "status": status,
            }),
        ))?;
        Ok(OAuthCompletionDto {
            server_id: pending.server_id,
            oauth_status: status.to_string(),
        })
    }

    pub fn list_roots_inner(&self, conversation_id: &str) -> Result<Vec<RootDto>> {
        let id = parse_id(conversation_id)?;
        let conversation = self.vault.load(id)?;
        Ok(self
            .effective_roots_for_conversation(&conversation)?
            .iter()
            .map(root_to_dto)
            .collect())
    }

    pub fn attach_root_inner(
        &self,
        conversation_id: &str,
        name: &str,
        uri: &str,
        kind: &str,
        scope: &str,
    ) -> Result<RootDto> {
        let id = parse_id(conversation_id)?;
        let mut conversation = self.vault.load(id)?;
        let root = attach_root(
            &mut conversation,
            name,
            uri,
            parse_root_kind(kind)?,
            parse_root_scope(scope)?,
        )?;
        self.vault.save_meta(&conversation)?;
        self.invalidate_conversation_cache(id);
        Ok(root_to_dto(&root))
    }

    pub fn remove_root_inner(&self, conversation_id: &str, root_id: &str) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let mut conversation = self.vault.load(id)?;
        remove_root(&mut conversation, root_id)?;
        self.vault.save_meta(&conversation)?;
        self.invalidate_conversation_cache(id);
        Ok(())
    }

    pub fn sync_runtime_roots_inner(
        &self,
        conversation_id: &str,
        roots: Vec<RootDto>,
    ) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let roots: Vec<Root> = roots
            .into_iter()
            .map(|dto| root_from_dto(&dto))
            .collect::<Result<_>>()?;
        for root in &roots {
            validate_root(root)?;
        }
        self.runtime_roots
            .lock()
            .map_err(|_| CoreError::Protocol("runtime roots lock poisoned".to_string()))?
            .insert(id, roots);
        Ok(())
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

    pub fn write_workdir_file_inner(
        &self,
        conversation_id: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<String> {
        let id = parse_id(conversation_id)?;
        let workdir = self
            .vault
            .conversation_workdir(id)?
            .ok_or_else(|| CoreError::Protocol("conversation has no workdir".to_string()))?;
        fs::create_dir_all(&workdir)?;
        let safe_name = safe_workdir_filename(filename);
        fs::write(workdir.join(&safe_name), data)?;
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

    pub fn conversation_folder_path_inner(&self, conversation_id: &str) -> Result<String> {
        let id = parse_id(conversation_id)?;
        let folder = self.vault.conversation_folder(id)?;
        Ok(folder.to_string_lossy().into_owned())
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
        Ok(WorkdirFileContentDto {
            mime_type,
            data: bytes,
        })
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

    pub fn resolve_artifact_path_inner(&self, conversation_id: &str, path: &str) -> Result<String> {
        let id = parse_id(conversation_id)?;
        let conversation = self.vault.load(id)?;
        let mut size = None;
        let mut sha256 = None;
        for message in &conversation.messages {
            for block in &message.content {
                if let ContentBlock::Artifact {
                    path: artifact_path,
                    size: artifact_size,
                    sha256: artifact_sha,
                    ..
                } = block
                    && artifact_path == path
                {
                    size = Some(*artifact_size);
                    sha256 = Some(artifact_sha.clone());
                    break;
                }
            }
            if size.is_some() {
                break;
            }
        }
        let (size, sha256) = match (size, sha256) {
            (Some(size), Some(sha256)) => (size, sha256),
            _ => {
                return Err(CoreError::MalformedVault(format!(
                    "artifact not found in transcript: {path}"
                )));
            }
        };
        self.verified_attachment_path_inner(conversation_id, path, size, &sha256)
    }

    pub fn import_bundle_or_folder_as_new_inner(
        &self,
        source_path: PathBuf,
    ) -> Result<ImportResultDto> {
        let result = import_bundle_or_folder_as_new(&self.vault, &source_path)?;
        self.invalidate_conversation_cache(result.conversation.id);
        Ok(ImportResultDto {
            conversation: conversation_to_dto(&result.conversation)?,
            warnings: result
                .warnings
                .into_iter()
                .map(|warning| ImportWarningDto {
                    kind: warning.kind,
                    detail: warning.detail,
                })
                .collect(),
        })
    }

    pub fn search_conversations_inner(&self, query: &str) -> Result<Vec<SearchHitDto>> {
        Ok(search_conversations(self.vault.as_ref(), query)?
            .into_iter()
            .map(|hit| SearchHitDto {
                conversation_id: hit.conversation_id.to_string(),
                title: hit.title,
                snippet: hit.snippet,
                match_field: match hit.match_field {
                    SearchMatchField::Title => "title".to_string(),
                    SearchMatchField::Text => "text".to_string(),
                    SearchMatchField::Thinking => "thinking".to_string(),
                },
            })
            .collect())
    }

    pub fn list_harness_health_inner(&self) -> Result<Vec<HarnessHealthEntryDto>> {
        Ok(
            health_entries_from_roster(&self.roster_specs_from_config()?)
                .into_iter()
                .map(|entry| HarnessHealthEntryDto {
                    id: entry.id,
                    display_name: entry.display_name,
                    command: entry.command,
                    status: match entry.status {
                        HarnessHealthStatus::Missing => "missing".to_string(),
                        HarnessHealthStatus::Ready => "ready".to_string(),
                        HarnessHealthStatus::Unknown => "unknown".to_string(),
                    },
                    install_doc_url: entry.install_doc_url,
                })
                .collect(),
        )
    }

    pub fn harness_health_checklist_inner(&self) -> Result<String> {
        Ok(it_admin_checklist(&health_entries_from_roster(
            &self.roster_specs_from_config()?,
        )))
    }

    fn roster_specs_from_config(&self) -> Result<Vec<AgentLaunchSpec>> {
        let config = load_app_config(self.vault.root())?;
        Ok(crate::harness::discovery::sort_roster_by_picker_order(
            config.agent_roster,
            &config.harness_order,
        ))
    }

    fn picker_settings_from_config(&self) -> Result<HarnessPickerSettingsDto> {
        let config = load_app_config(self.vault.root())?;
        Ok(HarnessPickerSettingsDto {
            harness_order: config.harness_order,
            hidden_harness_ids: config.hidden_harness_ids,
            enable_cli_update_checks: config.enable_cli_update_checks,
        })
    }

    pub fn list_harness_providers_inner(&self) -> Result<Vec<HarnessProviderEntryDto>> {
        let config = load_app_config(self.vault.root())?;
        let hidden: std::collections::HashSet<_> =
            config.hidden_harness_ids.iter().cloned().collect();
        let order_index = |id: &str| -> Option<u32> {
            config
                .harness_order
                .iter()
                .position(|entry| entry == id)
                .map(|index| index as u32)
        };
        let roster = self.roster_specs_from_config()?;
        let adapters = self
            .adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?;
        Ok(roster
            .iter()
            .map(|spec| {
                let diagnostic = adapters
                    .get(&spec.id)
                    .map(|adapter| adapter.readiness_diagnostic(spec.enabled))
                    .unwrap_or_else(|| diagnose_agent(spec, spec.enabled));
                // Never probe live models here: available_models() can spawn agents and
                // block_on inside spawn_blocking deadlocks the daemon runtime.
                let model_count = None;
                let legacy_status = match diagnostic.state {
                    ReadinessState::Missing => "missing",
                    ReadinessState::Ready => "ready",
                    ReadinessState::Disabled => "disabled",
                    ReadinessState::SignInRequired => "sign_in_required",
                    ReadinessState::Misconfigured => "misconfigured",
                    ReadinessState::CheckFailed => "check_failed",
                    ReadinessState::Installed => "installed",
                };
                HarnessProviderEntryDto {
                    id: spec.id.clone(),
                    display_name: spec.display_name.clone(),
                    command: spec.command.clone(),
                    status: legacy_status.to_string(),
                    readiness_state: diagnostic.state.as_str().to_string(),
                    recovery_action: diagnostic.recovery_action.clone(),
                    readiness_message: diagnostic.message.clone(),
                    install_doc_url: diagnostic.install_doc_url.clone(),
                    adapter_type: adapter_type_label(&spec.adapter).to_string(),
                    adapter_kind: adapter_kind_label(&spec.adapter).to_string(),
                    enabled: spec.enabled,
                    model_count,
                    hidden: hidden.contains(&spec.id),
                    picker_order: order_index(&spec.id),
                }
            })
            .collect())
    }

    pub fn list_harness_installed_clis_inner(&self) -> Result<Vec<HarnessInstalledCliDto>> {
        let config = load_app_config(self.vault.root())?;
        Ok(crate::harness::discovery::list_installed_clis(&config.agent_roster)
            .into_iter()
            .map(|cli| HarnessInstalledCliDto {
                id: cli.id,
                display_name: cli.display_name,
                command: cli.command,
                version: cli.version,
                install_doc_url: cli.install_doc_url,
                in_roster: cli.in_roster,
                auth_ready: cli.auth_ready,
            })
            .collect())
    }

    pub fn harness_picker_settings_get_inner(&self) -> Result<HarnessPickerSettingsDto> {
        self.picker_settings_from_config()
    }

    pub fn harness_picker_settings_set_inner(
        &self,
        harness_order: Vec<String>,
        hidden_harness_ids: Vec<String>,
        enable_cli_update_checks: bool,
    ) -> Result<()> {
        crate::harness::discovery::update_picker_settings(
            self.vault.root(),
            harness_order,
            hidden_harness_ids,
            enable_cli_update_checks,
        )
    }

    pub fn harness_discovery_sync_inner(&self) -> Result<()> {
        crate::harness::discovery::sync_agent_roster_with_discovery(self.vault.root())?;
        let config = load_app_config(self.vault.root())?;
        let mut adapters = self
            .adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?;
        for spec in &config.agent_roster {
            adapters.insert(spec.id.clone(), crate::harness::registry::build_adapter(spec));
        }
        Ok(())
    }

    pub fn harness_readiness_recommend_inner(&self) -> Result<HarnessReadinessRecommendDto> {
        let providers = self.list_harness_providers_inner()?;
        let recommendable: Vec<RecommendableAgent<'_>> = providers
            .iter()
            .map(|entry| RecommendableAgent {
                id: &entry.id,
                display_name: &entry.display_name,
                state: match entry.readiness_state.as_str() {
                    "missing" => ReadinessState::Missing,
                    "installed" => ReadinessState::Installed,
                    "sign_in_required" => ReadinessState::SignInRequired,
                    "ready" => ReadinessState::Ready,
                    "disabled" => ReadinessState::Disabled,
                    "misconfigured" => ReadinessState::Misconfigured,
                    "check_failed" => ReadinessState::CheckFailed,
                    _ => ReadinessState::CheckFailed,
                },
                recovery_action: entry.recovery_action.clone(),
            })
            .collect();
        if let Some(picked) = recommend_agent(&recommendable) {
            let provider = providers
                .iter()
                .find(|entry| entry.id == picked.id)
                .expect("recommendation matches provider list");
            return Ok(HarnessReadinessRecommendDto {
                agent_id: Some(picked.id.to_string()),
                display_name: picked.display_name.to_string(),
                readiness_state: picked.state.as_str().to_string(),
                recovery_action: picked.recovery_action.clone(),
                message: provider.readiness_message.clone(),
                install_doc_url: provider.install_doc_url.clone(),
            });
        }
        Ok(HarnessReadinessRecommendDto {
            agent_id: None,
            display_name: "No agent app".into(),
            readiness_state: ReadinessState::Missing.as_str().to_string(),
            recovery_action: "ask_it".into(),
            message: Some("Install at least one agent app to run your own files.".into()),
            install_doc_url: String::new(),
        })
    }

    pub fn harness_roster_set_enabled_inner(&self, agent_id: &str, enabled: bool) -> Result<()> {
        set_agent_enabled(self.vault.root(), agent_id, enabled)?;
        let config = load_app_config(self.vault.root())?;
        if let Some(spec) = config
            .agent_roster
            .iter()
            .find(|entry| entry.id == agent_id)
        {
            let mut adapters = self
                .adapters
                .lock()
                .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?;
            adapters.insert(
                spec.id.clone(),
                crate::harness::registry::build_adapter(spec),
            );
        }
        Ok(())
    }

    pub fn harness_roster_add_inner(&self, spec: AgentLaunchSpec) -> Result<()> {
        add_agent_to_roster(self.vault.root(), spec.clone())?;
        let mut adapters = self
            .adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?;
        adapters.insert(
            spec.id.clone(),
            crate::harness::registry::build_adapter(&spec),
        );
        Ok(())
    }

    pub fn list_harness_usage_inner(&self) -> Result<HarnessUsageListDto> {
        let roster = self.roster_specs_from_config()?;
        let ids = roster
            .iter()
            .map(|spec| spec.id.clone())
            .collect::<Vec<_>>();
        Ok(HarnessUsageListDto {
            providers: list_harness_usage(&ids)
                .into_iter()
                .map(harness_usage_entry_to_dto)
                .collect(),
        })
    }

    pub fn list_recipes_inner(&self) -> Result<Vec<RecipeSummary>> {
        orchestration::list_recipes(self.vault.root())
    }

    pub fn load_recipe_inner(&self, recipe_id: &str) -> Result<RecipeLoadDto> {
        Ok(RecipeLoadDto {
            recipe_json: orchestration::store::load_recipe_json(self.vault.root(), recipe_id)?,
        })
    }

    pub fn orchestration_run_inner(
        &self,
        recipe_id: &str,
        source_conversation_id: &str,
        inputs_json: Option<&str>,
    ) -> Result<OrchestrationRunDto> {
        let _ = self.vault.load(parse_id(source_conversation_id)?)?;
        let recipe = orchestration::load_recipe(self.vault.root(), recipe_id)?;
        let inputs: HashMap<String, String> = inputs_json
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_default();
        let run_id = Uuid::now_v7().to_string();
        let mut run = OrchestrationRunMeta::new(
            run_id.clone(),
            recipe_id.to_string(),
            source_conversation_id.to_string(),
        );
        orchestration::store::save_run(self.vault.root(), &run)?;

        let cancelled = Arc::new(AtomicBool::new(false));
        if let Ok(mut active) = self.active_orchestration_runs.lock() {
            active.insert(
                run_id.clone(),
                OrchestrationRunHandle {
                    cancelled: Arc::clone(&cancelled),
                },
            );
        }

        self.emit_orchestration_ui(&run, "orchestration_started", orchestration_started(&run));
        let initial_dto = run.to_dto();

        let Some(core) = Self::shared() else {
            return self.orchestration_run_sync_fallback(&recipe, &mut run, &inputs);
        };

        let vault_root = self.vault.root().to_path_buf();
        let observer = Arc::clone(&self.observer);
        let active_orchestration_runs = Arc::clone(&self.active_orchestration_runs);
        self.runtime.spawn(async move {
            let mut run = run;
            let result =
                orchestration::engine::execute_async(&core, &recipe, &mut run, &inputs, &cancelled)
                    .await;
            match result {
                Ok(()) if cancelled.load(Ordering::SeqCst) => {
                    run.status = OrchestrationRunStatus::Cancelled;
                    run.error = Some("cancelled by user".to_string());
                }
                Ok(()) => run.status = OrchestrationRunStatus::Completed,
                Err(_) if cancelled.load(Ordering::SeqCst) => {
                    run.status = OrchestrationRunStatus::Cancelled;
                    run.error = Some("cancelled by user".to_string());
                }
                Err(err) => {
                    run.status = OrchestrationRunStatus::Failed;
                    run.error = Some(err.to_string());
                }
            }
            run.touch();
            let _ = orchestration::store::save_run(&vault_root, &run);
            let dto = run.to_dto();
            observer.on_event(UiEvent {
                conversation_id: run.source_conversation_id.clone(),
                kind: "orchestration_finished".to_string(),
                payload_json: orchestration_finished(&dto).to_string(),
            });
            if let Ok(mut active) = active_orchestration_runs.lock() {
                active.remove(&run.id);
            }
        });

        Ok(initial_dto)
    }

    fn orchestration_run_sync_fallback(
        &self,
        recipe: &orchestration::Recipe,
        run: &mut OrchestrationRunMeta,
        inputs: &HashMap<String, String>,
    ) -> Result<OrchestrationRunDto> {
        match orchestration::engine::execute(self, recipe, run, inputs) {
            Ok(()) => run.status = OrchestrationRunStatus::Completed,
            Err(err) => {
                run.status = OrchestrationRunStatus::Failed;
                run.error = Some(err.to_string());
            }
        }
        run.touch();
        orchestration::store::save_run(self.vault.root(), run)?;
        self.emit_orchestration_ui(
            run,
            "orchestration_finished",
            orchestration_finished(&run.to_dto()),
        );
        Ok(run.to_dto())
    }

    pub fn orchestration_status_inner(&self, run_id: &str) -> Result<OrchestrationRunDto> {
        Ok(orchestration::store::load_run(self.vault.root(), run_id)?.to_dto())
    }

    pub fn orchestration_cancel_inner(&self, run_id: &str) -> Result<OrchestrationRunDto> {
        let mut run = orchestration::store::load_run(self.vault.root(), run_id)?;
        if run.status != OrchestrationRunStatus::Running {
            return Ok(run.to_dto());
        }
        if let Ok(active) = self.active_orchestration_runs.lock()
            && let Some(handle) = active.get(run_id)
        {
            handle.cancelled.store(true, Ordering::SeqCst);
        }
        let _ = self.cancel_run_inner(&run.latest_conversation_id);
        run.status = OrchestrationRunStatus::Cancelled;
        run.error = Some("cancelled by user".to_string());
        run.touch();
        orchestration::store::save_run(self.vault.root(), &run)?;
        self.emit_orchestration_ui(
            &run,
            "orchestration_finished",
            orchestration_finished(&run.to_dto()),
        );
        Ok(run.to_dto())
    }

    pub fn install_shared(core: Arc<Self>) {
        let _ = SHARED_CORE.set(core);
    }

    pub fn shared() -> Option<Arc<Self>> {
        SHARED_CORE.get().cloned()
    }

    pub(crate) fn emit_orchestration_ui(
        &self,
        run: &OrchestrationRunMeta,
        kind: &str,
        payload: serde_json::Value,
    ) {
        self.observer.on_event(UiEvent {
            conversation_id: run.source_conversation_id.clone(),
            kind: kind.to_string(),
            payload_json: payload.to_string(),
        });
    }

    pub(crate) fn vault_root(&self) -> &Path {
        self.vault.root()
    }

    pub(crate) fn register_turn_waiter(&self, id: Id, tx: oneshot::Sender<TurnEndReason>) {
        if let Ok(mut waiters) = self.turn_waiters.lock() {
            waiters.entry(id).or_default().push(tx);
        }
    }

    pub(crate) fn send_message_and_wait_inner(
        &self,
        conversation_id: &str,
        text: &str,
    ) -> Result<TurnEndReason> {
        let id = parse_id(conversation_id)?;
        let (tx, rx) = oneshot::channel();
        self.register_turn_waiter(id, tx);
        if let Err(err) = self.send_message_inner(conversation_id, text) {
            if let Ok(mut waiters) = self.turn_waiters.lock() {
                waiters.remove(&id);
            }
            return Err(err);
        }
        self.wait_turn_receiver(conversation_id, rx)
    }

    pub(crate) fn wait_turn_receiver(
        &self,
        conversation_id: &str,
        rx: oneshot::Receiver<TurnEndReason>,
    ) -> Result<TurnEndReason> {
        match self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(TURN_WAIT_TIMEOUT_SECS), rx).await
        }) {
            Ok(Ok(reason)) => Ok(reason),
            Ok(Err(_)) => Err(CoreError::Protocol(format!(
                "turn waiter dropped for {conversation_id}"
            ))),
            Err(_) => {
                if let (Ok(id), Ok(mut waiters)) =
                    (parse_id(conversation_id), self.turn_waiters.lock())
                {
                    waiters.remove(&id);
                }
                Err(CoreError::Timeout {
                    method: "turn_wait".to_string(),
                })
            }
        }
    }

    pub(crate) async fn send_message_and_wait_async(
        &self,
        conversation_id: &str,
        text: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<TurnEndReason> {
        let id = parse_id(conversation_id)?;
        let (tx, rx) = oneshot::channel();
        self.register_turn_waiter(id, tx);
        if let Err(err) = self.send_message_inner(conversation_id, text) {
            if let Ok(mut waiters) = self.turn_waiters.lock() {
                waiters.remove(&id);
            }
            return Err(err);
        }
        self.wait_turn_receiver_async_inner(conversation_id, id, rx, cancel)
            .await
    }

    pub(crate) async fn wait_turn_receiver_async(
        &self,
        conversation_id: &str,
        rx: oneshot::Receiver<TurnEndReason>,
        cancel: &Arc<AtomicBool>,
    ) -> Result<TurnEndReason> {
        let id = parse_id(conversation_id)?;
        self.wait_turn_receiver_async_inner(conversation_id, id, rx, cancel)
            .await
    }

    async fn wait_turn_receiver_async_inner(
        &self,
        conversation_id: &str,
        id: Id,
        mut rx: oneshot::Receiver<TurnEndReason>,
        cancel: &Arc<AtomicBool>,
    ) -> Result<TurnEndReason> {
        loop {
            if cancel.load(Ordering::SeqCst) {
                if let Ok(mut waiters) = self.turn_waiters.lock() {
                    waiters.remove(&id);
                }
                return Err(CoreError::Protocol("orchestration cancelled".to_string()));
            }
            match tokio::time::timeout(Duration::from_millis(250), &mut rx).await {
                Ok(Ok(reason)) => return Ok(reason),
                Ok(Err(_)) => {
                    return Err(CoreError::Protocol(format!(
                        "turn waiter dropped for {conversation_id}"
                    )));
                }
                Err(_) => continue,
            }
        }
    }

    pub async fn handle_orchestration_tool(
        self: &Arc<Self>,
        conversation_id: &str,
        exposed_name: &str,
        arguments: serde_json::Value,
        _meta: Option<serde_json::Value>,
    ) -> Result<CallToolResult> {
        let tool = native_original_name(exposed_name)
            .ok_or_else(|| CoreError::Protocol(format!("unknown native tool: {exposed_name}")))?;
        match tool {
            TOOL_ORCHESTRATION_STATUS => {
                let run_id = arguments
                    .get("run_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| CoreError::Protocol("run_id required".to_string()))?;
                let dto = self.orchestration_status_inner(run_id)?;
                Ok(tool_result_structured(json!({ "run": dto })))
            }
            TOOL_ORCHESTRATION_CANCEL => {
                let run_id = arguments
                    .get("run_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| CoreError::Protocol("run_id required".to_string()))?;
                if !self
                    .await_orchestration_consent(
                        conversation_id,
                        &format!("Cancel orchestration run {run_id}"),
                        &arguments,
                    )
                    .await?
                {
                    return Ok(tool_result_error("orchestration cancel declined"));
                }
                let dto = self.orchestration_cancel_inner(run_id)?;
                Ok(tool_result_structured(json!({ "run": dto })))
            }
            TOOL_ORCHESTRATION_HANDOFF => {
                let harness_id = arguments
                    .get("harness_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| CoreError::Protocol("harness_id required".to_string()))?;
                let model_id = arguments
                    .get("model_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| CoreError::Protocol("model_id required".to_string()))?;
                let message = arguments
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| CoreError::Protocol("message required".to_string()))?;
                let inputs_json = json!({
                    "harness_id": harness_id,
                    "model_id": model_id,
                    "message": message,
                })
                .to_string();
                if !self
                    .await_orchestration_consent(
                        conversation_id,
                        "Hand off conversation to another harness",
                        &json!({ "recipe_id": "handoff", "inputs_json": inputs_json }),
                    )
                    .await?
                {
                    return Ok(tool_result_error("orchestration handoff declined"));
                }
                let dto = self.orchestration_run_inner(
                    "handoff",
                    conversation_id,
                    Some(inputs_json.as_str()),
                )?;
                Ok(tool_result_structured(json!({ "run": dto })))
            }
            TOOL_ORCHESTRATION_RUN => {
                let recipe_id = arguments
                    .get("recipe_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| CoreError::Protocol("recipe_id required".to_string()))?;
                let inputs_json = arguments.get("inputs_json").and_then(|value| {
                    if value.is_string() {
                        value.as_str().map(str::to_string)
                    } else {
                        Some(value.to_string())
                    }
                });
                if !self
                    .await_orchestration_consent(
                        conversation_id,
                        &format!("Run orchestration recipe: {recipe_id}"),
                        &arguments,
                    )
                    .await?
                {
                    return Ok(tool_result_error("orchestration run declined"));
                }
                let dto = self.orchestration_run_inner(
                    recipe_id,
                    conversation_id,
                    inputs_json.as_deref(),
                )?;
                Ok(tool_result_structured(json!({ "run": dto })))
            }
            _ => Err(CoreError::Protocol(format!(
                "unsupported native tool: {tool}"
            ))),
        }
    }

    async fn await_orchestration_consent(
        &self,
        conversation_id: &str,
        action: &str,
        detail: &serde_json::Value,
    ) -> Result<bool> {
        let request_id = Uuid::now_v7().to_string();
        let (tx, rx) = oneshot::channel();
        if let Ok(mut pending) = self.pending_orchestration_consents.lock() {
            pending.insert(request_id.clone(), tx);
        }
        self.observer.on_event(UiEvent {
            conversation_id: conversation_id.to_string(),
            kind: "permission_requested".to_string(),
            payload_json: json!({
                "type": "permission_requested",
                "request_id": request_id,
                "action": action,
                "detail": { "type": "other", "value": detail },
                "options": [
                    { "id": "allow_once", "label": "Allow once" },
                    { "id": "deny", "label": "Deny" },
                ],
            })
            .to_string(),
        });
        let allowed =
            match tokio::time::timeout(Duration::from_secs(ORCHESTRATION_CONSENT_TIMEOUT_SECS), rx)
                .await
            {
                Ok(Ok(value)) => value,
                Ok(Err(_)) => false,
                Err(_) => false,
            };
        self.observer.on_event(UiEvent {
            conversation_id: conversation_id.to_string(),
            kind: "permission_resolved".to_string(),
            payload_json: json!({
                "type": "permission_resolved",
                "request_id": request_id,
                "option_id": if allowed { "allow_once" } else { "deny" },
            })
            .to_string(),
        });
        Ok(allowed)
    }

    pub fn vault_issues_inner(&self) -> Result<Vec<VaultIssueDto>> {
        Ok(self
            .vault
            .issues()?
            .iter()
            .map(vault_issue_to_dto)
            .collect())
    }

    pub fn write_diagnostics_bundle_inner(
        &self,
        dest_path: PathBuf,
        system_info_json: &str,
    ) -> Result<String> {
        let system_info: serde_json::Value =
            serde_json::from_str(system_info_json).unwrap_or_else(|_| json!({}));
        let app_config = load_app_config(self.vault.root())?;
        let harness_health = self.list_harness_health_inner()?;
        let harness_health_json = serde_json::to_value(&harness_health)?;
        let issues = self.vault_issues_inner()?;
        let issues_json = serde_json::to_value(&issues)?;
        let bundle = diagnostics::write_diagnostics_bundle(
            self.vault.root(),
            &dest_path,
            &app_config,
            &harness_health_json,
            &issues_json,
            &system_info,
        )?;
        Ok(bundle.display().to_string())
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

fn vault_issue_to_dto(issue: &VaultIssue) -> VaultIssueDto {
    match issue {
        VaultIssue::DuplicateId { id, winner, losers } => VaultIssueDto {
            kind: "duplicate_id".to_string(),
            conversation_id: Some(id.to_string()),
            path: None,
            reason: None,
            winner_path: Some(winner.display().to_string()),
            loser_paths: losers
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            detail: format!(
                "Duplicate conversation id {}. Active folder: {}. Duplicate folders: {}.",
                id,
                winner.display(),
                losers
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
        VaultIssue::TornTailDetected { id } => VaultIssueDto {
            kind: "torn_tail".to_string(),
            conversation_id: Some(id.to_string()),
            path: None,
            reason: None,
            winner_path: None,
            loser_paths: Vec::new(),
            detail: format!(
                "Conversation {} has a torn final line in messages.jsonl. tamtri repairs this on the next write.",
                id
            ),
        },
        VaultIssue::UnreadableFolder { path, reason } => VaultIssueDto {
            kind: "unreadable_folder".to_string(),
            conversation_id: None,
            path: Some(path.display().to_string()),
            reason: Some(reason.clone()),
            winner_path: None,
            loser_paths: Vec::new(),
            detail: format!(
                "Unreadable conversation folder {}: {}",
                path.display(),
                reason
            ),
        },
    }
}

fn persist_native_session_link(
    vault: &FilesystemVault,
    conversation_id: Id,
    provider: &str,
    session_id: &str,
    cwd: Option<&str>,
) -> Result<()> {
    let mut conversation = vault.load(conversation_id)?;
    conversation.native_session = Some(crate::conversation::NativeSessionLink {
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        cwd: cwd
            .map(str::to_string)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                conversation
                    .native_session
                    .as_ref()
                    .map(|link| link.cwd.clone())
            })
            .unwrap_or_default(),
        source_path: conversation
            .native_session
            .as_ref()
            .and_then(|link| link.source_path.clone()),
    });
    vault.save_meta(&conversation)?;
    Ok(())
}

fn emit(
    observer: &Arc<dyn ConversationObserver>,
    conversation_id: Id,
    event: &HarnessEvent,
    harness_display_name: &str,
) {
    let payload_json = match event {
        HarnessEvent::PermissionRequested { .. } => {
            let mut value = serde_json::to_value(event)
                .unwrap_or_else(|_| serde_json::Value::Object(Default::default()));
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "harness_display_name".to_string(),
                    json!(harness_display_name),
                );
            }
            value.to_string()
        }
        _ => serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string()),
    };
    observer.on_event(UiEvent {
        conversation_id: conversation_id.to_string(),
        kind: event_kind(event).to_string(),
        payload_json,
    });
}

fn append_event_for_harness_event(
    vault: &FilesystemVault,
    id: Id,
    event: &HarnessEvent,
    harness_display_name: Option<&str>,
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
        } => {
            let mut payload = json!({
                "request_id": request_id,
                "action": action,
                "detail": detail,
                "options": options,
            });
            if let Some(name) = harness_display_name {
                payload["harness_display_name"] = json!(name);
            }
            (EventKind::PermissionRequested, payload)
        }
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
    let (kind, payload) = gateway_event_to_audit(event);
    vault.append_event(id, &Event::new(kind, payload))
}

fn append_vault_event_for_gateway_event(
    vault: &FilesystemVault,
    event: &GatewayEvent,
) -> Result<()> {
    let (kind, payload) = gateway_event_to_audit(event);
    vault.append_vault_event(&Event::new(kind, payload))
}

fn gateway_event_to_audit(event: &GatewayEvent) -> (EventKind, serde_json::Value) {
    match event {
        GatewayEvent::ServerConnected { server_id } => (
            EventKind::GatewayServerConnected,
            json!({ "server_id": server_id }),
        ),
        GatewayEvent::ServerDisconnected { server_id } => (
            EventKind::GatewayServerDisconnected,
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
        GatewayEvent::AppReturned {
            server_id,
            origin_tool_call_id,
            uri,
            template_ref,
            state,
        } => (
            EventKind::AppReturned,
            json!({
                "server_id": server_id,
                "origin_tool_call_id": origin_tool_call_id,
                "uri": uri,
                "template_ref": template_ref,
                "state": state,
            }),
        ),
        GatewayEvent::TaskStarted { state } => (
            EventKind::TaskStarted,
            serde_json::to_value(state).unwrap_or_else(|_| json!({})),
        ),
        GatewayEvent::TaskUpdated { state } => (
            EventKind::TaskUpdated,
            serde_json::to_value(state).unwrap_or_else(|_| json!({})),
        ),
        GatewayEvent::TaskCompleted { state, result } => (
            EventKind::TaskCompleted,
            json!({"state": state, "result": result}),
        ),
        GatewayEvent::RootsListed { server_id, count } => (
            EventKind::RootsListed,
            json!({ "server_id": server_id, "count": count }),
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
        GatewayEvent::CredentialUpdated {
            server_id,
            credential_ref,
            reason,
        } => (
            EventKind::GatewayCredentialInjected,
            json!({
                "server_id": server_id,
                "credential_ref": credential_ref,
                "target_kind": reason
            }),
        ),
    }
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
        HarnessEvent::NativeSessionBound { .. } => "native_session_bound",
        HarnessEvent::TurnEnded { .. } => "turn_ended",
    }
}

fn gateway_event_kind(event: &GatewayEvent) -> &'static str {
    match event {
        GatewayEvent::ServerConnected { .. } => "gateway_server_connected",
        GatewayEvent::ServerDisconnected { .. } => "gateway_server_disconnected",
        GatewayEvent::ToolRouted { .. } => "gateway_tool_routed",
        GatewayEvent::CredentialInjected { .. } => "gateway_credential_injected",
        GatewayEvent::CredentialUpdated { .. } => "gateway_credential_updated",
        GatewayEvent::Progress { .. } => "gateway_progress",
        GatewayEvent::Log { .. } => "gateway_log",
        GatewayEvent::Cancellation { .. } => "gateway_cancellation",
        GatewayEvent::DownstreamError { .. } => "gateway_downstream_error",
        GatewayEvent::ElicitationRequested { .. } => "elicitation_requested",
        GatewayEvent::ElicitationResolved { .. } => "elicitation_resolved",
        GatewayEvent::AppReturned { .. } => "app_returned",
        GatewayEvent::TaskStarted { .. } => "task_started",
        GatewayEvent::TaskUpdated { .. } => "task_updated",
        GatewayEvent::TaskCompleted { .. } => "task_completed",
        GatewayEvent::RootsListed { .. } => "roots_listed",
        GatewayEvent::OAuthHandoffStarted { .. } => "oauth_handoff_started",
        GatewayEvent::OAuthHandoffCompleted { .. } => "oauth_handoff_completed",
        GatewayEvent::OAuthRefreshFailed { .. } => "oauth_refresh_failed",
    }
}

fn record_gateway_content_block(blocks: &Mutex<Vec<ContentBlock>>, event: &GatewayEvent) {
    match event {
        GatewayEvent::ElicitationRequested {
            request_id,
            server_id,
            origin_tool_call_id,
            mode,
            message,
            schema,
            url,
            ..
        } => {
            let Ok(mut blocks) = blocks.lock() else {
                return;
            };
            blocks.push(elicitation_request_block(
                request_id.clone(),
                server_id.clone(),
                origin_tool_call_id.clone(),
                mode.clone(),
                message.clone(),
                schema.clone(),
                url.clone(),
            ));
        }
        GatewayEvent::AppReturned {
            server_id,
            origin_tool_call_id,
            uri,
            template_ref,
            state,
        } => {
            let Ok(mut blocks) = blocks.lock() else {
                return;
            };
            blocks.push(ContentBlock::AppResource {
                uri: uri.clone(),
                template_ref: template_ref.clone(),
                state: state.clone(),
                server_id: Some(server_id.clone()),
                origin_tool_call_id: origin_tool_call_id.clone(),
            });
        }
        GatewayEvent::TaskCompleted { state, .. } => {
            let Ok(mut blocks) = blocks.lock() else {
                return;
            };
            blocks.push(ContentBlock::TaskRef {
                task_id: state.task_id.clone(),
                status: state.status.clone(),
                title: state.title.clone(),
                result_summary: state.result.as_ref().map(summarize_task_result),
                origin_tool_call_id: state.origin_tool_call_id.clone(),
            });
        }
        _ => {}
    }
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

fn capability_label(
    report: Option<&ServerCapabilityReport>,
    pick: impl Fn(&ServerCapabilityReport) -> FeatureStatus,
) -> String {
    report
        .map(pick)
        .unwrap_or(FeatureStatus::Unknown)
        .label()
        .to_string()
}

fn gateway_server_to_dto(
    server: &GatewayServerConfig,
    credentials: &DurableCredentials,
    capability_report: Option<&ServerCapabilityReport>,
    status: &GatewayServerStatus,
) -> Result<GatewayServerDto> {
    let credential_refs = server
        .credentials
        .iter()
        .map(|credential| credential.credential_ref.clone())
        .collect::<Vec<_>>();
    let missing_credential_refs = credential_refs
        .iter()
        .filter_map(
            |credential_ref| match credentials.contains(credential_ref) {
                Ok(true) => None,
                Ok(false) | Err(_) => Some(credential_ref.clone()),
            },
        )
        .collect::<Vec<_>>();
    let (transport, stdio_command, stdio_args, stdio_env, http_endpoint) = match &server.transport {
        GatewayTransport::Stdio { command, args, env } => (
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
    let (
        oauth_token_ref,
        oauth_client_id,
        oauth_authorization_endpoint,
        oauth_token_endpoint,
        oauth_scopes,
    ) = server
        .oauth
        .as_ref()
        .map(|oauth| {
            (
                oauth.token_ref.clone(),
                oauth.client_id.clone(),
                oauth.authorization_endpoint.clone().unwrap_or_default(),
                oauth.token_endpoint.clone().unwrap_or_default(),
                oauth.scopes.clone(),
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
        oauth_token_endpoint,
        oauth_scopes,
        cap_tools: capability_label(capability_report, |report| report.tools),
        cap_resources: capability_label(capability_report, |report| report.resources),
        cap_prompts: capability_label(capability_report, |report| report.prompts),
        cap_elicitation: capability_label(capability_report, |report| report.elicitation),
        cap_apps: capability_label(capability_report, |report| report.apps),
        cap_tasks: capability_label(capability_report, |report| report.tasks),
        cap_roots: capability_label(capability_report, |report| report.roots),
        cap_sampling: capability_label(capability_report, |report| report.sampling),
        connection_status: if !server.enabled {
            "disabled".to_string()
        } else if status.connection_status == "unknown" && capability_report.is_some() {
            "connected".to_string()
        } else {
            status.connection_status.clone()
        },
        last_error: status.last_error.clone(),
        timeout_secs: server.timeout_secs,
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

    let incoming_oauth_empty = server.oauth_token_ref.trim().is_empty()
        && server.oauth_client_id.trim().is_empty()
        && server.oauth_authorization_endpoint.trim().is_empty()
        && server.oauth_token_endpoint.trim().is_empty()
        && server
            .oauth_scopes
            .iter()
            .all(|scope| scope.trim().is_empty());

    let oauth = if incoming_oauth_empty {
        existing.and_then(|existing| existing.oauth.clone())
    } else if server.oauth_token_ref.trim().is_empty()
        || server.oauth_client_id.trim().is_empty()
        || server.oauth_authorization_endpoint.trim().is_empty()
        || server.oauth_token_endpoint.trim().is_empty()
    {
        return Err(CoreError::Protocol(
            "oauth config requires token_ref, client_id, authorization_endpoint, and token_endpoint"
                .to_string(),
        ));
    } else {
        Some(OAuthConfig {
            issuer: None,
            authorization_endpoint: Some(server.oauth_authorization_endpoint.trim().to_string()),
            token_endpoint: Some(server.oauth_token_endpoint.trim().to_string()),
            client_id: server.oauth_client_id.trim().to_string(),
            scopes: server
                .oauth_scopes
                .iter()
                .map(|scope| scope.trim().to_string())
                .filter(|scope| !scope.is_empty())
                .collect(),
            token_ref: server.oauth_token_ref.trim().to_string(),
        })
    };

    Ok(GatewayServerConfig {
        id: server.id.clone(),
        display_name: server.display_name.clone(),
        enabled: server.enabled,
        scope,
        transport,
        timeout_secs: server
            .timeout_secs
            .or_else(|| existing.and_then(|existing| existing.timeout_secs)),
        credentials: existing
            .map(|existing| existing.credentials.clone())
            .unwrap_or_default(),
        oauth,
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

fn gateway_stdio_helper_binary_name() -> &'static str {
    if cfg!(windows) {
        "tamtri-gateway-stdio.exe"
    } else {
        "tamtri-gateway-stdio"
    }
}

fn gateway_stdio_helper_candidates(
    exe_dir: &std::path::Path,
    cwd: Option<&std::path::Path>,
    home: Option<&std::path::Path>,
) -> Vec<std::path::PathBuf> {
    let binary_name = gateway_stdio_helper_binary_name();
    let mut candidates = vec![
        exe_dir.join(binary_name),
        exe_dir.join("..").join(binary_name),
        // SwiftPM debug layout: macos/.build/<triple>/debug/Tamtri -> repo/target/debug.
        exe_dir
            .join("../../../..")
            .join("target/debug")
            .join(binary_name),
    ];
    if let Some(cwd) = cwd {
        candidates.push(cwd.join("target/debug").join(binary_name));
        candidates.push(cwd.join("../target/debug").join(binary_name));
    }
    if let Some(home) = home {
        candidates.push(home.join("Desktop/tamtri/target/debug").join(binary_name));
    }
    candidates
}

fn gateway_stdio_helper_path() -> Option<String> {
    if let Ok(path) = std::env::var("TAMTRI_GATEWAY_STDIO_HELPER")
        && std::path::Path::new(&path).is_file()
    {
        return Some(path);
    }
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let cwd = std::env::current_dir().ok();
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    gateway_stdio_helper_candidates(exe_dir, cwd.as_deref(), home.as_deref())
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
            let relative = path.strip_prefix(root).map_err(|err| {
                CoreError::Protocol(format!("workdir listing escaped root: {err}"))
            })?;
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

fn resolve_workdir_relative_path(
    workdir: &std::path::Path,
    relative_path: &str,
) -> Result<PathBuf> {
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

const TURN_WAIT_TIMEOUT_SECS: u64 = 3600;

fn signal_turn_completed(turn_waiters: &TurnWaiters, id: Id, reason: TurnEndReason) {
    if let Ok(mut waiters) = turn_waiters.lock()
        && let Some(list) = waiters.remove(&id)
    {
        for tx in list {
            let _ = tx.send(reason.clone());
        }
    }
}

fn summary_to_dto(summary: ConversationSummary) -> Result<ConversationSummaryDto> {
    Ok(ConversationSummaryDto {
        id: summary.id.to_string(),
        title: summary.title,
        updated_at: summary.updated_at.to_rfc3339(),
        project_id: Some(
            summary
                .project_id
                .unwrap_or_else(unfiled_project_id)
                .to_string(),
        ),
        active_harness_id: summary.active_harness_id,
        kind: match summary.kind {
            crate::conversation::ConversationKind::User => "user".to_string(),
            crate::conversation::ConversationKind::Example => "example".to_string(),
        },
    })
}

fn harness_usage_entry_to_dto(entry: HarnessUsageEntry) -> HarnessUsageEntryDto {
    HarnessUsageEntryDto {
        provider_id: entry.provider_id,
        display_name: entry.display_name,
        status: entry.status,
        plan_label: entry.plan_label,
        windows: entry
            .windows
            .into_iter()
            .map(|window| HarnessUsageWindowDto {
                id: window.id,
                label: window.label,
                utilization_pct: window.utilization_pct,
                resets_at: window.resets_at,
                tone: window.tone,
            })
            .collect(),
        balances: entry
            .balances
            .into_iter()
            .map(|balance| HarnessUsageBalanceDto {
                id: balance.id,
                label: balance.label,
                remaining: balance.remaining,
                unit: balance.unit,
                tone: balance.tone,
            })
            .collect(),
        error: entry.error,
        fetched_at: entry.fetched_at,
    }
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
        project_id: Some(
            conversation
                .project_id
                .unwrap_or_else(unfiled_project_id)
                .to_string(),
        ),
        active_harness_id: conversation.active_harness_id.clone(),
        model_id: conversation.model_id.clone(),
        forked_from: conversation.forked_from.map(|id| id.to_string()),
        kind: match conversation.kind {
            ConversationKind::User => "user".to_string(),
            ConversationKind::Example => "example".to_string(),
        },
        transcript_json,
    })
}

fn app_template_to_dto(template: AppTemplate) -> AppTemplateDto {
    let content_security_policy = app_sandbox_csp(&template.allowed_origins);
    AppTemplateDto {
        template_ref: template.template_ref,
        server_id: template.server_id,
        html: template.html,
        allowed_origins: template
            .allowed_origins
            .into_iter()
            .map(|origin| origin.0)
            .collect(),
        metadata_json: template.metadata.to_string(),
        bridge_script: app_bridge_bootstrap_script("tamtriAppBridge"),
        content_security_policy,
    }
}

fn root_to_dto(root: &Root) -> RootDto {
    RootDto {
        id: root.id.clone(),
        name: root.name.clone(),
        uri: root.uri.clone(),
        kind: root_kind_label(&root.kind).to_string(),
        scope: root_scope_label(&root.scope).to_string(),
        origin: Some(root_origin_label(&root.origin).to_string()),
    }
}

fn project_to_dto(project: &Project) -> ProjectDto {
    ProjectDto {
        id: project.id.to_string(),
        name: project.name.clone(),
        created_at: project.created_at.to_rfc3339(),
        updated_at: project.updated_at.to_rfc3339(),
        roots: project.roots.iter().map(root_to_dto).collect(),
    }
}

fn root_from_dto(dto: &RootDto) -> Result<Root> {
    Ok(Root {
        id: dto.id.clone(),
        name: dto.name.clone(),
        uri: dto.uri.clone(),
        kind: parse_root_kind(&dto.kind)?,
        scope: parse_root_scope(&dto.scope)?,
        origin: dto
            .origin
            .as_deref()
            .map(parse_root_origin)
            .transpose()?
            .unwrap_or_default(),
    })
}

fn roots_for_gateway(
    runtime_roots: &Mutex<HashMap<Id, Vec<Root>>>,
    conversation_id: Id,
    stored: &[Root],
) -> Vec<Root> {
    let Ok(overrides) = runtime_roots.lock() else {
        return stored.to_vec();
    };
    let Some(resolved) = overrides.get(&conversation_id) else {
        return stored.to_vec();
    };
    merge_runtime_roots(stored, resolved)
}

fn merge_runtime_roots(stored: &[Root], resolved: &[Root]) -> Vec<Root> {
    let resolved_by_id: HashMap<&str, &Root> = resolved
        .iter()
        .map(|root| (root.id.as_str(), root))
        .collect();
    stored
        .iter()
        .filter_map(|stored| {
            resolved_by_id.get(stored.id.as_str()).map(|resolved| Root {
                id: stored.id.clone(),
                name: stored.name.clone(),
                uri: resolved.uri.clone(),
                kind: stored.kind.clone(),
                scope: stored.scope.clone(),
                origin: stored.origin.clone(),
            })
        })
        .collect()
}

fn summarize_task_result(value: &serde_json::Value) -> String {
    const MAX: usize = 240;
    let summary = match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Object(map) => map
            .get("message")
            .or_else(|| map.get("summary"))
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    };
    if summary.len() <= MAX {
        summary
    } else {
        format!("{}…", &summary[..MAX])
    }
}

fn parse_root_kind(kind: &str) -> Result<RootKind> {
    match kind {
        "filesystem" => Ok(RootKind::Filesystem),
        "knowledge_base" => Ok(RootKind::KnowledgeBase),
        "other" => Ok(RootKind::Other),
        _ => Err(CoreError::MalformedVault(format!(
            "unknown root kind: {kind}"
        ))),
    }
}

fn parse_root_scope(scope: &str) -> Result<RootScope> {
    match scope {
        "conversation" => Ok(RootScope::Conversation),
        "user" => Ok(RootScope::User),
        _ => Err(CoreError::MalformedVault(format!(
            "unknown root scope: {scope}"
        ))),
    }
}

fn parse_root_origin(origin: &str) -> Result<RootOrigin> {
    match origin {
        "conversation" => Ok(RootOrigin::Conversation),
        "project" => Ok(RootOrigin::Project),
        "project_snapshot" => Ok(RootOrigin::ProjectSnapshot),
        _ => Err(CoreError::MalformedVault(format!(
            "unknown root origin: {origin}"
        ))),
    }
}

fn root_kind_label(kind: &RootKind) -> &'static str {
    match kind {
        RootKind::Filesystem => "filesystem",
        RootKind::KnowledgeBase => "knowledge_base",
        RootKind::Other => "other",
    }
}

fn root_scope_label(scope: &RootScope) -> &'static str {
    match scope {
        RootScope::Conversation => "conversation",
        RootScope::User => "user",
    }
}

fn root_origin_label(origin: &RootOrigin) -> &'static str {
    match origin {
        RootOrigin::Conversation => "conversation",
        RootOrigin::Project => "project",
        RootOrigin::ProjectSnapshot => "project_snapshot",
    }
}

/// Resolve the tamtri home directory for runtime files (credentials, relay keys).
/// Production vaults live at `~/.tamtri/vault`; tests often use a temp dir as the vault root.
fn tamtri_home_for_vault(vault_path: &Path) -> PathBuf {
    if vault_path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("vault"))
    {
        vault_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| vault_path.to_path_buf())
    } else {
        vault_path.to_path_buf()
    }
}

#[cfg(test)]
mod gateway_stdio_helper_tests {
    use super::gateway_stdio_helper_candidates;

    #[test]
    fn gateway_stdio_helper_candidates_include_swiftpm_debug_layout() {
        let exe_dir = std::path::Path::new("/repo/macos/.build/arm64-apple-macosx/debug");
        let candidates = gateway_stdio_helper_candidates(exe_dir, None, None);
        assert!(
            candidates.iter().any(|path| {
                path.ends_with("target/debug/tamtri-gateway-stdio")
                    && !path.starts_with("/repo/macos/target/debug")
            }),
            "expected repo-root target/debug candidate, got: {candidates:?}"
        );
    }
}
