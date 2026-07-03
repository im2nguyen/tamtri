use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::harness::acp::AgentLaunchSpec;
use crate::{CoreError, Result};

const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_harness_id: Option<String>,
    #[serde(default)]
    pub agent_roster: Vec<AgentLaunchSpec>,
    #[serde(default)]
    pub gateway: GatewayConfig,
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

pub fn load_app_config(vault_path: &Path) -> Result<AppConfig> {
    let path = vault_path.join(CONFIG_FILE);
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let config: AppConfig = serde_json::from_slice(&std::fs::read(path)?)?;
    validate_app_config(&config)?;
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

pub fn replace_gateway_servers(
    vault_path: &Path,
    servers: Vec<GatewayServerConfig>,
) -> Result<()> {
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
        if let Some(oauth) = &server.oauth {
            if oauth.client_id.trim().is_empty() {
                return Err(CoreError::Protocol(format!(
                    "gateway server {} has an empty oauth client_id",
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

#[cfg(test)]
mod tests {
    use serde_json::json;

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
    fn registry_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let config = AppConfig {
            default_harness_id: Some("hermes-acp".to_string()),
            agent_roster: Vec::new(),
            gateway: GatewayConfig {
                default_call_timeout_secs: 120,
                servers: vec![server("mock", true)],
            },
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
    }
}
