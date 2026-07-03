use std::sync::{Arc, Mutex};
use std::time::Duration;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::conversation::Id;
use tamtri_core::mcp::app_bridge::{
    AppBridgeCoordinator, AppBridgeResolution, APP_BRIDGE_DENY,
};

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<UiEvent>>,
}

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, event: UiEvent) {
        self.events.lock().expect("events").push(event);
    }
}

#[test]
fn app_bridge_action_requires_consent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer.clone())
        .expect("core");
    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");
    let conversation = core
        .create_conversation(
            "App bridge".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "hello".to_string())
        .expect("send");

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if core
            .submit_app_bridge_request(
                conversation.id.clone(),
                "m7-app".to_string(),
                "ui://m7-app/demo".to_string(),
                "ui://m7-app/demo".to_string(),
                r#"{"jsonrpc":"2.0","id":"bridge-1","method":"tools/call","params":{"name":"echo","arguments":{}}}"#
                    .to_string(),
            )
            .is_ok()
        {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("active run never became available for app bridge");
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(
        observer
            .events
            .lock()
            .expect("events")
            .iter()
            .any(|event| event.kind == "app_bridge_consent_requested"),
        "expected app_bridge_consent_requested UI event"
    );
}

#[test]
fn app_bridge_denied_action_not_executed() {
    let coordinator = AppBridgeCoordinator::default();
    let conversation_id = Id::now_v7();
    let request = tamtri_core::mcp::app_bridge::parse_app_bridge_rpc(
        r#"{"jsonrpc":"2.0","id":"9","method":"tools/call","params":{"name":"echo","arguments":{}}}"#,
    )
    .expect("rpc");
    let result = coordinator
        .begin_request(
            conversation_id,
            "m7-app",
            "ui://m7-app/demo",
            "ui://m7-app/demo",
            &request,
        )
        .expect("begin");
    let (consent, mut rx) = match result {
        tamtri_core::mcp::app_bridge::AppBridgeBeginResult::NeedsConsent(consent, rx) => {
            (consent, rx)
        }
        tamtri_core::mcp::app_bridge::AppBridgeBeginResult::AlreadyGranted(_) => {
            panic!("expected consent prompt")
        }
    };
    match coordinator
        .resolve_consent(conversation_id, &consent.request_id, APP_BRIDGE_DENY)
        .expect("resolve")
    {
        AppBridgeResolution::Denied { response, .. } => {
            assert!(response.contains("user denied"));
        }
        AppBridgeResolution::Approved { .. } => panic!("expected deny"),
    }
    let delivered = rx.try_recv().expect("response");
    assert!(delivered.contains("user denied"));
}
