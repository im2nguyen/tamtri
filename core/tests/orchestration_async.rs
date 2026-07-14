use std::sync::Arc;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::mcp::gateway::{McpGateway, NoCredentials};
use tamtri_core::orchestration::mcp_tools::TAMTRI_SERVER_ID;

struct RecordingObserver {
    events: std::sync::Mutex<Vec<UiEvent>>,
}

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, event: UiEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[test]
fn gateway_lists_tamtri_orchestration_tools_with_agent_context() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let gateway = Arc::new(
            McpGateway::new(
                tamtri_core::config::GatewayConfig::default(),
                Arc::new(NoCredentials),
                None,
            )
            .expect("gateway"),
        );
        gateway
            .set_agent_context("conversation-id".to_string(), true)
            .await;

        let tools = gateway.list_tools().await.expect("tools");
        assert!(tools.iter().any(|tool| tool.server_id == TAMTRI_SERVER_ID));
        assert!(tools
            .iter()
            .any(|tool| tool.exposed_name.contains("orchestration_run")));
    });
}

#[test]
fn orchestration_run_returns_running_when_shared_core_installed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver {
        events: std::sync::Mutex::new(Vec::new()),
    });
    let core = Arc::new(
        TamtriCore::new(
            temp.path().to_string_lossy().into_owned(),
            Arc::clone(&observer) as Arc<dyn ConversationObserver>,
        )
        .expect("core"),
    );
    TamtriCore::install_shared(Arc::clone(&core));

    tamtri_core::orchestration::store::seed_starter_recipes(temp.path()).expect("seed");

    let conversation = core
        .create_conversation_inner("test", "mock-acp", "default")
        .expect("conversation");

    let dto = core
        .orchestration_run_inner(
            "handoff",
            &conversation.id,
            Some(r#"{"harness_id":"mock-acp","model_id":"default","message":"hi"}"#),
        )
        .expect("run");

    assert_eq!(dto.status, "running");
}
