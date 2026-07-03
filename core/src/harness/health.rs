use std::path::Path;

use crate::harness::acp::AgentLaunchSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessHealthStatus {
    Missing,
    Ready,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessHealthEntry {
    pub id: String,
    pub display_name: String,
    pub command: String,
    pub status: HarnessHealthStatus,
    pub install_doc_url: String,
}

pub fn probe_agent_launch_spec(spec: &AgentLaunchSpec) -> HarnessHealthStatus {
    let command = Path::new(&spec.command);
    if command.is_absolute() {
        if command.is_file() {
            return if is_executable(command) {
                HarnessHealthStatus::Ready
            } else {
                HarnessHealthStatus::Unknown
            };
        }
        return HarnessHealthStatus::Missing;
    }
    if which_command(&spec.command).is_some() {
        HarnessHealthStatus::Ready
    } else {
        HarnessHealthStatus::Missing
    }
}

pub fn install_doc_url(agent_id: &str) -> &'static str {
    match agent_id {
        "claude-code-acp" => "https://docs.anthropic.com/en/docs/claude-code",
        "goose" | "goose-acp" => "https://block.github.io/goose/docs/getting-started/installation/",
        "hermes" | "hermes-acp" => "https://github.com/NousResearch/hermes-agent",
        "mock-acp" => "https://github.com/tamtri/tamtri/tree/main/fixtures/mock-acp-agent",
        _ => "https://agentclientprotocol.com",
    }
}

pub fn it_admin_checklist(entries: &[HarnessHealthEntry]) -> String {
    let mut lines = vec![
        "tamtri harness setup checklist".to_string(),
        String::new(),
        "Install at least one ACP-capable agent and confirm tamtri can find its binary.".to_string(),
        "Gateway MCP servers live in <vault>/config.json; credentials stay in macOS Keychain.".to_string(),
        String::new(),
        "Configured agents:".to_string(),
    ];
    for entry in entries {
        lines.push(format!(
            "- {} ({}) — status: {:?}, command: {}",
            entry.display_name, entry.id, entry.status, entry.command
        ));
        if !entry.install_doc_url.is_empty() {
            lines.push(format!("  Install docs: {}", entry.install_doc_url));
        }
    }
    lines.join("\n")
}

pub fn health_entries_from_roster(roster: &[AgentLaunchSpec]) -> Vec<HarnessHealthEntry> {
    roster
        .iter()
        .map(|spec| HarnessHealthEntry {
            id: spec.id.clone(),
            display_name: spec.display_name.clone(),
            command: spec.command.clone(),
            status: probe_agent_launch_spec(spec),
            install_doc_url: install_doc_url(&spec.id).to_string(),
        })
        .collect()
}

fn which_command(command: &str) -> Option<std::path::PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(command);
        if candidate.is_file() && is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_health_detects_missing_ready_and_unknown() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ready_path = temp.path().join("ready-agent");
        std::fs::write(&ready_path, b"#!/bin/sh\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ready_path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }

        assert_eq!(
            probe_agent_launch_spec(&AgentLaunchSpec {
                id: "ready".into(),
                display_name: "Ready".into(),
                command: ready_path.to_string_lossy().into_owned(),
                args: Vec::new(),
                env: Vec::new(),
            }),
            HarnessHealthStatus::Ready
        );
        assert_eq!(
            probe_agent_launch_spec(&AgentLaunchSpec {
                id: "missing".into(),
                display_name: "Missing".into(),
                command: temp.path().join("nope").to_string_lossy().into_owned(),
                args: Vec::new(),
                env: Vec::new(),
            }),
            HarnessHealthStatus::Missing
        );
    }

    #[test]
    fn harness_health_checklist_copies() {
        let entries = vec![HarnessHealthEntry {
            id: "mock-acp".into(),
            display_name: "Mock".into(),
            command: "/tmp/mock".into(),
            status: HarnessHealthStatus::Missing,
            install_doc_url: install_doc_url("mock-acp").to_string(),
        }];
        let checklist = it_admin_checklist(&entries);
        assert!(checklist.contains("tamtri harness setup checklist"));
        assert!(checklist.contains("Mock"));
        assert!(checklist.contains("Install docs:"));
    }
}
