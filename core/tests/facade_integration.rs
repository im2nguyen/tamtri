use std::sync::{Arc, Mutex};
use std::time::Duration;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};

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
fn facade_run_commits_exactly_one_assistant_message() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");
    let conversation = core
        .create_conversation(
            "Run".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "hello".to_string())
        .expect("send");
    std::thread::sleep(Duration::from_millis(200));
    core.respond_permission(
        conversation.id.clone(),
        "perm-1".to_string(),
        "allow_once".to_string(),
    )
    .expect("permission");
    std::thread::sleep(Duration::from_millis(500));

    let loaded = core.load_conversation(conversation.id).expect("load");
    assert_eq!(loaded.messages_json.len(), 2);
}
