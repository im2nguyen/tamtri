use std::fs;
use std::path::Path;
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
fn milestone5_artifact_snapshot_from_file_changed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(
        temp.path().to_string_lossy().into_owned(),
        Arc::clone(&observer) as Arc<dyn ConversationObserver>,
    )
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
            "Artifact".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "hello".to_string())
        .expect("send");
    let request_id = wait_for_permission_requested(observer.as_ref());
    core.respond_permission(
        conversation.id.clone(),
        request_id,
        "allow_once".to_string(),
    )
    .expect("permission");
    wait_for_turn_end(observer.as_ref());
    std::thread::sleep(Duration::from_millis(200));

    let loaded = core.load_conversation(conversation.id).expect("load");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&loaded.transcript_json).expect("transcript");
    assert_eq!(messages.len(), 2);
    let assistant = &messages[1];
    let artifact = assistant["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["type"] == "artifact")
        .expect("artifact block");
    assert_eq!(artifact["mime_type"], "text/html");
    assert!(artifact["inline"].as_str().unwrap().contains("<h1>ok</h1>"));
    let attachment_path = artifact["path"].as_str().unwrap();
    assert!(attachment_path.starts_with("attachments/"));
    assert!(artifact["sha256"].as_str().is_some_and(|hash| hash.len() == 64));
    assert!(artifact["size"].as_u64().is_some_and(|size| size > 0));
    assert!(
        find_file(temp.path(), attachment_path).is_some(),
        "attachment file should exist in vault"
    );

    let workdir_report = find_file(temp.path(), "workdir/report.html").unwrap();
    fs::write(workdir_report, "<h1>changed</h1>").unwrap();
    let reloaded = core.load_conversation(loaded.id).expect("reload");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&reloaded.transcript_json).expect("transcript");
    let assistant = &messages[1];
    let artifact = assistant["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["type"] == "artifact")
        .expect("artifact block");
    assert!(artifact["inline"].as_str().unwrap().contains("<h1>ok</h1>"));

    let events_path = find_file(temp.path(), "events.jsonl").expect("events");
    let events_text = fs::read_to_string(events_path).expect("read events");
    let events: Vec<serde_json::Value> = events_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("event line"))
        .collect();
    let snapshotted: Vec<_> = events
        .iter()
        .filter(|event| event["kind"] == "artifact_snapshotted")
        .collect();
    assert!(!snapshotted.is_empty());
    assert_eq!(snapshotted.len(), 1);
    assert_eq!(
        snapshotted[0]["payload"]["tool_call_id"].as_str(),
        Some("tool-1")
    );
    assert!(
        snapshotted[0]["payload"]["original_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("workdir/report.html")),
    );
    assert_eq!(
        snapshotted[0]["payload"]["attachment_path"].as_str(),
        Some(attachment_path)
    );
}

fn wait_for_permission_requested(observer: &RecordingObserver) -> String {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for permission_requested");
        }
        let events = observer.events.lock().expect("events");
        if let Some(event) = events.iter().find(|event| event.kind == "permission_requested") {
            let payload: serde_json::Value =
                serde_json::from_str(&event.payload_json).expect("payload");
            return payload["request_id"]
                .as_str()
                .unwrap_or("perm-1")
                .to_string();
        }
        drop(events);
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_turn_end(observer: &RecordingObserver) {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for turn_ended");
        }
        let events = observer.events.lock().expect("events");
        if events.iter().any(|event| event.kind == "turn_ended") {
            return;
        }
        drop(events);
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn find_file(root: &Path, suffix: &str) -> Option<std::path::PathBuf> {
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
