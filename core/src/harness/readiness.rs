//! Normalized agent readiness diagnostics. Adapters own quirks; the UI receives
//! only state + recovery action.

use crate::harness::acp::{AdapterKind, AgentLaunchSpec};
use crate::harness::auth::{claude_auth_ready, codex_credentials_present};
use crate::harness::health::{
    HarnessHealthStatus, install_doc_url, probe_agent_launch_spec_result,
};
use crate::harness::roster::roster_adapter_mismatch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadinessState {
    Missing,
    Installed,
    SignInRequired,
    Ready,
    Disabled,
    Misconfigured,
    CheckFailed,
}

impl ReadinessState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Installed => "installed",
            Self::SignInRequired => "sign_in_required",
            Self::Ready => "ready",
            Self::Disabled => "disabled",
            Self::Misconfigured => "misconfigured",
            Self::CheckFailed => "check_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessDiagnostic {
    pub state: ReadinessState,
    pub recovery_action: String,
    pub message: Option<String>,
    pub install_doc_url: String,
}

pub fn default_recommendation_priorities() -> Vec<&'static str> {
    vec![
        "claude-native",
        "claude-code-acp",
        "codex-native",
        "hermes-acp",
        "goose-acp",
    ]
}

pub fn diagnose_agent(spec: &AgentLaunchSpec, enabled: bool) -> ReadinessDiagnostic {
    let install_doc_url = install_doc_url(&spec.id).to_string();
    if !enabled {
        return ReadinessDiagnostic {
            state: ReadinessState::Disabled,
            recovery_action: "enable".into(),
            message: Some(format!(
                "{} is disabled in your agent roster.",
                spec.display_name
            )),
            install_doc_url,
        };
    }

    if let Some(message) = roster_adapter_mismatch(spec) {
        return ReadinessDiagnostic {
            state: ReadinessState::Misconfigured,
            recovery_action: "fix_roster".into(),
            message: Some(message),
            install_doc_url,
        };
    }

    let path_status = match probe_agent_launch_spec_result(spec) {
        Ok(status) => status,
        Err(err) => {
            return ReadinessDiagnostic {
                state: ReadinessState::CheckFailed,
                recovery_action: "retry".into(),
                message: Some(err.to_string()),
                install_doc_url,
            };
        }
    };
    match path_status {
        HarnessHealthStatus::Missing => ReadinessDiagnostic {
            state: ReadinessState::Missing,
            recovery_action: "install".into(),
            message: Some(format!("{} is not installed yet.", spec.display_name)),
            install_doc_url,
        },
        HarnessHealthStatus::Unknown => ReadinessDiagnostic {
            state: ReadinessState::Misconfigured,
            recovery_action: "ask_it".into(),
            message: Some(format!(
                "{} was found but is not executable.",
                spec.display_name
            )),
            install_doc_url,
        },
        HarnessHealthStatus::Ready => diagnose_installed(spec, install_doc_url),
    }
}

fn diagnose_installed(spec: &AgentLaunchSpec, install_doc_url: String) -> ReadinessDiagnostic {
    match spec.adapter {
        AdapterKind::ClaudeNative | AdapterKind::Acp if spec.id.contains("claude") => {
            if claude_auth_ready(&spec.command) {
                ReadinessDiagnostic {
                    state: ReadinessState::Ready,
                    recovery_action: "continue".into(),
                    message: None,
                    install_doc_url,
                }
            } else {
                ReadinessDiagnostic {
                    state: ReadinessState::SignInRequired,
                    recovery_action: "sign_in".into(),
                    message: Some(
                        "Claude Code is installed but not signed in. Run `claude login` in Terminal."
                            .into(),
                    ),
                    install_doc_url,
                }
            }
        }
        AdapterKind::CodexNative | AdapterKind::Acp if spec.id.contains("codex") => {
            if codex_credentials_present() {
                ReadinessDiagnostic {
                    state: ReadinessState::Ready,
                    recovery_action: "continue".into(),
                    message: None,
                    install_doc_url,
                }
            } else {
                ReadinessDiagnostic {
                    state: ReadinessState::SignInRequired,
                    recovery_action: "sign_in".into(),
                    message: Some("Codex is installed but needs sign-in.".into()),
                    install_doc_url,
                }
            }
        }
        _ => ReadinessDiagnostic {
            state: ReadinessState::Ready,
            recovery_action: "continue".into(),
            message: None,
            install_doc_url,
        },
    }
}

pub struct RecommendableAgent<'a> {
    pub id: &'a str,
    pub display_name: &'a str,
    pub state: ReadinessState,
    pub recovery_action: String,
}

pub fn recommend_agent<'a>(
    entries: &'a [RecommendableAgent<'a>],
) -> Option<&'a RecommendableAgent<'a>> {
    let priorities = default_recommendation_priorities();
    for priority_id in &priorities {
        if let Some(entry) = entries.iter().find(|e| e.id == *priority_id)
            && entry.state == ReadinessState::Ready
        {
            return Some(entry);
        }
    }
    for priority_id in &priorities {
        if let Some(entry) = entries.iter().find(|e| e.id == *priority_id)
            && matches!(
                entry.state,
                ReadinessState::SignInRequired
                    | ReadinessState::Installed
                    | ReadinessState::Missing
            )
        {
            return Some(entry);
        }
    }
    entries.first()
}

pub fn recommend_agent_id<'a>(entries: &'a [RecommendableAgent<'a>]) -> Option<String> {
    recommend_agent(entries).map(|entry| entry.id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::acp::{AdapterKind, AgentLaunchSpec};

    fn spec(id: &str, adapter: AdapterKind) -> AgentLaunchSpec {
        AgentLaunchSpec {
            id: id.into(),
            display_name: id.into(),
            command: "/nonexistent/agent-binary".into(),
            args: if id == "hermes-acp" {
                vec!["acp".into()]
            } else {
                Vec::new()
            },
            env: Vec::new(),
            adapter,
            enabled: true,
        }
    }

    #[test]
    fn disabled_agent_reports_disabled() {
        let mut agent = spec("claude-native", AdapterKind::ClaudeNative);
        agent.enabled = false;
        let diag = diagnose_agent(&agent, false);
        assert_eq!(diag.state, ReadinessState::Disabled);
        assert_eq!(diag.recovery_action, "enable");
    }

    #[test]
    fn misconfigured_roster_reports_before_binary_probe() {
        let agent = spec("claude-native", AdapterKind::Acp);
        let diag = diagnose_agent(&agent, true);
        assert_eq!(diag.state, ReadinessState::Misconfigured);
        assert_eq!(diag.recovery_action, "fix_roster");
    }

    #[test]
    fn missing_binary_reports_missing() {
        let agent = spec("hermes-acp", AdapterKind::Acp);
        let diag = diagnose_agent(&agent, true);
        assert_eq!(diag.state, ReadinessState::Missing);
    }

    #[test]
    fn recommend_prefers_ready_claude_over_missing_codex() {
        let entries = vec![
            RecommendableAgent {
                id: "codex-native",
                display_name: "Codex",
                state: ReadinessState::Missing,
                recovery_action: "install".into(),
            },
            RecommendableAgent {
                id: "claude-native",
                display_name: "Claude",
                state: ReadinessState::Ready,
                recovery_action: "continue".into(),
            },
        ];
        let picked = recommend_agent(&entries).expect("recommendation");
        assert_eq!(picked.id, "claude-native");
    }
}
