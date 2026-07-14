//! Roster discovery sync and installed CLI inventory.

use std::path::Path;

use crate::harness::acp::{AdapterKind, AgentLaunchSpec};
use crate::harness::auth::{claude_auth_ready, codex_credentials_present};
use crate::harness::health::{HarnessHealthStatus, discover_known_agents, install_doc_url, probe_agent_launch_spec};
use crate::harness::roster::repair_roster_spec;
use crate::{CoreError, Result};
use crate::config::{load_app_config, save_app_config, AppConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledCli {
    pub id: String,
    pub display_name: String,
    pub command: String,
    pub version: Option<String>,
    pub install_doc_url: String,
    pub in_roster: bool,
    pub auth_ready: bool,
}

const KNOWN_CLI_COMMANDS: &[(&str, &str, &str)] = &[
    ("claude", "claude", "claude-native"),
    ("codex", "codex", "codex-native"),
    ("cursor-agent", "Cursor Agent", "cursor-agent"),
    ("opencode", "OpenCode", "opencode-native"),
    ("pi", "Pi", "pi-native"),
    ("pi-acp", "Pi (ACP)", "pi-acp"),
    ("hermes", "Hermes", "hermes-acp"),
    ("goose", "Goose", "goose-acp"),
    ("gemini", "Gemini", "gemini"),
];

/// Merge freshly discovered harness agents into the vault roster.
pub fn sync_agent_roster_with_discovery(vault_path: &Path) -> Result<()> {
    let mut config = load_app_config(vault_path)?;
    if apply_discovery_to_config(&mut config)? {
        save_app_config(vault_path, &config)?;
    }
    Ok(())
}

pub fn apply_discovery_to_config(config: &mut AppConfig) -> Result<bool> {
    let discovered = discover_known_agents();
    let mut changed = false;
    for mut spec in discovered {
        repair_roster_spec(&mut spec);
        if let Some(existing) = config
            .agent_roster
            .iter_mut()
            .find(|entry| entry.id == spec.id)
        {
            if existing.command != spec.command {
                existing.command = spec.command.clone();
                changed = true;
            }
            continue;
        }
        spec.enabled = should_auto_enable(&spec, &config.agent_roster);
        if !config.harness_order.iter().any(|id| id == &spec.id) {
            config.harness_order.push(spec.id.clone());
        }
        config.agent_roster.push(spec);
        changed = true;
    }
    if config.default_harness_id.is_none() {
        if let Some(spec) = config.agent_roster.iter().find(|entry| entry.enabled) {
            config.default_harness_id = Some(spec.id.clone());
            changed = true;
        } else if let Some(spec) = config.agent_roster.first() {
            config.default_harness_id = Some(spec.id.clone());
            changed = true;
        }
    }
    Ok(changed)
}

pub fn list_installed_clis(roster: &[AgentLaunchSpec]) -> Vec<InstalledCli> {
    let roster_ids: std::collections::HashSet<_> = roster.iter().map(|spec| spec.id.as_str()).collect();
    KNOWN_CLI_COMMANDS
        .iter()
        .filter_map(|(command, display_name, roster_id)| {
            let resolved = resolve_command(command)?;
            Some(InstalledCli {
                id: roster_id.to_string(),
                display_name: (*display_name).to_string(),
                command: resolved,
                version: probe_cli_version(command),
                install_doc_url: install_doc_url(roster_id).to_string(),
                in_roster: roster_ids.contains(roster_id),
                auth_ready: cli_auth_ready(command),
            })
        })
        .collect()
}

pub fn sort_roster_by_picker_order(mut specs: Vec<AgentLaunchSpec>, order: &[String]) -> Vec<AgentLaunchSpec> {
    if order.is_empty() {
        specs.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        return specs;
    }
    specs.sort_by(|left, right| {
        picker_rank(order, &left.id).cmp(&picker_rank(order, &right.id))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    specs
}

pub fn update_picker_settings(
    vault_path: &Path,
    harness_order: Vec<String>,
    hidden_harness_ids: Vec<String>,
    enable_cli_update_checks: bool,
) -> Result<()> {
    let mut config = load_app_config(vault_path)?;
    let roster_ids: std::collections::HashSet<_> =
        config.agent_roster.iter().map(|spec| spec.id.clone()).collect();
    for id in &harness_order {
        if !roster_ids.contains(id) {
            return Err(CoreError::Protocol(format!("unknown harness in picker order: {id}")));
        }
    }
    for id in &hidden_harness_ids {
        if !roster_ids.contains(id) {
            return Err(CoreError::Protocol(format!("unknown hidden harness: {id}")));
        }
    }
    config.harness_order = harness_order;
    config.hidden_harness_ids = hidden_harness_ids;
    config.enable_cli_update_checks = enable_cli_update_checks;
    save_app_config(vault_path, &config)
}

fn picker_rank(order: &[String], id: &str) -> usize {
    order.iter().position(|entry| entry == id).unwrap_or(usize::MAX)
}

fn should_auto_enable(spec: &AgentLaunchSpec, roster: &[AgentLaunchSpec]) -> bool {
    if probe_agent_launch_spec(spec) != HarnessHealthStatus::Ready {
        return false;
    }
    match spec.id.as_str() {
        "claude-native" => claude_auth_ready(&spec.command),
        "codex-native" => codex_credentials_present(),
        "claude-code-acp" => {
            !roster.iter().any(|entry| entry.id == "claude-native")
                && claude_auth_ready(&spec.command)
        }
        _ if spec.id.contains("codex") => false,
        _ if spec.id.contains("claude") => claude_auth_ready(&spec.command),
        _ => true,
    }
}

fn cli_auth_ready(command: &str) -> bool {
    match command {
        "claude" => claude_auth_ready(command),
        "codex" => codex_credentials_present(),
        _ => probe_agent_launch_spec(&AgentLaunchSpec {
            id: command.into(),
            display_name: command.into(),
            command: resolve_command(command).unwrap_or_else(|| command.into()),
            args: Vec::new(),
            env: Vec::new(),
            adapter: AdapterKind::Acp,
            enabled: true,
        }) == HarnessHealthStatus::Ready,
    }
}

fn resolve_command(command: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn probe_cli_version(command: &str) -> Option<String> {
    let resolved = resolve_command(command)?;
    let output = std::process::Command::new(&resolved)
        .arg("--version")
        .env_clear()
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::acp::AdapterKind;
    use tempfile::TempDir;

    #[test]
    fn should_auto_enable_claude_native_when_signed_in() {
        let spec = AgentLaunchSpec {
            id: "claude-native".into(),
            display_name: "Claude".into(),
            command: "/nonexistent/claude".into(),
            args: Vec::new(),
            env: Vec::new(),
            adapter: AdapterKind::ClaudeNative,
            enabled: false,
        };
        assert!(!should_auto_enable(&spec, &[]));
    }

    #[test]
    fn sync_preserves_existing_roster_entries() {
        let dir = TempDir::new().expect("tempdir");
        let mut config = AppConfig::default();
        config.agent_roster.push(AgentLaunchSpec {
            id: "claude-native".into(),
            display_name: "Claude".into(),
            command: "claude".into(),
            args: Vec::new(),
            env: Vec::new(),
            adapter: AdapterKind::ClaudeNative,
            enabled: false,
        });
        save_app_config(dir.path(), &config).expect("save");
        sync_agent_roster_with_discovery(dir.path()).expect("sync");
        let loaded = load_app_config(dir.path()).expect("load");
        assert_eq!(loaded.agent_roster.len(), 1);
        assert!(!loaded.agent_roster[0].enabled);
    }

    #[test]
    fn sort_roster_by_picker_order_respects_custom_order() {
        let specs = vec![
            AgentLaunchSpec {
                id: "b".into(),
                display_name: "B".into(),
                command: "b".into(),
                args: Vec::new(),
                env: Vec::new(),
                adapter: AdapterKind::Acp,
                enabled: true,
            },
            AgentLaunchSpec {
                id: "a".into(),
                display_name: "A".into(),
                command: "a".into(),
                args: Vec::new(),
                env: Vec::new(),
                adapter: AdapterKind::Acp,
                enabled: true,
            },
        ];
        let sorted = sort_roster_by_picker_order(specs, &["a".into(), "b".into()]);
        assert_eq!(sorted[0].id, "a");
        assert_eq!(sorted[1].id, "b");
    }
}
