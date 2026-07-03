use std::fs;
use std::sync::{Arc, Mutex};

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
fn app_navigation_blocked_appends_events_receipt() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    let conversation = core
        .create_conversation(
            "App nav block".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let blocked_url = "https://evil.example/redirect";
    core.log_app_navigation_blocked(
        conversation.id.clone(),
        "m7-app".to_string(),
        "ui://m7-app/demo".to_string(),
        blocked_url.to_string(),
    )
    .expect("log");

    let events_path = find_file(temp.path(), "events.jsonl").expect("events.jsonl");
    let events_text = fs::read_to_string(events_path).expect("read events");
    let events: Vec<serde_json::Value> = events_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("event line"))
        .collect();
    let blocked: Vec<_> = events
        .iter()
        .filter(|event| event["kind"] == "app_navigation_blocked")
        .collect();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["payload"]["url"].as_str(), Some(blocked_url));
    assert_eq!(blocked[0]["payload"]["server_id"].as_str(), Some("m7-app"));
    assert_eq!(
        blocked[0]["payload"]["template_ref"].as_str(),
        Some("ui://m7-app/demo")
    );
}

fn find_file(root: &std::path::Path, suffix: &str) -> Option<std::path::PathBuf> {
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, suffix) {
                return Some(found);
            }
        } else if path.to_string_lossy().ends_with(suffix) {
            return Some(path);
        }
    }
    None
}
