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
    let assistant: serde_json::Value = serde_json::from_str(&loaded.messages_json[1]).unwrap();
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
    assert!(Path::new(temp.path()).join("conversations").exists());

    let workdir_report = find_file(temp.path(), "workdir/report.html").unwrap();
    fs::write(workdir_report, "<h1>changed</h1>").unwrap();
    let reloaded = core.load_conversation(loaded.id).expect("reload");
    let assistant: serde_json::Value = serde_json::from_str(&reloaded.messages_json[1]).unwrap();
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
    assert!(
        snapshotted.iter().any(|event| {
            event["payload"]["tool_call_id"].as_str() == Some("tool-1")
        }),
        "expected artifact_snapshotted receipt with tool_call_id"
    );
}

#[test]
fn list_workdir_files_returns_copied_and_harness_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    let conversation = core
        .create_conversation(
            "Files".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let source = temp.path().join("source.csv");
    fs::write(&source, "a,b\n1,2\n").expect("source");
    let copied = core
        .copy_file_to_workdir(conversation.id.clone(), source.to_string_lossy().into_owned())
        .expect("copy");
    assert_eq!(copied, "source.csv");

    let workdir = core
        .conversation_workdir_path(conversation.id.clone())
        .expect("workdir path");
    fs::create_dir_all(Path::new(&workdir).join("nested")).expect("nested");
    fs::write(Path::new(&workdir).join("nested/report.html"), "<h1>ok</h1>").expect("report");

    let files = core
        .list_workdir_files(conversation.id)
        .expect("list");
    let paths: Vec<_> = files.iter().map(|file| file.relative_path.as_str()).collect();
    assert_eq!(paths, vec!["nested/report.html", "source.csv"]);
    assert_eq!(files[1].size, 8);
    assert!(files.iter().all(|file| file.modified_at > 0));
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
