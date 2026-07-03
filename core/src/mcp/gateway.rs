use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{Mutex, broadcast, mpsc};

use crate::config::{
    CredentialTarget, GatewayConfig, GatewayServerConfig, GatewayTransport, validate_app_config,
};
use crate::mcp::client::{McpClient, McpClientConfig, McpClientEvent};
use crate::mcp::protocol::{
    CallToolResult, GetPromptResult, Prompt, ReadResourceResult, Resource, Tool,
};
use crate::{CoreError, Result};

#[async_trait]
pub trait CredentialResolver: Send + Sync {
    async fn resolve(&self, credential_ref: &str) -> Result<Option<String>>;
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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayEvent {
    ServerConnected {
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
    events: Option<mpsc::UnboundedSender<GatewayEvent>>,
    event_broadcast: broadcast::Sender<GatewayEvent>,
    clients: Mutex<HashMap<String, Arc<McpClient>>>,
    routes: Mutex<HashMap<String, ToolRoute>>,
    resource_routes: Mutex<HashMap<String, ResourceRoute>>,
    prompt_routes: Mutex<HashMap<String, PromptRoute>>,
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
        Ok(Self {
            config,
            credentials,
            events,
            event_broadcast,
            clients: Mutex::new(HashMap::new()),
            routes: Mutex::new(HashMap::new()),
            resource_routes: Mutex::new(HashMap::new()),
            prompt_routes: Mutex::new(HashMap::new()),
        })
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
        for server in self.config.enabled_servers() {
            let client = self.client_for(server).await?;
            match client.list_tools().await {
                Ok(server_tools) => {
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
                    self.emit(GatewayEvent::DownstreamError {
                        server_id: server.id.clone(),
                        message: err.to_string(),
                    });
                    return Err(err);
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
        client
            .call_tool(&route.original_name, arguments, meta)
            .await
            .inspect_err(|err| {
                self.emit(GatewayEvent::DownstreamError {
                    server_id: route.server_id.clone(),
                    message: err.to_string(),
                });
            })
    }

    pub async fn list_resources(&self) -> Result<Vec<GatewayResource>> {
        let mut resources = Vec::new();
        for server in self.config.enabled_servers() {
            let client = self.client_for(server).await?;
            let server_resources = client.list_resources().await?;
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
        client.read_resource(&route.original_uri).await
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

    async fn client_for(&self, server: &GatewayServerConfig) -> Result<Arc<McpClient>> {
        if let Some(client) = self.clients.lock().await.get(&server.id).cloned() {
            return Ok(client);
        }
        let client = Arc::new(self.connect_server(server).await?);
        self.clients
            .lock()
            .await
            .insert(server.id.clone(), Arc::clone(&client));
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
            init_timeout: Duration::from_secs(30),
            call_timeout: timeout,
        };
        let client_events = self.client_event_sender(&server.id);
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
                )
                .await
            }
            GatewayTransport::StreamableHttp { endpoint, headers } => {
                let mut resolved_headers = headers.clone();
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
                )
                .await
            }
        }
    }

    fn client_event_sender(&self, server_id: &str) -> mpsc::UnboundedSender<McpClientEvent> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let events = self.events.clone();
        let server_id = server_id.to_string();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let Some(events) = &events else {
                    continue;
                };
                let gateway_event = match event {
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
                };
                let _ = events.send(gateway_event);
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
}

fn exposed_tool_name(server_id: &str, tool_name: &str) -> String {
    format!("{}__{}", slug(server_id), slug(tool_name))
}

fn exposed_resource_uri(server_id: &str, uri: &str) -> String {
    format!("tamtri://gateway/{}/{}", slug(server_id), slug(uri))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_tool_name_collision_is_stable() {
        assert_eq!(
            exposed_tool_name("my server", "Echo Tool"),
            "my_server__echo_tool"
        );
    }
}
