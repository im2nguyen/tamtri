use std::path::Path;

use crate::CoreError;
use crate::Result;
use crate::harness::acp::{AdapterKind, AgentLaunchSpec};

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
    probe_agent_launch_spec_result(spec).unwrap_or(HarnessHealthStatus::Unknown)
}

pub fn probe_agent_launch_spec_result(spec: &AgentLaunchSpec) -> Result<HarnessHealthStatus> {
    let command = Path::new(&spec.command);
    if command.is_absolute() {
        match command.metadata() {
            Ok(meta) if meta.is_file() => {
                if is_executable(command) {
                    Ok(HarnessHealthStatus::Ready)
                } else {
                    Ok(HarnessHealthStatus::Unknown)
                }
            }
            Ok(_) => Ok(HarnessHealthStatus::Missing),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Ok(HarnessHealthStatus::Missing)
            }
            Err(err) => Err(CoreError::Io(err)),
        }
    } else if which_command(&spec.command).is_some() {
        Ok(HarnessHealthStatus::Ready)
    } else {
        Ok(HarnessHealthStatus::Missing)
    }
}

pub fn install_doc_url(agent_id: &str) -> &'static str {
    match agent_id {
        "claude-code-acp" => "https://docs.anthropic.com/en/docs/claude-code",
        "claude-native" => "https://docs.anthropic.com/en/docs/claude-code",
        "codex-native" => "https://developers.openai.com/codex",
        "goose" | "goose-acp" => "https://block.github.io/goose/docs/getting-started/installation/",
        "hermes" | "hermes-acp" => "https://github.com/NousResearch/hermes-agent",
        "opencode" | "opencode-acp" | "opencode-native" => "https://opencode.ai/docs",
        "pi" | "pi-acp" | "pi-native" => "https://github.com/badlogic/pi-mono",
        "mock-acp" => "https://github.com/tamtri/tamtri/tree/main/fixtures/mock-acp-agent",
        _ => "https://agentclientprotocol.com",
    }
}

pub fn it_admin_checklist(entries: &[HarnessHealthEntry]) -> String {
    let mut lines = vec![
        "tamtri harness setup checklist".to_string(),
        String::new(),
        "Install at least one ACP-capable agent and confirm tamtri can find its binary."
            .to_string(),
        "Gateway MCP servers live in <vault>/config.json; credentials stay in the daemon store."
            .to_string(),
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

pub fn discover_known_agents() -> Vec<AgentLaunchSpec> {
    let mut found = Vec::new();
    if let Some(command) = resolve_hermes_command() {
        found.push(AgentLaunchSpec {
            id: "hermes-acp".into(),
            display_name: "Hermes".into(),
            command,
            args: vec!["acp".into()],
            env: Vec::new(),
            adapter: AdapterKind::default(),
            enabled: true,
        });
    }
    if let Some(command) = which_command("claude") {
        found.push(AgentLaunchSpec {
            id: "claude-native".into(),
            display_name: "Claude Code".into(),
            command: command.to_string_lossy().into_owned(),
            args: Vec::new(),
            env: Vec::new(),
            adapter: AdapterKind::ClaudeNative,
            enabled: true,
        });
        found.push(AgentLaunchSpec {
            id: "claude-code-acp".into(),
            display_name: "Claude Code (ACP)".into(),
            command: command.to_string_lossy().into_owned(),
            args: vec!["acp".into()],
            env: Vec::new(),
            adapter: AdapterKind::default(),
            enabled: true,
        });
    }
    if let Some(command) = which_command("codex") {
        found.push(AgentLaunchSpec {
            id: "codex-native".into(),
            display_name: "Codex".into(),
            command: command.to_string_lossy().into_owned(),
            args: vec!["app-server".into()],
            env: Vec::new(),
            adapter: AdapterKind::CodexNative,
            enabled: true,
        });
    }
    if which_command("goose").is_some() {
        found.push(AgentLaunchSpec {
            id: "goose-acp".into(),
            display_name: "Goose".into(),
            command: "goose".into(),
            args: Vec::new(),
            env: Vec::new(),
            adapter: AdapterKind::default(),
            enabled: true,
        });
    }
    if which_command("opencode").is_some() {
        found.push(AgentLaunchSpec {
            id: "opencode-native".into(),
            display_name: "OpenCode".into(),
            command: "opencode".into(),
            args: vec!["serve".into()],
            env: Vec::new(),
            adapter: AdapterKind::OpenCodeNative,
            enabled: true,
        });
        found.push(AgentLaunchSpec {
            id: "opencode-acp".into(),
            display_name: "OpenCode (ACP)".into(),
            command: "opencode".into(),
            args: vec!["acp".into()],
            env: Vec::new(),
            adapter: AdapterKind::default(),
            enabled: true,
        });
    }
    if which_command("pi").is_some() {
        found.push(AgentLaunchSpec {
            id: "pi-native".into(),
            display_name: "Pi".into(),
            command: "pi".into(),
            args: Vec::new(),
            env: Vec::new(),
            adapter: AdapterKind::PiNative,
            enabled: true,
        });
    }
    if which_command("pi-acp").is_some() {
        found.push(AgentLaunchSpec {
            id: "pi-acp".into(),
            display_name: "Pi (ACP)".into(),
            command: "pi-acp".into(),
            args: Vec::new(),
            env: Vec::new(),
            adapter: AdapterKind::default(),
            enabled: true,
        });
    }
    found
}

fn resolve_hermes_command() -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(Path::new(&home).join(".local/bin/hermes"));
    }
    candidates.push(Path::new("/opt/homebrew/bin/hermes").to_path_buf());
    candidates.push(Path::new("/usr/local/bin/hermes").to_path_buf());
    for path in candidates {
        if path.is_file() && is_executable(&path) {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    which_command("hermes").map(|path| path.to_string_lossy().into_owned())
}

pub fn adapter_type_label(kind: &AdapterKind) -> &'static str {
    match kind {
        AdapterKind::Acp => "acp",
        AdapterKind::ClaudeNative
        | AdapterKind::CodexNative
        | AdapterKind::OpenCodeNative
        | AdapterKind::PiNative => "native",
    }
}

pub fn adapter_kind_label(kind: &AdapterKind) -> &'static str {
    match kind {
        AdapterKind::Acp => "acp",
        AdapterKind::ClaudeNative => "claude_native",
        AdapterKind::CodexNative => "codex_native",
        AdapterKind::OpenCodeNative => "opencode_native",
        AdapterKind::PiNative => "pi_native",
    }
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
    use std::sync::Mutex;

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn harness_health_detects_missing_ready_and_unknown() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ready_path = temp.path().join("ready-agent");
        std::fs::write(&ready_path, b"#!/bin/sh\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ready_path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }

        assert_eq!(
            probe_agent_launch_spec(&AgentLaunchSpec {
                id: "ready".into(),
                display_name: "Ready".into(),
                command: ready_path.to_string_lossy().into_owned(),
                args: Vec::new(),
                env: Vec::new(),
                adapter: AdapterKind::default(),
                enabled: true,
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
                adapter: AdapterKind::default(),
                enabled: true,
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

    #[test]
    fn discover_known_agents_finds_hermes_at_known_path() {
        let _guard = ENV_TEST_LOCK.lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let hermes_path = temp.path().join("hermes");
        std::fs::write(&hermes_path, b"#!/bin/sh\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hermes_path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let home = temp.path().join("home");
        std::fs::create_dir_all(home.join(".local/bin")).expect("mkdir");
        std::fs::copy(&hermes_path, home.join(".local/bin/hermes")).expect("copy");
        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }
        let discovered = discover_known_agents();
        if let Some(value) = previous_home {
            unsafe {
                std::env::set_var("HOME", value);
            }
        } else {
            unsafe {
                std::env::remove_var("HOME");
            }
        }
        assert!(
            discovered
                .iter()
                .any(|spec| spec.id == "hermes-acp" && spec.args == vec!["acp".to_string()])
        );
    }

    #[test]
    fn install_doc_url_known_agents() {
        assert!(install_doc_url("opencode-native").contains("opencode.ai"));
        assert!(install_doc_url("pi-native").contains("pi-mono"));
    }

    #[test]
    fn discover_known_agents_finds_opencode_on_path() {
        let _guard = ENV_TEST_LOCK.lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let opencode_path = temp.path().join("opencode");
        std::fs::write(&opencode_path, b"#!/bin/sh\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&opencode_path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let previous_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", temp.path());
        }
        let discovered = discover_known_agents();
        if let Some(value) = previous_path {
            unsafe {
                std::env::set_var("PATH", value);
            }
        } else {
            unsafe {
                std::env::remove_var("PATH");
            }
        }
        assert!(discovered.iter().any(|spec| {
            spec.id == "opencode-native"
                && spec.command == "opencode"
                && spec.adapter == AdapterKind::OpenCodeNative
        }));
        assert!(discovered.iter().any(|spec| {
            spec.id == "opencode-acp"
                && spec.command == "opencode"
                && spec.args == vec!["acp".to_string()]
        }));
    }

    #[test]
    fn discover_known_agents_finds_pi_native_on_path() {
        let _guard = ENV_TEST_LOCK.lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let pi_path = temp.path().join("pi");
        std::fs::write(&pi_path, b"#!/bin/sh\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&pi_path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let previous_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", temp.path());
        }
        let discovered = discover_known_agents();
        if let Some(value) = previous_path {
            unsafe {
                std::env::set_var("PATH", value);
            }
        } else {
            unsafe {
                std::env::remove_var("PATH");
            }
        }
        assert!(discovered.iter().any(|spec| {
            spec.id == "pi-native" && spec.command == "pi" && spec.adapter == AdapterKind::PiNative
        }));
    }

    #[test]
    fn discover_known_agents_skips_pi_acp_without_bridge() {
        let _guard = ENV_TEST_LOCK.lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let pi_path = temp.path().join("pi");
        std::fs::write(&pi_path, b"#!/bin/sh\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&pi_path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let previous_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", temp.path());
        }
        let discovered = discover_known_agents();
        if let Some(value) = previous_path {
            unsafe {
                std::env::set_var("PATH", value);
            }
        } else {
            unsafe {
                std::env::remove_var("PATH");
            }
        }
        assert!(discovered.iter().any(|spec| spec.id == "pi-native"));
        assert!(!discovered.iter().any(|spec| spec.id == "pi-acp"));
    }
}
