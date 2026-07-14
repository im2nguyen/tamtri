use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::harness::acp::AgentLaunchSpec;
use crate::harness::roster::{repair_agent_roster, repair_roster_spec};
use crate::{CoreError, Result};

const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_harness_id: Option<String>,
    #[serde(default)]
    pub agent_roster: Vec<AgentLaunchSpec>,
    /// Composer/settings picker order for roster entries.
    #[serde(default)]
    pub harness_order: Vec<String>,
    /// Roster ids hidden from the composer picker.
    #[serde(default)]
    pub hidden_harness_ids: Vec<String>,
    /// When true, the shell may check installed provider CLIs for updates.
    #[serde(default = "default_enable_cli_update_checks")]
    pub enable_cli_update_checks: bool,
    #[serde(default)]
    pub gateway: GatewayConfig,
}

fn default_enable_cli_update_checks() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GatewayConfig {
    pub default_call_timeout_secs: u64,
    #[serde(default)]
    pub servers: Vec<GatewayServerConfig>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            default_call_timeout_secs: 300,
            servers: Vec::new(),
        }
    }
}

impl GatewayConfig {
    pub fn enabled_servers(&self) -> impl Iterator<Item = &GatewayServerConfig> {
        self.servers.iter().filter(|server| server.enabled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OAuthConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
    pub client_id: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub token_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GatewayServerConfig {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub scope: GatewayScope,
    pub transport: GatewayTransport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub credentials: Vec<CredentialBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayScope {
    System,
    User,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum GatewayTransport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: Vec<(String, String)>,
    },
    StreamableHttp {
        endpoint: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CredentialBinding {
    pub credential_ref: String,
    pub target: CredentialTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CredentialTarget {
    EnvVar {
        name: String,
    },
    Header {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        prefix: Option<String>,
    },
}

pub fn seed_agent_roster_if_empty(vault_path: &Path) -> Result<()> {
    let mut config = load_app_config(vault_path)?;
    if !config.agent_roster.is_empty() {
        return Ok(());
    }
    if crate::harness::discovery::apply_discovery_to_config(&mut config)? {
        save_app_config(vault_path, &config)?;
    }
    Ok(())
}

/// First-run vault content: agent roster discovery and the bundled example thread.
pub fn seed_vault_content(vault_path: &Path) -> Result<()> {
    seed_agent_roster_if_empty(vault_path)?;
    let vault = crate::vault::fs::FilesystemVault::new(vault_path)?;
    crate::vault::example_seed::seed_example_conversation_if_missing(vault_path, &vault)
}

pub fn load_app_config(vault_path: &Path) -> Result<AppConfig> {
    let path = vault_path.join(CONFIG_FILE);
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let mut config: AppConfig = serde_json::from_slice(&std::fs::read(path)?)?;
    let migrated = repair_agent_roster(&mut config.agent_roster);
    validate_app_config(&config)?;
    if migrated {
        save_app_config(vault_path, &config)?;
    }
    Ok(config)
}

pub fn save_app_config(vault_path: &Path, config: &AppConfig) -> Result<()> {
    validate_app_config(config)?;
    std::fs::create_dir_all(vault_path)?;
    let path = vault_path.join(CONFIG_FILE);
    let tmp = vault_path.join(format!("{CONFIG_FILE}.tmp"));
    let bytes = serde_json::to_vec_pretty(config)?;
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn set_agent_enabled(vault_path: &Path, agent_id: &str, enabled: bool) -> Result<()> {
    let mut config = load_app_config(vault_path)?;
    let entry = config
        .agent_roster
        .iter_mut()
        .find(|spec| spec.id == agent_id)
        .ok_or_else(|| CoreError::Protocol(format!("unknown agent: {agent_id}")))?;
    entry.enabled = enabled;
    save_app_config(vault_path, &config)
}

pub fn add_agent_to_roster(vault_path: &Path, spec: AgentLaunchSpec) -> Result<()> {
    if spec.id.trim().is_empty() {
        return Err(CoreError::Protocol("agent id cannot be empty".to_string()));
    }
    let mut config = load_app_config(vault_path)?;
    if config.agent_roster.iter().any(|entry| entry.id == spec.id) {
        return Err(CoreError::Protocol(format!(
            "agent already in roster: {}",
            spec.id
        )));
    }
    let mut spec = spec;
    repair_roster_spec(&mut spec);
    config.agent_roster.push(spec);
    save_app_config(vault_path, &config)
}

pub fn replace_gateway_servers(vault_path: &Path, servers: Vec<GatewayServerConfig>) -> Result<()> {
    let mut config = load_app_config(vault_path)?;
    config.gateway.servers = servers;
    save_app_config(vault_path, &config)
}

pub fn validate_app_config(config: &AppConfig) -> Result<()> {
    let mut ids = HashSet::new();
    for server in &config.gateway.servers {
        if !ids.insert(server.id.as_str()) {
            return Err(CoreError::Protocol(format!(
                "duplicate gateway server id: {}",
                server.id
            )));
        }
        if server.id.trim().is_empty() {
            return Err(CoreError::Protocol(
                "gateway server id cannot be empty".to_string(),
            ));
        }
        for credential in &server.credentials {
            if credential.credential_ref.trim().is_empty() {
                return Err(CoreError::Protocol(format!(
                    "gateway server {} has an empty credential reference",
                    server.id
                )));
            }
        }
        validate_transport_secrets(&server.id, &server.transport)?;
        if let Some(oauth) = &server.oauth {
            if oauth.client_id.trim().is_empty() {
                return Err(CoreError::Protocol(format!(
                    "gateway server {} has an empty oauth client_id",
                    server.id
                )));
            }
            if let Some(authorization_endpoint) = oauth.authorization_endpoint.as_deref()
                && authorization_endpoint.trim().is_empty()
            {
                return Err(CoreError::Protocol(format!(
                    "gateway server {} has an empty oauth authorization_endpoint",
                    server.id
                )));
            }
            if oauth
                .token_endpoint
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(CoreError::Protocol(format!(
                    "gateway server {} has an empty oauth token_endpoint",
                    server.id
                )));
            }
            if oauth.token_ref.trim().is_empty() {
                return Err(CoreError::Protocol(format!(
                    "gateway server {} has an empty oauth token_ref",
                    server.id
                )));
            }
        }
    }
    Ok(())
}

fn validate_transport_secrets(server_id: &str, transport: &GatewayTransport) -> Result<()> {
    match transport {
        GatewayTransport::Stdio { env, .. } => {
            for (name, value) in env {
                reject_inline_secret(server_id, "stdio env", name, value)?;
            }
        }
        GatewayTransport::StreamableHttp { headers, .. } => {
            for (name, value) in headers {
                reject_inline_secret(server_id, "HTTP header", name, value)?;
            }
        }
    }
    Ok(())
}

fn reject_inline_secret(server_id: &str, location: &str, name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Ok(());
    }
    let key = name.to_ascii_lowercase();
    let secret_key = key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("api_key")
        || key == "authorization"
        || key == "proxy-authorization";
    let bearer_value = value
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("bearer ");
    if secret_key || bearer_value {
        return Err(CoreError::Protocol(format!(
            "gateway server {server_id} must not store inline secrets in {location} `{name}`; use credentials[] credential_ref bindings"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::harness::acp::{AdapterKind, AgentLaunchSpec};

    use super::*;

    fn server(id: &str, enabled: bool) -> GatewayServerConfig {
        GatewayServerConfig {
            id: id.to_string(),
            display_name: id.to_string(),
            enabled,
            scope: GatewayScope::Project,
            transport: GatewayTransport::Stdio {
                command: "mock".to_string(),
                args: Vec::new(),
                env: Vec::new(),
            },
            timeout_secs: None,
            credentials: vec![CredentialBinding {
                credential_ref: "keychain://mock".to_string(),
                target: CredentialTarget::EnvVar {
                    name: "MOCK_TOKEN".to_string(),
                },
            }],
            oauth: None,
        }
    }

    #[test]
    fn replace_gateway_servers_preserves_other_fields() {
        let dir = tempfile::tempdir().unwrap();
        let config = AppConfig {
            default_harness_id: Some("hermes-acp".to_string()),
            agent_roster: Vec::new(),
            gateway: GatewayConfig {
                default_call_timeout_secs: 120,
                servers: vec![server("old", true)],
            },
            ..AppConfig::default()
        };
        save_app_config(dir.path(), &config).unwrap();
        replace_gateway_servers(dir.path(), vec![server("new", false)]).unwrap();
        let loaded = load_app_config(dir.path()).unwrap();
        assert_eq!(loaded.default_harness_id, Some("hermes-acp".to_string()));
        assert_eq!(loaded.gateway.default_call_timeout_secs, 120);
        assert_eq!(loaded.gateway.servers.len(), 1);
        assert_eq!(loaded.gateway.servers[0].id, "new");
        assert!(!loaded.gateway.servers[0].enabled);
    }

    #[test]
    fn seed_agent_roster_if_empty_discovers_hermes() {
        let dir = tempfile::tempdir().unwrap();
        let hermes_path = dir.path().join("hermes");
        std::fs::write(&hermes_path, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hermes_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let home = dir.path().join("home");
        std::fs::create_dir_all(home.join(".local/bin")).unwrap();
        std::fs::copy(&hermes_path, home.join(".local/bin/hermes")).unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }
        seed_agent_roster_if_empty(dir.path()).unwrap();
        if let Some(value) = previous_home {
            unsafe {
                std::env::set_var("HOME", value);
            }
        } else {
            unsafe {
                std::env::remove_var("HOME");
            }
        }
        let loaded = load_app_config(dir.path()).unwrap();
        assert!(
            loaded
                .agent_roster
                .iter()
                .any(|spec| spec.id == "hermes-acp"),
            "expected hermes-acp in {:?}",
            loaded.agent_roster
        );
        assert_eq!(loaded.default_harness_id.as_deref(), Some("hermes-acp"));
    }

    #[test]
    fn load_app_config_accepts_roster_entries_without_env_or_args() {
        let dir = tempfile::tempdir().unwrap();
        let raw = r#"{
  "agent_roster": [
    {
      "id": "hermes-acp",
      "display_name": "Hermes",
      "command": "hermes",
      "args": ["acp"],
      "default_model_id": "default"
    }
  ],
  "gateway": {
    "default_call_timeout_secs": 300,
    "servers": []
  }
}"#;
        std::fs::write(dir.path().join(CONFIG_FILE), raw).unwrap();
        let config = load_app_config(dir.path()).unwrap();
        assert_eq!(config.agent_roster.len(), 1);
        assert_eq!(config.agent_roster[0].id, "hermes-acp");
        assert!(config.agent_roster[0].env.is_empty());
    }

    #[test]
    fn seed_agent_roster_if_empty_preserves_existing_roster() {
        let dir = tempfile::tempdir().unwrap();
        let config = AppConfig {
            default_harness_id: Some("mock-acp".to_string()),
            agent_roster: vec![AgentLaunchSpec {
                id: "mock-acp".into(),
                display_name: "Mock".into(),
                command: "/tmp/mock".into(),
                args: Vec::new(),
                env: Vec::new(),
                adapter: Default::default(),
                enabled: true,
            }],
            ..AppConfig::default()
        };
        save_app_config(dir.path(), &config).unwrap();
        seed_agent_roster_if_empty(dir.path()).unwrap();
        assert_eq!(load_app_config(dir.path()).unwrap(), config);
    }

    #[test]
    fn load_app_config_repairs_claude_native_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let raw = r#"{
  "agent_roster": [
    {
      "id": "claude-native",
      "display_name": "Claude Code",
      "command": "claude",
      "args": [],
      "env": [],
      "adapter": "acp",
      "enabled": true
    }
  ],
  "gateway": {
    "default_call_timeout_secs": 300,
    "servers": []
  }
}"#;
        std::fs::write(dir.path().join(CONFIG_FILE), raw).unwrap();
        let config = load_app_config(dir.path()).unwrap();
        assert_eq!(config.agent_roster[0].adapter, AdapterKind::ClaudeNative);
        let persisted = std::fs::read_to_string(dir.path().join(CONFIG_FILE)).unwrap();
        assert!(persisted.contains("claude_native"));
    }

    #[test]
    fn registry_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let config = AppConfig {
            default_harness_id: Some("hermes-acp".to_string()),
            agent_roster: Vec::new(),
            gateway: GatewayConfig {
                default_call_timeout_secs: 120,
                servers: vec![server("mock", true)],
            },
            ..AppConfig::default()
        };
        save_app_config(dir.path(), &config).unwrap();
        assert_eq!(load_app_config(dir.path()).unwrap(), config);
    }

    #[test]
    fn registry_rejects_duplicate_server_ids() {
        let config = AppConfig {
            gateway: GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![server("dup", true), server("dup", false)],
            },
            ..AppConfig::default()
        };
        assert!(matches!(
            validate_app_config(&config),
            Err(CoreError::Protocol(message)) if message.contains("duplicate")
        ));
    }

    #[test]
    fn disabled_servers_are_ignored_by_enabled_iterator() {
        let config = GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![server("on", true), server("off", false)],
        };
        assert_eq!(
            config
                .enabled_servers()
                .map(|server| server.id.as_str())
                .collect::<Vec<_>>(),
            vec!["on"]
        );
    }

    #[test]
    fn credential_refs_only_in_config() {
        let value = json!({
            "gateway": {
                "default_call_timeout_secs": 300,
                "servers": [{
                    "id": "bad",
                    "display_name": "Bad",
                    "enabled": true,
                    "scope": "project",
                    "transport": {"type": "stdio", "command": "mock"},
                    "credentials": [{
                        "credential_ref": "keychain://mock",
                        "value": "secret-inline-value",
                        "target": {"type": "env_var", "name": "MOCK_TOKEN"}
                    }]
                }]
            }
        });
        assert!(serde_json::from_value::<AppConfig>(value).is_err());

        let stdio_secret = AppConfig {
            gateway: GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![GatewayServerConfig {
                    id: "bad-env".to_string(),
                    display_name: "Bad Env".to_string(),
                    enabled: true,
                    scope: GatewayScope::Project,
                    transport: GatewayTransport::Stdio {
                        command: "mock".to_string(),
                        args: Vec::new(),
                        env: vec![("API_KEY".to_string(), "sk-live-inline".to_string())],
                    },
                    timeout_secs: None,
                    credentials: Vec::new(),
                    oauth: None,
                }],
            },
            ..AppConfig::default()
        };
        assert!(matches!(
            validate_app_config(&stdio_secret),
            Err(CoreError::Protocol(message)) if message.contains("inline secrets")
        ));

        let http_secret = AppConfig {
            gateway: GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![GatewayServerConfig {
                    id: "bad-header".to_string(),
                    display_name: "Bad Header".to_string(),
                    enabled: true,
                    scope: GatewayScope::Project,
                    transport: GatewayTransport::StreamableHttp {
                        endpoint: "http://127.0.0.1:8080/mcp".to_string(),
                        headers: vec![(
                            "Authorization".to_string(),
                            "Bearer secret-token".to_string(),
                        )],
                    },
                    timeout_secs: None,
                    credentials: Vec::new(),
                    oauth: None,
                }],
            },
            ..AppConfig::default()
        };
        assert!(matches!(
            validate_app_config(&http_secret),
            Err(CoreError::Protocol(message)) if message.contains("inline secrets")
        ));
    }
}
