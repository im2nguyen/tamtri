use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::config::{GatewayScope, GatewayServerConfig, GatewayTransport};

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<UiEvent>>,
}

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, event: UiEvent) {
        self.events.lock().expect("events").push(event);
    }
}

fn m7_roots_server(command: &str) -> GatewayServerConfig {
    GatewayServerConfig {
        id: "m7roots".to_string(),
        display_name: "M7 Roots".to_string(),
        enabled: true,
        scope: GatewayScope::Project,
        transport: GatewayTransport::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        timeout_secs: Some(30),
        credentials: Vec::new(),
        oauth: None,
    }
}

#[test]
fn roots_listed_appends_events_receipt_during_probe_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");

    tamtri_core::config::replace_gateway_servers(
        temp.path(),
        vec![m7_roots_server(env!("CARGO_BIN_EXE_m7-roots-mcp"))],
    )
    .expect("gateway config");

    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");

    let conversation = core
        .create_conversation(
            "Roots audit".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let root_path = temp.path().join("data");
    fs::create_dir_all(&root_path).expect("mkdir");
    let root_uri = format!("file://{}", root_path.to_string_lossy());
    let root = core
        .attach_root(
            conversation.id.clone(),
            "Data".to_string(),
            root_uri,
            "filesystem".to_string(),
            "conversation".to_string(),
        )
        .expect("attach");
    core.sync_runtime_roots(conversation.id.clone(), vec![root])
        .expect("sync roots");

    core.send_message(conversation.id.clone(), "probe roots".to_string())
        .expect("send");

    let workdir = core
        .conversation_workdir_path(conversation.id.clone())
        .expect("workdir");
    let marker = Path::new(&workdir).join(".gateway-probe-roots-ok");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if marker.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(marker.is_file(), "probe_roots gateway call did not complete");

    core.cancel_run(conversation.id).ok();
    std::thread::sleep(Duration::from_millis(300));

    let events_path = find_file(temp.path(), "events.jsonl").expect("events.jsonl");
    let events_text = fs::read_to_string(events_path).expect("read events");
    let events: Vec<serde_json::Value> = events_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("event line"))
        .collect();
    let listed: Vec<_> = events
        .iter()
        .filter(|event| event["kind"] == "roots_listed")
        .collect();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["payload"]["server_id"].as_str(), Some("m7roots"));
    assert_eq!(listed[0]["payload"]["count"].as_u64(), Some(1));
}

fn find_file(root: &Path, name: &str) -> Option<std::path::PathBuf> {
    if root.file_name().is_some_and(|part| part == name) {
        return Some(root.to_path_buf());
    }
    if !root.is_dir() {
        return None;
    }
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().is_some_and(|part| part == name) {
            return Some(path);
        }
        if let Some(found) = find_file(&path, name) {
            return Some(found);
        }
    }
    None
}
