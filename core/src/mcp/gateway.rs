use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::config::{
    CredentialTarget, GatewayConfig, GatewayServerConfig, GatewayTransport, OAuthConfig,
    validate_app_config,
};
use crate::conversation::{ElicitationAction, ElicitationMode, Root};
use crate::mcp::app::{
    AppTemplate, GatewayAppState, app_instance_from_tool_result, apps_enabled_for_server,
    is_ui_resource_uri, template_from_resource_contents,
};
use crate::mcp::capabilities::{
    EXT_TASKS, ServerCapabilityReport, TamtriFeatureSupport, report_from_initialize,
    tasks_available,
};
use crate::mcp::client::{ElicitationHandler, McpClient, McpClientConfig, McpClientEvent};
use crate::mcp::elicitation::{
    elicitation_mode, elicitation_request_id, origin_tool_call_id_from_meta, parse_create_params,
    result_for_action, schema_looks_secret, validate_elicitation_url,
};
use crate::mcp::gateway_tasks::{GatewayTaskTracker, parse_task_from_tool_response};
use crate::mcp::oauth::{OAuthResolveOutcome, resolve_oauth_access_token, serialize_stored_oauth};
use crate::mcp::protocol::{
    CallToolResult, GetPromptResult, MCP_PROTOCOL_VERSION, Prompt, ReadResourceResult, Resource,
    Tool,
};
use crate::mcp::roots::{RootsHandler, roots_list_result};
use crate::rpc::jsonrpc::JsonRpcError;
use crate::{CoreError, Result};

#[async_trait]
pub trait CredentialResolver: Send + Sync {
    async fn resolve(&self, credential_ref: &str) -> Result<Option<String>>;
    async fn store(&self, credential_ref: &str, value: &str) -> Result<()> {
        let _ = credential_ref;
        let _ = value;
        Err(CoreError::Protocol(
            "credential store not supported".to_string(),
        ))
    }
}

pub struct NoCredentials;

#[async_trait]
impl CredentialResolver for NoCredentials {
    async fn resolve(&self, _credential_ref: &str) -> Result<Option<String>> {
        Ok(None)
    }
}

#[derive(Default)]
pub struct MemoryCredentials {
    values: StdMutex<HashMap<String, String>>,
}

impl MemoryCredentials {
    pub fn set(&self, credential_ref: String, value: String) -> Result<()> {
        self.values
            .lock()
            .map_err(|_| CoreError::Protocol("credential resolver lock poisoned".to_string()))?
            .insert(credential_ref, value);
        Ok(())
    }

    pub fn contains(&self, credential_ref: &str) -> Result<bool> {
        Ok(self
            .values
            .lock()
            .map_err(|_| CoreError::Protocol("credential resolver lock poisoned".to_string()))?
            .contains_key(credential_ref))
    }

    pub fn get_stored(&self, credential_ref: &str) -> Result<Option<String>> {
        Ok(self
            .values
            .lock()
            .map_err(|_| CoreError::Protocol("credential resolver lock poisoned".to_string()))?
            .get(credential_ref)
            .cloned())
    }
}

#[async_trait]
impl CredentialResolver for MemoryCredentials {
    async fn resolve(&self, credential_ref: &str) -> Result<Option<String>> {
        Ok(self
            .values
            .lock()
            .map_err(|_| CoreError::Protocol("credential resolver lock poisoned".to_string()))?
            .get(credential_ref)
            .cloned())
    }

    async fn store(&self, credential_ref: &str, value: &str) -> Result<()> {
        self.set(credential_ref.to_string(), value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayEvent {
    ServerConnected {
        server_id: String,
    },
    ServerDisconnected {
        server_id: String,
    },
    ToolRouted {
        server_id: String,
        exposed_name: String,
        original_name: String,
    },
    CredentialInjected {
        server_id: String,
        credential_ref: String,
        target_kind: String,
    },
    Progress {
        server_id: String,
        params: Value,
    },
    Log {
        server_id: String,
        params: Value,
    },
    Cancellation {
        server_id: String,
        params: Value,
    },
    DownstreamError {
        server_id: String,
        message: String,
    },
    ElicitationRequested {
        origin_tool_call_id: Option<String>,
        server_id: String,
        request_id: String,
        mode: ElicitationMode,
        message: String,
        schema: Option<Value>,
        url: Option<String>,
    },
    ElicitationResolved {
        server_id: String,
        request_id: String,
        action: ElicitationAction,
    },
    AppReturned {
        origin_tool_call_id: Option<String>,
        server_id: String,
        uri: String,
        template_ref: String,
        state: Value,
    },
    OAuthHandoffStarted {
        server_id: String,
        credential_ref: String,
    },
    OAuthHandoffCompleted {
        server_id: String,
        credential_ref: String,
        status: String,
    },
    OAuthRefreshFailed {
        server_id: String,
        credential_ref: String,
    },
    CredentialUpdated {
        server_id: String,
        credential_ref: String,
        reason: String,
    },
    TaskStarted {
        state: crate::mcp::tasks::TaskState,
    },
    TaskUpdated {
        state: crate::mcp::tasks::TaskState,
    },
    TaskCompleted {
        state: crate::mcp::tasks::TaskState,
        result: Option<Value>,
    },
    RootsListed {
        server_id: String,
        count: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GatewayTool {
    pub exposed_name: String,
    pub server_id: String,
    pub original_name: String,
    pub tool: Tool,
}

pub struct McpGateway {
    config: GatewayConfig,
    credentials: Arc<dyn CredentialResolver>,
    feature_support: TamtriFeatureSupport,
    events: Option<mpsc::UnboundedSender<GatewayEvent>>,
    event_broadcast: broadcast::Sender<GatewayEvent>,
    elicitation: Arc<GatewayElicitationService>,
    app_state: Arc<Mutex<GatewayAppState>>,
    task_tracker: Arc<GatewayTaskTracker>,
    clients: Mutex<HashMap<String, Arc<McpClient>>>,
    server_call_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    routes: Mutex<HashMap<String, ToolRoute>>,
    resource_routes: Mutex<HashMap<String, ResourceRoute>>,
    prompt_routes: Mutex<HashMap<String, PromptRoute>>,
    server_capability_reports: Mutex<HashMap<String, ServerCapabilityReport>>,
    roots: Arc<tokio::sync::RwLock<Vec<Root>>>,
    agent_context: Mutex<Option<AgentGatewayContext>>,
}

#[derive(Debug, Clone)]
struct AgentGatewayContext {
    conversation_id: String,
    orchestration_enabled: bool,
}

#[derive(Debug, Clone)]
struct ActiveToolCall {
    origin_tool_call_id: Option<String>,
}

#[derive(Debug)]
struct PendingElicitation {
    server_id: String,
    response_tx: oneshot::Sender<ElicitationAction>,
    content_tx: oneshot::Sender<Option<Value>>,
}

struct GatewayElicitationService {
    events: Option<mpsc::UnboundedSender<GatewayEvent>>,
    event_broadcast: broadcast::Sender<GatewayEvent>,
    active_tool_calls: Mutex<HashMap<String, Vec<ActiveToolCall>>>,
    pending_elicitations: Mutex<HashMap<String, PendingElicitation>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolRoute {
    server_id: String,
    original_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceRoute {
    server_id: String,
    original_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptRoute {
    server_id: String,
    original_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GatewayResource {
    pub exposed_uri: String,
    pub server_id: String,
    pub original_uri: String,
    pub resource: Resource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GatewayPrompt {
    pub exposed_name: String,
    pub server_id: String,
    pub original_name: String,
    pub prompt: Prompt,
}

impl McpGateway {
    pub fn new(
        config: GatewayConfig,
        credentials: Arc<dyn CredentialResolver>,
        events: Option<mpsc::UnboundedSender<GatewayEvent>>,
    ) -> Result<Self> {
        validate_app_config(&crate::config::AppConfig {
            gateway: config.clone(),
            ..Default::default()
        })?;
        let (event_broadcast, _) = broadcast::channel(256);
        let elicitation = Arc::new(GatewayElicitationService {
            events: events.clone(),
            event_broadcast: event_broadcast.clone(),
            active_tool_calls: Mutex::new(HashMap::new()),
            pending_elicitations: Mutex::new(HashMap::new()),
        });
        Ok(Self {
            config,
            credentials,
            feature_support: TamtriFeatureSupport::current(),
            events,
            event_broadcast,
            elicitation,
            app_state: Arc::new(Mutex::new(GatewayAppState::default())),
            task_tracker: Arc::new(GatewayTaskTracker::new()),
            clients: Mutex::new(HashMap::new()),
            server_call_locks: Mutex::new(HashMap::new()),
            routes: Mutex::new(HashMap::new()),
            resource_routes: Mutex::new(HashMap::new()),
            prompt_routes: Mutex::new(HashMap::new()),
            server_capability_reports: Mutex::new(HashMap::new()),
            roots: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            agent_context: Mutex::new(None),
        })
    }

    pub async fn set_agent_context(&self, conversation_id: String, orchestration_enabled: bool) {
        *self.agent_context.lock().await = Some(AgentGatewayContext {
            conversation_id,
            orchestration_enabled,
        });
    }

    async fn agent_conversation_id(&self) -> Option<String> {
        self.agent_context
            .lock()
            .await
            .as_ref()
            .map(|ctx| ctx.conversation_id.clone())
    }

    async fn orchestration_tools_enabled(&self) -> bool {
        self.agent_context
            .lock()
            .await
            .as_ref()
            .is_some_and(|ctx| ctx.orchestration_enabled)
    }

    pub async fn set_roots(&self, roots: Vec<Root>) {
        *self.roots.write().await = roots;
    }

    pub async fn list_roots(&self) -> Result<Value> {
        let roots = self.roots.read().await.clone();
        Ok(roots_list_result(&roots))
    }

    pub async fn capability_report(&self, server_id: &str) -> Option<ServerCapabilityReport> {
        self.server_capability_reports
            .lock()
            .await
            .get(server_id)
            .cloned()
    }

    pub async fn capability_reports(&self) -> Vec<ServerCapabilityReport> {
        self.server_capability_reports
            .lock()
            .await
            .values()
            .cloned()
            .collect()
    }

    pub async fn probe_server_capabilities(&self) -> Result<Vec<ServerCapabilityReport>> {
        let mut reports = Vec::new();
        for server in self.config.enabled_servers() {
            match self.client_for(server).await {
                Ok(client) => {
                    if let Some(caps) = client.server_capabilities() {
                        let report = report_from_initialize(
                            &server.id,
                            MCP_PROTOCOL_VERSION,
                            caps,
                            self.feature_support,
                        );
                        self.store_capability_report(report.clone()).await;
                        reports.push(report);
                    }
                }
                Err(err) => {
                    self.emit(GatewayEvent::DownstreamError {
                        server_id: server.id.clone(),
                        message: err.to_string(),
                    });
                }
            }
        }
        Ok(reports)
    }

    pub async fn check_server_connection(&self, server: &GatewayServerConfig) -> Result<()> {
        self.client_for(server).await?;
        Ok(())
    }

    async fn store_capability_report(&self, report: ServerCapabilityReport) {
        self.server_capability_reports
            .lock()
            .await
            .insert(report.server_id.clone(), report);
    }

    async fn record_connected_capabilities(&self, server_id: &str, client: &McpClient) {
        let Some(caps) = client.server_capabilities() else {
            return;
        };
        let report =
            report_from_initialize(server_id, MCP_PROTOCOL_VERSION, caps, self.feature_support);
        self.store_capability_report(report).await;
    }

    pub async fn respond_elicitation(
        &self,
        request_id: &str,
        action: ElicitationAction,
        data: Option<Value>,
    ) -> Result<()> {
        self.elicitation.respond(request_id, action, data).await
    }

    pub async fn cancel_pending_elicitations(&self) {
        self.elicitation.cancel_all().await;
    }

    pub async fn disconnect_all_clients(&self) {
        let server_ids: Vec<String> = self.clients.lock().await.keys().cloned().collect();
        for server_id in server_ids {
            self.evict_client(&server_id).await;
        }
    }

    pub fn agent_cancelled(&self, params: Value) {
        self.emit(GatewayEvent::Cancellation {
            server_id: "tamtri-gateway".to_string(),
            params,
        });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<GatewayEvent> {
        self.event_broadcast.subscribe()
    }

    pub async fn list_tools(&self) -> Result<Vec<GatewayTool>> {
        let mut tools = Vec::new();
        if self.orchestration_tools_enabled().await {
            for (exposed_name, tool) in
                crate::orchestration::mcp_tools::exposed_orchestration_tools()
            {
                self.routes.lock().await.insert(
                    exposed_name.clone(),
                    ToolRoute {
                        server_id: crate::orchestration::mcp_tools::TAMTRI_SERVER_ID.to_string(),
                        original_name: tool.name.clone(),
                    },
                );
                tools.push(GatewayTool {
                    exposed_name,
                    server_id: crate::orchestration::mcp_tools::TAMTRI_SERVER_ID.to_string(),
                    original_name: tool.name.clone(),
                    tool,
                });
            }
        }
        for server in self.config.enabled_servers() {
            let client = match self.client_for(server).await {
                Ok(client) => client,
                Err(err) => {
                    self.emit(GatewayEvent::DownstreamError {
                        server_id: server.id.clone(),
                        message: err.to_string(),
                    });
                    continue;
                }
            };
            match client.list_tools().await {
                Ok(server_tools) => {
                    GatewayAppState::with_registry(&self.app_state, &server.id, |registry| {
                        GatewayAppState::index_tools(&server.id, &server_tools, registry);
                    })
                    .await;
                    for tool in server_tools {
                        let exposed_name = exposed_tool_name(&server.id, &tool.name);
                        self.routes.lock().await.insert(
                            exposed_name.clone(),
                            ToolRoute {
                                server_id: server.id.clone(),
                                original_name: tool.name.clone(),
                            },
                        );
                        tools.push(GatewayTool {
                            exposed_name,
                            server_id: server.id.clone(),
                            original_name: tool.name.clone(),
                            tool,
                        });
                    }
                }
                Err(err) => {
                    if matches!(&err, CoreError::Timeout { .. } | CoreError::TransportClosed) {
                        self.evict_client(&server.id).await;
                    }
                    self.emit(GatewayEvent::DownstreamError {
                        server_id: server.id.clone(),
                        message: err.to_string(),
                    });
                }
            }
        }
        Ok(tools)
    }

    pub async fn call_tool(&self, exposed_name: &str, arguments: Value) -> Result<CallToolResult> {
        self.call_tool_with_meta(exposed_name, arguments, None)
            .await
    }

    pub async fn call_tool_with_meta(
        &self,
        exposed_name: &str,
        arguments: Value,
        meta: Option<Value>,
    ) -> Result<CallToolResult> {
        if crate::orchestration::mcp_tools::is_native_tool(exposed_name) {
            let conversation_id = self.agent_conversation_id().await.ok_or_else(|| {
                CoreError::Protocol("native tools require an active agent conversation".to_string())
            })?;
            let core = crate::app::TamtriCore::shared().ok_or_else(|| {
                CoreError::Protocol("daemon core unavailable for native tools".to_string())
            })?;
            self.emit(GatewayEvent::ToolRouted {
                server_id: crate::orchestration::mcp_tools::TAMTRI_SERVER_ID.to_string(),
                exposed_name: exposed_name.to_string(),
                original_name: crate::orchestration::mcp_tools::native_original_name(exposed_name)
                    .unwrap_or(exposed_name)
                    .to_string(),
            });
            return core
                .handle_orchestration_tool(&conversation_id, exposed_name, arguments, meta)
                .await;
        }

        let route = {
            let routes = self.routes.lock().await;
            routes.get(exposed_name).cloned()
        };
        let route = match route {
            Some(route) => route,
            None => {
                self.list_tools().await?;
                self.routes
                    .lock()
                    .await
                    .get(exposed_name)
                    .cloned()
                    .ok_or_else(|| {
                        CoreError::Protocol(format!("unknown gateway tool: {exposed_name}"))
                    })?
            }
        };
        let server = self.server_config(&route.server_id)?;
        let client = self.client_for(server).await?;
        self.emit(GatewayEvent::ToolRouted {
            server_id: route.server_id.clone(),
            exposed_name: exposed_name.to_string(),
            original_name: route.original_name.clone(),
        });
        let lock = self.server_call_lock(&route.server_id).await;
        let _guard = lock.lock().await;
        let origin_tool_call_id = origin_tool_call_id_from_meta(meta.as_ref());
        self.elicitation
            .push_active_tool_call(&route.server_id, origin_tool_call_id.clone())
            .await;
        let tasks_enabled = client
            .server_capabilities()
            .map(|caps| tasks_available(caps, self.feature_support))
            .unwrap_or(false);
        let task_param = if tasks_enabled {
            Some(serde_json::json!({}))
        } else {
            None
        };
        let raw_result = client
            .call_tool_raw(&route.original_name, arguments, task_param, meta.clone())
            .await;
        if let Err(err) = &raw_result {
            if matches!(err, CoreError::Timeout { .. } | CoreError::TransportClosed) {
                self.evict_client(&route.server_id).await;
            }
            self.emit(GatewayEvent::DownstreamError {
                server_id: route.server_id.clone(),
                message: err.to_string(),
            });
        }
        self.elicitation
            .pop_active_tool_call(&route.server_id)
            .await;
        let result = match raw_result {
            Ok(raw) => {
                if tasks_enabled && let Some(mcp_task) = parse_task_from_tool_response(&raw) {
                    let subscribe_capable =
                        server_supports_task_subscribe(client.server_capabilities());
                    self.task_tracker
                        .register_created_task(
                            &route.server_id,
                            mcp_task,
                            origin_tool_call_id.clone(),
                            Some(route.original_name.clone()),
                            subscribe_capable,
                            self.emit_fn(),
                            Arc::clone(&client),
                        )
                        .await;
                }
                if raw.get("task").is_some() {
                    Ok(CallToolResult {
                        content: vec![serde_json::json!({
                            "type": "text",
                            "text": "Task started"
                        })],
                        is_error: Some(false),
                        structured_content: Some(raw),
                    })
                } else {
                    Ok(serde_json::from_value(raw)?)
                }
            }
            Err(err) => Err(err),
        };
        if let Ok(ref tool_result) = result
            && apps_enabled_for_server(client.server_capabilities(), self.feature_support)
            && let Some(template_ref) =
                GatewayAppState::with_registry(&self.app_state, &route.server_id, |registry| {
                    registry
                        .tool_resource_uri(&route.original_name)
                        .map(str::to_string)
                })
                .await
        {
            if let Err(err) = self
                .ensure_app_template(&route.server_id, &template_ref, client.as_ref())
                .await
            {
                self.emit(GatewayEvent::DownstreamError {
                    server_id: route.server_id.clone(),
                    message: err.to_string(),
                });
            } else {
                let instance = app_instance_from_tool_result(
                    &route.server_id,
                    &template_ref,
                    tool_result,
                    origin_tool_call_id.clone(),
                );
                self.emit(GatewayEvent::AppReturned {
                    origin_tool_call_id: instance.origin_tool_call_id.clone(),
                    server_id: instance.server_id.clone(),
                    uri: instance.uri.clone(),
                    template_ref: instance.template_ref.clone(),
                    state: instance.state.clone(),
                });
            }
        }
        result
    }

    async fn ensure_app_template(
        &self,
        server_id: &str,
        template_ref: &str,
        client: &McpClient,
    ) -> Result<()> {
        if self
            .cached_app_template(server_id, template_ref)
            .await
            .is_some()
        {
            return Ok(());
        }
        let declared = GatewayAppState::with_registry(&self.app_state, server_id, |registry| {
            registry.is_declared(template_ref)
        })
        .await;
        if !declared {
            return Err(CoreError::Protocol(format!(
                "undeclared app template: {template_ref}"
            )));
        }
        let read = client.read_resource(template_ref).await?;
        let template = template_from_resource_contents(server_id, template_ref, &read.contents)?;
        GatewayAppState::with_registry(&self.app_state, server_id, |registry| {
            registry.insert_template(template);
        })
        .await;
        Ok(())
    }

    pub async fn cached_app_template(
        &self,
        server_id: &str,
        template_ref: &str,
    ) -> Option<AppTemplate> {
        GatewayAppState::with_registry(&self.app_state, server_id, |registry| {
            registry.template(template_ref).cloned()
        })
        .await
    }

    async fn server_call_lock(&self, server_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.server_call_locks.lock().await;
        if let Some(existing) = locks.get(server_id).cloned() {
            return existing;
        }
        let lock = Arc::new(Mutex::new(()));
        locks.insert(server_id.to_string(), Arc::clone(&lock));
        lock
    }

    pub async fn list_resources(&self) -> Result<Vec<GatewayResource>> {
        let mut resources = Vec::new();
        for server in self.config.enabled_servers() {
            let client = self.client_for(server).await?;
            let server_resources = client.list_resources().await?;
            GatewayAppState::with_registry(&self.app_state, &server.id, |registry| {
                GatewayAppState::index_resources(&server_resources, registry)
            })
            .await;
            for resource in server_resources {
                let exposed_uri = exposed_resource_uri(&server.id, &resource.uri);
                self.resource_routes.lock().await.insert(
                    exposed_uri.clone(),
                    ResourceRoute {
                        server_id: server.id.clone(),
                        original_uri: resource.uri.clone(),
                    },
                );
                resources.push(GatewayResource {
                    exposed_uri,
                    server_id: server.id.clone(),
                    original_uri: resource.uri.clone(),
                    resource,
                });
            }
        }
        Ok(resources)
    }

    pub async fn read_resource(&self, exposed_uri: &str) -> Result<ReadResourceResult> {
        let route = {
            let routes = self.resource_routes.lock().await;
            routes.get(exposed_uri).cloned()
        };
        let route = match route {
            Some(route) => route,
            None => {
                self.list_resources().await?;
                self.resource_routes
                    .lock()
                    .await
                    .get(exposed_uri)
                    .cloned()
                    .ok_or_else(|| {
                        CoreError::Protocol(format!("unknown gateway resource: {exposed_uri}"))
                    })?
            }
        };
        let server = self.server_config(&route.server_id)?;
        let client = self.client_for(server).await?;
        let read = client.read_resource(&route.original_uri).await?;
        if is_ui_resource_uri(&route.original_uri)
            && apps_enabled_for_server(client.server_capabilities(), self.feature_support)
        {
            let template = template_from_resource_contents(
                &route.server_id,
                &route.original_uri,
                &read.contents,
            )?;
            GatewayAppState::with_registry(&self.app_state, &route.server_id, |registry| {
                registry.insert_template(template)
            })
            .await;
            self.emit(GatewayEvent::AppReturned {
                origin_tool_call_id: None,
                server_id: route.server_id.clone(),
                uri: route.original_uri.clone(),
                template_ref: route.original_uri.clone(),
                state: serde_json::json!({}),
            });
        }
        Ok(read)
    }

    pub async fn list_prompts(&self) -> Result<Vec<GatewayPrompt>> {
        let mut prompts = Vec::new();
        for server in self.config.enabled_servers() {
            let client = self.client_for(server).await?;
            let server_prompts = client.list_prompts().await?;
            for prompt in server_prompts {
                let exposed_name = exposed_tool_name(&server.id, &prompt.name);
                self.prompt_routes.lock().await.insert(
                    exposed_name.clone(),
                    PromptRoute {
                        server_id: server.id.clone(),
                        original_name: prompt.name.clone(),
                    },
                );
                prompts.push(GatewayPrompt {
                    exposed_name,
                    server_id: server.id.clone(),
                    original_name: prompt.name.clone(),
                    prompt,
                });
            }
        }
        Ok(prompts)
    }

    pub async fn get_prompt(
        &self,
        exposed_name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult> {
        let route = {
            let routes = self.prompt_routes.lock().await;
            routes.get(exposed_name).cloned()
        };
        let route = match route {
            Some(route) => route,
            None => {
                self.list_prompts().await?;
                self.prompt_routes
                    .lock()
                    .await
                    .get(exposed_name)
                    .cloned()
                    .ok_or_else(|| {
                        CoreError::Protocol(format!("unknown gateway prompt: {exposed_name}"))
                    })?
            }
        };
        let server = self.server_config(&route.server_id)?;
        let client = self.client_for(server).await?;
        client.get_prompt(&route.original_name, arguments).await
    }

    fn server_config(&self, server_id: &str) -> Result<&GatewayServerConfig> {
        self.config
            .servers
            .iter()
            .find(|server| server.id == server_id && server.enabled)
            .ok_or_else(|| CoreError::Protocol(format!("gateway server not found: {server_id}")))
    }

    async fn evict_client(&self, server_id: &str) {
        let removed = if let Some(client) = self.clients.lock().await.remove(server_id) {
            if let Ok(client) = Arc::try_unwrap(client) {
                let _ = client.close().await;
            }
            true
        } else {
            false
        };
        self.task_tracker.unregister_client(server_id).await;
        if removed {
            self.emit(GatewayEvent::ServerDisconnected {
                server_id: server_id.to_string(),
            });
        }
    }

    async fn client_for(&self, server: &GatewayServerConfig) -> Result<Arc<McpClient>> {
        if let Some(client) = self.clients.lock().await.get(&server.id).cloned() {
            return Ok(client);
        }
        let client = Arc::new(self.connect_server(server).await?);
        self.record_connected_capabilities(&server.id, client.as_ref())
            .await;
        self.clients
            .lock()
            .await
            .insert(server.id.clone(), Arc::clone(&client));
        self.task_tracker
            .register_client(&server.id, Arc::clone(&client))
            .await;
        self.emit(GatewayEvent::ServerConnected {
            server_id: server.id.clone(),
        });
        Ok(client)
    }

    async fn connect_server(&self, server: &GatewayServerConfig) -> Result<McpClient> {
        let timeout = Duration::from_secs(
            server
                .timeout_secs
                .unwrap_or(self.config.default_call_timeout_secs),
        );
        let client_config = McpClientConfig {
            init_timeout: server
                .timeout_secs
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(30)),
            call_timeout: timeout,
        };
        let client_events = self.client_event_sender(&server.id);
        let elicitation_handler = Arc::new(GatewayElicitationHandler {
            server_id: server.id.clone(),
            service: Arc::clone(&self.elicitation),
        });
        let roots_handler = Arc::new(GatewayRootsHandler {
            server_id: server.id.clone(),
            roots: Arc::clone(&self.roots),
            events: self.events.clone(),
            event_broadcast: self.event_broadcast.clone(),
        });
        match &server.transport {
            GatewayTransport::Stdio { command, args, env } => {
                let mut resolved_env = env.clone();
                for credential in &server.credentials {
                    if let CredentialTarget::EnvVar { name } = &credential.target
                        && let Some(value) =
                            self.credentials.resolve(&credential.credential_ref).await?
                    {
                        resolved_env.push((name.clone(), value));
                        self.emit(GatewayEvent::CredentialInjected {
                            server_id: server.id.clone(),
                            credential_ref: credential.credential_ref.clone(),
                            target_kind: "env_var".to_string(),
                        });
                    }
                }
                McpClient::connect_stdio_with_events(
                    command,
                    args,
                    &resolved_env,
                    client_config,
                    client_events,
                    Some(elicitation_handler),
                    Some(roots_handler),
                )
                .await
            }
            GatewayTransport::StreamableHttp { endpoint, headers } => {
                let mut resolved_headers = headers.clone();
                if let Some(oauth) = &server.oauth {
                    self.inject_oauth_header(&server.id, oauth, &mut resolved_headers)
                        .await?;
                }
                for credential in &server.credentials {
                    if let CredentialTarget::Header { name, prefix } = &credential.target
                        && let Some(value) =
                            self.credentials.resolve(&credential.credential_ref).await?
                    {
                        let value = match prefix {
                            Some(prefix) => format!("{prefix}{value}"),
                            None => value,
                        };
                        resolved_headers.push((name.clone(), value));
                        self.emit(GatewayEvent::CredentialInjected {
                            server_id: server.id.clone(),
                            credential_ref: credential.credential_ref.clone(),
                            target_kind: "header".to_string(),
                        });
                    }
                }
                McpClient::connect_http_with_events(
                    endpoint,
                    &resolved_headers,
                    client_config,
                    client_events,
                    Some(elicitation_handler),
                    Some(roots_handler),
                )
                .await
            }
        }
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        self.task_tracker.cancel_task(task_id, self.emit_fn()).await
    }

    pub async fn suspend_task_polling(&self, task_id: &str) {
        self.task_tracker.suspend_polling(task_id).await;
    }

    pub async fn resume_task_polling(&self, task_id: &str) -> Result<()> {
        self.task_tracker
            .resume_polling(task_id, self.emit_fn())
            .await
    }

    fn emit_fn(&self) -> Arc<dyn Fn(GatewayEvent) + Send + Sync> {
        let events = self.events.clone();
        let broadcast = self.event_broadcast.clone();
        Arc::new(move |event| {
            let _ = broadcast.send(event.clone());
            if let Some(tx) = &events {
                let _ = tx.send(event);
            }
        })
    }

    fn client_event_sender(&self, server_id: &str) -> mpsc::UnboundedSender<McpClientEvent> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let events = self.events.clone();
        let task_tracker = Arc::clone(&self.task_tracker);
        let emit = self.emit_fn();
        let server_id = server_id.to_string();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    McpClientEvent::TaskStatus { params } => {
                        if let Some(client) = task_tracker.client_for(&server_id).await {
                            task_tracker
                                .handle_status_notification(
                                    &server_id,
                                    &params,
                                    Arc::clone(&emit),
                                    client,
                                )
                                .await;
                        }
                    }
                    other => {
                        let Some(events) = &events else {
                            continue;
                        };
                        let gateway_event = match other {
                            McpClientEvent::Progress { params } => GatewayEvent::Progress {
                                server_id: server_id.clone(),
                                params,
                            },
                            McpClientEvent::Log { params } => GatewayEvent::Log {
                                server_id: server_id.clone(),
                                params,
                            },
                            McpClientEvent::Cancelled { params } => GatewayEvent::Cancellation {
                                server_id: server_id.clone(),
                                params,
                            },
                            McpClientEvent::TaskStatus { .. } => unreachable!(),
                        };
                        let _ = events.send(gateway_event);
                    }
                }
            }
        });
        tx
    }

    fn emit(&self, event: GatewayEvent) {
        let _ = self.event_broadcast.send(event.clone());
        if let Some(tx) = &self.events {
            let _ = tx.send(event);
        }
    }

    async fn inject_oauth_header(
        &self,
        server_id: &str,
        oauth: &OAuthConfig,
        resolved_headers: &mut Vec<(String, String)>,
    ) -> Result<()> {
        let Some(stored_raw) = self.credentials.resolve(&oauth.token_ref).await? else {
            self.emit(GatewayEvent::DownstreamError {
                server_id: server_id.to_string(),
                message: format!("oauth token missing for {server_id}; connect in settings"),
            });
            return Ok(());
        };
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| CoreError::Protocol(format!("oauth http client failed: {err}")))?;
        let credentials = Arc::clone(&self.credentials);
        let token_ref = oauth.token_ref.clone();
        let oauth_config = oauth.clone();
        let (outcome, updated_bundle) =
            resolve_oauth_access_token(&client, &oauth_config, &stored_raw).await?;
        if let Some(bundle) = updated_bundle {
            credentials
                .store(&token_ref, &serialize_stored_oauth(&bundle)?)
                .await?;
            self.emit(GatewayEvent::CredentialUpdated {
                server_id: server_id.to_string(),
                credential_ref: token_ref.clone(),
                reason: "oauth_refresh".to_string(),
            });
        }
        match outcome {
            OAuthResolveOutcome::AccessToken(token) => {
                resolved_headers.push(("Authorization".to_string(), format!("Bearer {token}")));
                self.emit(GatewayEvent::CredentialInjected {
                    server_id: server_id.to_string(),
                    credential_ref: oauth.token_ref.clone(),
                    target_kind: "oauth_bearer".to_string(),
                });
            }
            OAuthResolveOutcome::ReauthRequired => {
                self.emit(GatewayEvent::OAuthRefreshFailed {
                    server_id: server_id.to_string(),
                    credential_ref: oauth.token_ref.clone(),
                });
                self.emit(GatewayEvent::DownstreamError {
                    server_id: server_id.to_string(),
                    message: format!("oauth re-authentication required for {server_id}"),
                });
            }
            OAuthResolveOutcome::Missing => {
                self.emit(GatewayEvent::DownstreamError {
                    server_id: server_id.to_string(),
                    message: format!("oauth token missing for {server_id}; connect in settings"),
                });
            }
        }
        Ok(())
    }
}

fn server_supports_task_subscribe(
    capabilities: Option<&crate::mcp::protocol::ServerCapabilities>,
) -> bool {
    let Some(caps) = capabilities else {
        return false;
    };
    caps.extensions.as_ref().is_some_and(|extensions| {
        extensions
            .get(EXT_TASKS)
            .and_then(|value| value.get("subscribe"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    })
}

fn exposed_tool_name(server_id: &str, tool_name: &str) -> String {
    format!("{}__{}", slug(server_id), slug(tool_name))
}

pub fn gateway_exposed_tool_name(server_id: &str, tool_name: &str) -> String {
    exposed_tool_name(server_id, tool_name)
}

fn exposed_resource_uri(server_id: &str, uri: &str) -> String {
    format!("tamtri://gateway/{}/{}", slug(server_id), slug(uri))
}

pub fn gateway_exposed_resource_uri(server_id: &str, uri: &str) -> String {
    exposed_resource_uri(server_id, uri)
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

struct GatewayElicitationHandler {
    server_id: String,
    service: Arc<GatewayElicitationService>,
}

struct GatewayRootsHandler {
    server_id: String,
    roots: Arc<tokio::sync::RwLock<Vec<Root>>>,
    events: Option<mpsc::UnboundedSender<GatewayEvent>>,
    event_broadcast: broadcast::Sender<GatewayEvent>,
}

#[async_trait]
impl RootsHandler for GatewayRootsHandler {
    async fn handle_list(&self) -> std::result::Result<Value, JsonRpcError> {
        let roots = self.roots.read().await;
        let count = roots.len();
        let event = GatewayEvent::RootsListed {
            server_id: self.server_id.clone(),
            count,
        };
        let _ = self.event_broadcast.send(event.clone());
        if let Some(tx) = &self.events {
            let _ = tx.send(event);
        }
        Ok(roots_list_result(&roots))
    }
}

#[async_trait]
impl ElicitationHandler for GatewayElicitationHandler {
    async fn handle_create(&self, params: Value) -> std::result::Result<Value, JsonRpcError> {
        self.service.handle_create(&self.server_id, params).await
    }
}

impl GatewayElicitationService {
    fn emit(&self, event: GatewayEvent) {
        let _ = self.event_broadcast.send(event.clone());
        if let Some(tx) = &self.events {
            let _ = tx.send(event);
        }
    }

    async fn push_active_tool_call(&self, server_id: &str, origin_tool_call_id: Option<String>) {
        self.active_tool_calls
            .lock()
            .await
            .entry(server_id.to_string())
            .or_default()
            .push(ActiveToolCall {
                origin_tool_call_id,
            });
    }

    async fn pop_active_tool_call(&self, server_id: &str) {
        let mut map = self.active_tool_calls.lock().await;
        if let Some(stack) = map.get_mut(server_id) {
            stack.pop();
            if stack.is_empty() {
                map.remove(server_id);
            }
        }
    }

    async fn respond(
        &self,
        request_id: &str,
        action: ElicitationAction,
        data: Option<Value>,
    ) -> Result<()> {
        let pending = {
            let mut map = self.pending_elicitations.lock().await;
            map.remove(request_id)
        };
        let Some(pending) = pending else {
            return Err(CoreError::Protocol(format!(
                "unknown elicitation request: {request_id}"
            )));
        };
        if matches!(action, ElicitationAction::Accept)
            && let Some(data) = data
        {
            let _ = pending.content_tx.send(Some(data));
        } else {
            let _ = pending.content_tx.send(None);
        }
        pending
            .response_tx
            .send(action.clone())
            .map_err(|_| CoreError::Protocol("elicitation waiter dropped".to_string()))?;
        self.emit(GatewayEvent::ElicitationResolved {
            server_id: pending.server_id,
            request_id: request_id.to_string(),
            action,
        });
        Ok(())
    }

    async fn cancel_all(&self) {
        let pending: Vec<_> = self.pending_elicitations.lock().await.drain().collect();
        for (request_id, entry) in pending {
            let _ = entry.content_tx.send(None);
            let _ = entry.response_tx.send(ElicitationAction::Cancel);
            self.emit(GatewayEvent::ElicitationResolved {
                server_id: entry.server_id,
                request_id,
                action: ElicitationAction::Cancel,
            });
        }
    }

    async fn handle_create(
        &self,
        server_id: &str,
        params: Value,
    ) -> std::result::Result<Value, JsonRpcError> {
        let parsed = parse_create_params(params).map_err(|err| JsonRpcError {
            code: -32602,
            message: err.to_string(),
            data: None,
        })?;
        let mode = elicitation_mode(&parsed);
        if matches!(mode, ElicitationMode::Form)
            && let Some(schema) = parsed.requested_schema.as_ref()
            && schema_looks_secret(schema)
        {
            return Ok(result_for_action(ElicitationAction::Decline, None));
        }
        if matches!(mode, ElicitationMode::Url) {
            let Some(url) = parsed.url.as_deref() else {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "url elicitation requires url".to_string(),
                    data: None,
                });
            };
            validate_elicitation_url(url).map_err(|err| JsonRpcError {
                code: -32602,
                message: err.to_string(),
                data: None,
            })?;
        }
        let request_id = elicitation_request_id(&parsed, &Uuid::now_v7().to_string());
        let origin_tool_call_id = self
            .active_tool_calls
            .lock()
            .await
            .get(server_id)
            .and_then(|stack| stack.last())
            .and_then(|call| call.origin_tool_call_id.clone());
        let (response_tx, response_rx) = oneshot::channel();
        let (content_tx, content_rx) = oneshot::channel();
        self.pending_elicitations.lock().await.insert(
            request_id.clone(),
            PendingElicitation {
                server_id: server_id.to_string(),
                response_tx,
                content_tx,
            },
        );
        self.emit(GatewayEvent::ElicitationRequested {
            origin_tool_call_id,
            server_id: server_id.to_string(),
            request_id: request_id.clone(),
            mode: mode.clone(),
            message: parsed.message.clone(),
            schema: parsed.requested_schema.clone(),
            url: parsed.url.clone(),
        });
        let action = response_rx.await.unwrap_or(ElicitationAction::Cancel);
        let content = if matches!(action, ElicitationAction::Accept) {
            content_rx.await.ok().flatten()
        } else {
            None
        };
        Ok(result_for_action(action, content))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::config::GatewayScope;
    use serde_json::json;
    use tokio::sync::mpsc;

    fn stdio_server(id: &str, command: &str) -> GatewayServerConfig {
        GatewayServerConfig {
            id: id.to_string(),
            display_name: id.to_string(),
            enabled: true,
            scope: GatewayScope::Project,
            transport: GatewayTransport::Stdio {
                command: command.to_string(),
                args: Vec::new(),
                env: Vec::new(),
            },
            timeout_secs: None,
            credentials: Vec::new(),
            oauth: None,
        }
    }

    #[tokio::test]
    async fn gateway_tool_name_collision_is_stable() {
        assert_eq!(
            exposed_tool_name("my server", "Echo Tool"),
            "my_server__echo_tool"
        );

        let Some(command) = option_env!("CARGO_BIN_EXE_mock-mcp-server") else {
            return;
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        let gateway = Arc::new(
            McpGateway::new(
                GatewayConfig {
                    default_call_timeout_secs: 300,
                    servers: vec![
                        stdio_server("alpha", command),
                        stdio_server("beta", command),
                    ],
                },
                Arc::new(NoCredentials),
                Some(tx),
            )
            .unwrap(),
        );

        let tools = gateway.list_tools().await.unwrap();
        let alpha_echo = tools
            .iter()
            .find(|tool| tool.server_id == "alpha" && tool.original_name == "echo")
            .expect("alpha echo");
        let beta_echo = tools
            .iter()
            .find(|tool| tool.server_id == "beta" && tool.original_name == "echo")
            .expect("beta echo");
        assert_ne!(alpha_echo.exposed_name, beta_echo.exposed_name);
        assert_eq!(alpha_echo.exposed_name, "alpha__echo");
        assert_eq!(beta_echo.exposed_name, "beta__echo");

        gateway
            .call_tool(
                &alpha_echo.exposed_name,
                json!({"server": "alpha", "message": "alpha"}),
            )
            .await
            .unwrap();
        let alpha_routed = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(GatewayEvent::ToolRouted { server_id, .. }) = rx.recv().await
                    && server_id == "alpha"
                {
                    return server_id;
                }
            }
        })
        .await
        .expect("alpha route event");

        gateway
            .call_tool(
                &beta_echo.exposed_name,
                json!({"server": "beta", "message": "beta"}),
            )
            .await
            .unwrap();
        let beta_routed = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(GatewayEvent::ToolRouted { server_id, .. }) = rx.recv().await
                    && server_id == "beta"
                {
                    return server_id;
                }
            }
        })
        .await
        .expect("beta route event");

        assert_eq!(alpha_routed, "alpha");
        assert_eq!(beta_routed, "beta");
    }

    #[tokio::test]
    async fn malformed_elicitation_params_return_invalid_params_error() {
        let (event_broadcast, _) = broadcast::channel(8);
        let service = Arc::new(GatewayElicitationService {
            events: None,
            event_broadcast,
            active_tool_calls: Mutex::new(HashMap::new()),
            pending_elicitations: Mutex::new(HashMap::new()),
        });
        let err = service
            .handle_create("mock", serde_json::json!({"mode": 5}))
            .await
            .expect_err("expected invalid params");
        assert_eq!(err.code, -32602);
    }
}
