use serde::{Deserialize, Serialize};

use super::protocol::ServerCapabilities;

/// MCP 2026-07-28 RC extension identifiers for Apps and Tasks.
pub const EXT_APPS: &str = "io.modelcontextprotocol/apps";
pub const EXT_TASKS: &str = "io.modelcontextprotocol/tasks";
pub const EXT_ROOTS: &str = "io.modelcontextprotocol/roots";

/// Which MCP features tamtri has wired end-to-end in the current build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TamtriFeatureSupport {
    pub apps: bool,
    pub tasks: bool,
    pub roots: bool,
}

impl TamtriFeatureSupport {
    /// Milestone 7 PR1: RC extensions are parsed and gated, not enabled yet.
    pub const fn milestone_7_pr1() -> Self {
        Self {
            apps: false,
            tasks: false,
            roots: false,
        }
    }

    /// Milestone 7 PR2: Apps model and gateway intercept are wired.
    pub const fn milestone_7_pr2() -> Self {
        Self {
            apps: true,
            tasks: false,
            roots: false,
        }
    }

    /// Milestone 7 PR4: tasks wired end-to-end (apps remain enabled from PR2/3).
    pub const fn milestone_7_pr4() -> Self {
        Self {
            apps: true,
            tasks: true,
            roots: false,
        }
    }

    /// Milestone 7 PR5: roots wired end-to-end.
    pub const fn milestone_7_pr5() -> Self {
        Self {
            apps: true,
            tasks: true,
            roots: true,
        }
    }

    /// Current M7 build support level.
    pub const fn current() -> Self {
        Self::milestone_7_pr4()
    }
}

/// Per-feature availability for settings/debug surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    /// Server advertises the feature and tamtri supports it.
    Supported,
    /// Server advertises the feature but tamtri has not wired it yet.
    ServerOnly,
    /// Server does not advertise the feature.
    Unavailable,
    /// Tamtri explicitly declines (sampling).
    Declined,
    /// Not probed yet (no downstream initialize for this server).
    Unknown,
}

impl FeatureStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::ServerOnly => "server_only",
            Self::Unavailable => "unavailable",
            Self::Declined => "declined",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilityReport {
    pub server_id: String,
    pub protocol_version: String,
    pub tools: FeatureStatus,
    pub resources: FeatureStatus,
    pub prompts: FeatureStatus,
    pub elicitation: FeatureStatus,
    pub apps: FeatureStatus,
    pub tasks: FeatureStatus,
    pub roots: FeatureStatus,
    pub sampling: FeatureStatus,
}

pub fn extension_present(extensions: &serde_json::Value, extension_id: &str) -> bool {
    extensions
        .get(extension_id)
        .is_some_and(|value| !value.is_null())
}

pub fn server_advertises_apps(capabilities: &ServerCapabilities) -> bool {
    capabilities
        .extensions
        .as_ref()
        .is_some_and(|extensions| extension_present(extensions, EXT_APPS))
}

pub fn server_advertises_tasks(capabilities: &ServerCapabilities) -> bool {
    capabilities
        .tasks
        .is_some()
        || capabilities
            .extensions
            .as_ref()
            .is_some_and(|extensions| extension_present(extensions, EXT_TASKS))
}

pub fn server_advertises_roots(capabilities: &ServerCapabilities) -> bool {
    capabilities
        .roots
        .is_some()
        || capabilities
            .extensions
            .as_ref()
            .is_some_and(|extensions| extension_present(extensions, EXT_ROOTS))
}

pub fn apps_available(capabilities: &ServerCapabilities, support: TamtriFeatureSupport) -> bool {
    support.apps && server_advertises_apps(capabilities)
}

pub fn tasks_available(capabilities: &ServerCapabilities, support: TamtriFeatureSupport) -> bool {
    support.tasks && server_advertises_tasks(capabilities)
}

pub fn roots_available(capabilities: &ServerCapabilities, support: TamtriFeatureSupport) -> bool {
    support.roots && server_advertises_roots(capabilities)
}

pub fn report_from_initialize(
    server_id: &str,
    protocol_version: &str,
    capabilities: &ServerCapabilities,
    support: TamtriFeatureSupport,
) -> ServerCapabilityReport {
    let tools = if capabilities.tools.is_some() {
        FeatureStatus::Supported
    } else {
        FeatureStatus::Unavailable
    };
    let resources = if capabilities.resources.is_some() {
        FeatureStatus::Supported
    } else {
        FeatureStatus::Unavailable
    };
    let prompts = if capabilities.prompts.is_some() {
        FeatureStatus::Supported
    } else {
        FeatureStatus::Unavailable
    };
    let elicitation = if capabilities.elicitation.is_some() {
        FeatureStatus::Supported
    } else {
        FeatureStatus::Unavailable
    };
    let apps = if server_advertises_apps(capabilities) {
        if support.apps {
            FeatureStatus::Supported
        } else {
            FeatureStatus::ServerOnly
        }
    } else {
        FeatureStatus::Unavailable
    };
    let tasks = if server_advertises_tasks(capabilities) {
        if support.tasks {
            FeatureStatus::Supported
        } else {
            FeatureStatus::ServerOnly
        }
    } else {
        FeatureStatus::Unavailable
    };
    let roots = if server_advertises_roots(capabilities) {
        if support.roots {
            FeatureStatus::Supported
        } else {
            FeatureStatus::ServerOnly
        }
    } else {
        FeatureStatus::Unavailable
    };
    let sampling = if capabilities.sampling.is_some() {
        FeatureStatus::Declined
    } else {
        FeatureStatus::Unavailable
    };

    ServerCapabilityReport {
        server_id: server_id.to_string(),
        protocol_version: protocol_version.to_string(),
        tools,
        resources,
        prompts,
        elicitation,
        apps,
        tasks,
        roots,
        sampling,
    }
}

/// Capabilities tamtri advertises to agents on the upstream gateway surface.
pub fn upstream_gateway_capabilities() -> ServerCapabilities {
    upstream_gateway_capabilities_for(TamtriFeatureSupport::current())
}

pub fn upstream_gateway_capabilities_for(
    support: TamtriFeatureSupport,
) -> ServerCapabilities {
    use super::protocol::{
        ElicitationCapability, PromptsCapability, ResourcesCapability, RootsCapability,
        ToolsCapability,
    };

    ServerCapabilities {
        tools: Some(ToolsCapability {
            list_changed: Some(false),
        }),
        resources: Some(ResourcesCapability {
            subscribe: None,
            list_changed: Some(false),
        }),
        prompts: Some(PromptsCapability {
            list_changed: Some(false),
        }),
        elicitation: Some(ElicitationCapability {
            form: Some(serde_json::json!({})),
            url: Some(serde_json::json!({})),
        }),
        sampling: None,
        tasks: None,
        roots: if support.roots {
            Some(RootsCapability {
                list_changed: Some(false),
            })
        } else {
            None
        },
        extensions: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::protocol::{ToolsCapability};

    #[test]
    fn rc_extension_capability_gate_marks_server_only() {
        let caps = ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: None }),
            extensions: Some(serde_json::json!({
                EXT_APPS: {"version": "1"},
                EXT_TASKS: {"version": "1"}
            })),
            ..Default::default()
        };
        let report = report_from_initialize(
            "rc",
            "2026-07-28",
            &caps,
            TamtriFeatureSupport::milestone_7_pr1(),
        );
        assert_eq!(report.apps, FeatureStatus::ServerOnly);
        assert_eq!(report.tasks, FeatureStatus::ServerOnly);
        assert_eq!(report.tools, FeatureStatus::Supported);
    }

    #[test]
    fn unknown_extension_does_not_break_report() {
        let caps = ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: None }),
            extensions: Some(serde_json::json!({
                "io.example/unknown": {"version": "9"}
            })),
            ..Default::default()
        };
        let report = report_from_initialize(
            "stable",
            "2025-11-25",
            &caps,
            TamtriFeatureSupport::milestone_7_pr1(),
        );
        assert_eq!(report.apps, FeatureStatus::Unavailable);
        assert_eq!(report.tasks, FeatureStatus::Unavailable);
    }

    #[test]
    fn upstream_gateway_omits_sampling() {
        let caps = upstream_gateway_capabilities();
        assert!(caps.sampling.is_none());
        assert!(caps.tools.is_some());
        assert!(caps.elicitation.is_some());
    }
}
