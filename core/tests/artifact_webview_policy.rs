use tamtri_core::conversation::ContentBlock;
use tamtri_core::mcp::app::app_bridge_bootstrap_script;

/// Milestone 7 enumerated test #6: artifact webviews must never expose the MCP App bridge.
#[test]
fn artifact_webview_still_has_no_bridge() {
    let bridge = app_bridge_bootstrap_script("tamtriAppBridge");
    assert!(bridge.contains("tamtriAppBridge"));
    assert!(bridge.contains("__tamtriAppBridgeInstalled"));

    let block = ContentBlock::artifact(
        "attachments/report.html",
        "text/html",
        12,
        "abc",
        Some("<h1>Report</h1>".into()),
    )
    .unwrap();
    let json = serde_json::to_string(&block).unwrap();
    assert!(!json.contains("tamtriAppBridge"));
    assert!(!json.contains("__tamtriAppBridgeInstalled"));

    let ContentBlock::Artifact { inline, .. } = block else {
        panic!("expected artifact block");
    };
    let inline = inline.expect("inline html");
    assert!(!inline.contains("tamtriAppBridge"));
    assert!(!inline.contains("__tamtriAppBridgeInstalled"));
}
