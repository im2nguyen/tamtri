//! Canonical roster id → adapter/args expectations and repair for mismatches.

use crate::harness::acp::{AdapterKind, AgentLaunchSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RosterExpectation {
    adapter: AdapterKind,
    acp_args: bool,
}

fn expectation_for_id(id: &str) -> Option<RosterExpectation> {
    match id {
        "claude-native" => Some(RosterExpectation {
            adapter: AdapterKind::ClaudeNative,
            acp_args: false,
        }),
        "claude-code-acp" => Some(RosterExpectation {
            adapter: AdapterKind::Acp,
            acp_args: true,
        }),
        "codex-native" => Some(RosterExpectation {
            adapter: AdapterKind::CodexNative,
            acp_args: false,
        }),
        "opencode-native" => Some(RosterExpectation {
            adapter: AdapterKind::OpenCodeNative,
            acp_args: false,
        }),
        "opencode-acp" => Some(RosterExpectation {
            adapter: AdapterKind::Acp,
            acp_args: true,
        }),
        "pi-native" => Some(RosterExpectation {
            adapter: AdapterKind::PiNative,
            acp_args: false,
        }),
        "pi-acp" => Some(RosterExpectation {
            adapter: AdapterKind::Acp,
            acp_args: false,
        }),
        "hermes-acp" | "goose-acp" => Some(RosterExpectation {
            adapter: AdapterKind::Acp,
            acp_args: id == "hermes-acp",
        }),
        _ => None,
    }
}

/// Infer adapter kind from roster id when the caller omitted an explicit adapter.
pub fn infer_adapter_kind(id: &str, explicit: &str) -> AdapterKind {
    if !explicit.trim().is_empty() {
        return parse_adapter_label(explicit);
    }
    expectation_for_id(id)
        .map(|expectation| expectation.adapter)
        .unwrap_or(AdapterKind::Acp)
}

pub fn parse_adapter_label(raw: &str) -> AdapterKind {
    match raw.trim() {
        "claude_native" => AdapterKind::ClaudeNative,
        "codex_native" => AdapterKind::CodexNative,
        "opencode_native" => AdapterKind::OpenCodeNative,
        "pi_native" => AdapterKind::PiNative,
        _ => AdapterKind::Acp,
    }
}

/// Returns a user-facing message when id, adapter, and args disagree.
pub fn roster_adapter_mismatch(spec: &AgentLaunchSpec) -> Option<String> {
    let expectation = expectation_for_id(&spec.id)?;
    let adapter_wrong = spec.adapter != expectation.adapter;
    let args_wrong = expectation.acp_args && spec.args != ["acp"];
    if !adapter_wrong && !args_wrong {
        return None;
    }
    let expected_adapter = adapter_kind_label(&expectation.adapter);
    let actual_adapter = adapter_kind_label(&spec.adapter);
    let mut parts = vec![format!(
        "{} is configured with adapter `{}` but this roster id expects `{}`.",
        spec.display_name, actual_adapter, expected_adapter
    )];
    if args_wrong {
        parts.push("ACP mode requires args `[\"acp\"]`. Update the roster entry or re-add the agent from Settings.".into());
    } else {
        parts.push("Edit ~/.tamtri/vault/config.json or re-add the agent from Settings.".into());
    }
    Some(parts.join(" "))
}

/// Auto-correct known id/adapter mismatches. Returns true if the spec changed.
pub fn repair_roster_spec(spec: &mut AgentLaunchSpec) -> bool {
    let Some(expectation) = expectation_for_id(&spec.id) else {
        return false;
    };
    let mut changed = false;
    if spec.adapter != expectation.adapter {
        spec.adapter = expectation.adapter;
        changed = true;
    }
    if expectation.acp_args && spec.args != ["acp"] {
        spec.args = vec!["acp".into()];
        changed = true;
    }
    changed
}

pub fn repair_agent_roster(roster: &mut [AgentLaunchSpec]) -> bool {
    roster.iter_mut().any(repair_roster_spec)
}

fn adapter_kind_label(kind: &AdapterKind) -> &'static str {
    match kind {
        AdapterKind::Acp => "acp",
        AdapterKind::ClaudeNative => "claude_native",
        AdapterKind::CodexNative => "codex_native",
        AdapterKind::OpenCodeNative => "opencode_native",
        AdapterKind::PiNative => "pi_native",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, adapter: AdapterKind, args: &[&str]) -> AgentLaunchSpec {
        AgentLaunchSpec {
            id: id.into(),
            display_name: id.into(),
            command: "mock".into(),
            args: args.iter().map(|value| (*value).to_string()).collect(),
            env: Vec::new(),
            adapter,
            enabled: true,
        }
    }

    #[test]
    fn detects_claude_native_with_acp_adapter() {
        let agent = spec("claude-native", AdapterKind::Acp, &[]);
        assert!(roster_adapter_mismatch(&agent).is_some());
    }

    #[test]
    fn repairs_claude_native_adapter() {
        let mut agent = spec("claude-native", AdapterKind::Acp, &[]);
        assert!(repair_roster_spec(&mut agent));
        assert_eq!(agent.adapter, AdapterKind::ClaudeNative);
    }

    #[test]
    fn infers_native_adapter_from_id_when_explicit_empty() {
        assert_eq!(
            infer_adapter_kind("claude-native", ""),
            AdapterKind::ClaudeNative
        );
        assert_eq!(infer_adapter_kind("hermes-acp", ""), AdapterKind::Acp);
    }

    #[test]
    fn explicit_adapter_overrides_inference() {
        assert_eq!(infer_adapter_kind("claude-native", "acp"), AdapterKind::Acp);
    }
}
