use std::sync::Arc;

use serde_json::json;
use tamtri_core::config::{GatewayConfig, GatewayServerConfig, GatewayTransport};
use tamtri_core::mcp::capabilities::{
    FeatureStatus, TamtriFeatureSupport, apps_available, report_from_initialize,
    server_advertises_apps, server_advertises_tasks, tasks_available,
    upstream_gateway_capabilities,
};
use tamtri_core::mcp::gateway::{McpGateway, NoCredentials};
use tamtri_core::mcp::client::McpClient;
use tamtri_core::mcp::protocol::{ServerCapabilities, ToolsCapability};

fn stdio_server(command: &str) -> GatewayServerConfig {
    GatewayServerConfig {
        id: "fixture".to_string(),
        display_name: "Fixture".to_string(),
        enabled: true,
        scope: tamtri_core::config::GatewayScope::User,
        transport: GatewayTransport::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        credentials: Vec::new(),
        oauth: None,
        timeout_secs: Some(30),
    }
}

#[tokio::test]
async fn sampling_declined_cleanly() {
    let command = env!("CARGO_BIN_EXE_m7-rc-mcp");
    let client = McpClient::connect_stdio(command, &[], &[], Default::default())
        .await
        .expect("connect rc fixture");
    let result = client
        .call_tool("probe_sampling", json!({}), None)
        .await
        .expect("tool call");
    let declined = result
        .structured_content
        .as_ref()
        .and_then(|value| value.get("samplingDeclined"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    assert!(declined, "downstream sampling request should be declined by tamtri client");
    let _ = client.close().await;
}

#[tokio::test]
async fn rc_extension_capability_gate() {
    let command = env!("CARGO_BIN_EXE_m7-rc-mcp");
    let client = McpClient::connect_stdio(command, &[], &[], Default::default())
        .await
        .expect("connect rc fixture");
    let caps = client
        .server_capabilities()
        .expect("initialize capabilities")
        .clone();
    assert!(server_advertises_apps(&caps));
    assert!(server_advertises_tasks(&caps));
    assert!(!apps_available(&caps, TamtriFeatureSupport::milestone_7_pr1()));
    assert!(!tasks_available(&caps, TamtriFeatureSupport::milestone_7_pr1()));
    assert!(apps_available(&caps, TamtriFeatureSupport::milestone_7_pr2()));
    assert!(!tasks_available(&caps, TamtriFeatureSupport::milestone_7_pr2()));
    assert!(tasks_available(&caps, TamtriFeatureSupport::milestone_7_pr4()));

    let report = report_from_initialize(
        "rc",
        "2026-07-28",
        &caps,
        TamtriFeatureSupport::milestone_7_pr1(),
    );
    assert_eq!(report.apps, FeatureStatus::ServerOnly);
    assert_eq!(report.tasks, FeatureStatus::ServerOnly);
    assert_eq!(report.sampling, FeatureStatus::Declined);
    let _ = client.close().await;
}

#[tokio::test]
async fn stable_server_without_extensions() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let client = McpClient::connect_stdio(command, &[], &[], Default::default())
        .await
        .expect("connect stable fixture");
    let caps = client
        .server_capabilities()
        .expect("initialize capabilities")
        .clone();
    assert!(caps.tools.is_some());
    assert!(!server_advertises_apps(&caps));
    assert!(!server_advertises_tasks(&caps));
    let report = report_from_initialize(
        "mock",
        "2025-11-25",
        &caps,
        TamtriFeatureSupport::milestone_7_pr1(),
    );
    assert_eq!(report.apps, FeatureStatus::Unavailable);
    assert_eq!(report.tasks, FeatureStatus::Unavailable);
    let _ = client.close().await;
}

#[test]
fn unknown_extension_ignored_in_capability_report() {
    let caps = ServerCapabilities {
        tools: Some(ToolsCapability { list_changed: None }),
        extensions: Some(json!({
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
fn upstream_gateway_advertises_roots_when_enabled() {
    let caps = upstream_gateway_capabilities();
    assert!(caps.roots.is_some(), "M7 current build should advertise roots");
    assert!(TamtriFeatureSupport::current().roots);
}

#[tokio::test]
async fn gateway_probe_records_per_server_capability_report() {
    let command = env!("CARGO_BIN_EXE_m7-rc-mcp");
    let mut config = GatewayConfig::default();
    let mut server = stdio_server(command);
    server.id = "rc".to_string();
    config.servers.push(server);

    let gateway = McpGateway::new(config, Arc::new(NoCredentials), None).unwrap();
    let reports = gateway.probe_server_capabilities().await.expect("probe");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].server_id, "rc");
    assert_eq!(reports[0].apps, FeatureStatus::Supported);

    let cached = gateway
        .capability_report("rc")
        .await
        .expect("cached capability report");
    assert_eq!(cached.tasks, FeatureStatus::Supported);
}
