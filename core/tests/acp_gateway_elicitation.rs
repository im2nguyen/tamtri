use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
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

fn gateway_mock_server(command: &str) -> GatewayServerConfig {
    GatewayServerConfig {
        id: "mock".to_string(),
        display_name: "Mock".to_string(),
        enabled: true,
        scope: GatewayScope::User,
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
fn acp_gateway_elicitation_writes_events_and_redacts_url_query() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer.clone())
        .expect("core");

    // Downstream server behind gateway.
    let mock_mcp = env!("CARGO_BIN_EXE_mock-mcp-server").to_string();
    tamtri_core::config::replace_gateway_servers(
        temp.path(),
        vec![gateway_mock_server(&mock_mcp)],
    )
    .expect("save config");

    // ACP harness that calls the gateway's elicit_url tool.
    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");

    let conversation = core
        .create_conversation(
            "Elicit".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "go".to_string())
        .expect("send");

    // Wait for elicitation requested event.
    let (request_id, events_path) = wait_for_elicitation_requested(&observer, temp.path());

    // Assert run-path receipts are present before turn end.
    let live_events = fs::read_to_string(&events_path).expect("read events.jsonl");
    assert!(live_events.contains("\"kind\":\"elicitation_requested\""));
    assert!(!live_events.contains("client_id"));

    core.respond_elicitation(
        conversation.id.clone(),
        request_id.to_string(),
        "accept".to_string(),
        None,
    )
    .expect("respond elicitation");

    // The mock ACP agent also requests permission as part of ending its turn.
    let perm_request_id = wait_for_permission_requested(&observer);
    core.respond_permission(
        conversation.id.clone(),
        perm_request_id,
        "allow_once".to_string(),
    )
    .expect("permission");

    wait_for_turn_end(&observer);

    let text = fs::read_to_string(events_path).expect("read events.jsonl");
    let events: Vec<Value> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("event line"))
        .collect();

    let requested = events
        .iter()
        .find(|event| event["kind"] == "elicitation_requested")
        .expect("requested receipt");
    assert_eq!(requested["payload"]["origin_tool_call_id"], "acp-tool-1");
    let url = requested["payload"]["url"].as_str().unwrap_or_default();
    assert!(url.starts_with("https://example.com/"));
    assert!(!url.contains('?'));

    let resolved = events
        .iter()
        .find(|event| event["kind"] == "elicitation_resolved")
        .expect("resolved receipt");
    assert_eq!(resolved["payload"]["request_id"], requested["payload"]["request_id"]);
    assert_eq!(resolved["payload"]["action"], "accept");
}

#[test]
fn prepare_for_app_quit_writes_elicitation_cancel_receipts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer.clone())
        .expect("core");

    let mock_mcp = env!("CARGO_BIN_EXE_mock-mcp-server").to_string();
    tamtri_core::config::replace_gateway_servers(
        temp.path(),
        vec![gateway_mock_server(&mock_mcp)],
    )
    .expect("save config");

    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");

    let conversation = core
        .create_conversation(
            "Quit elicit".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "go".to_string())
        .expect("send");

    let (request_id, events_path) = wait_for_elicitation_requested(&observer, temp.path());
    wait_for_harness_spawned(&events_path);

    core.prepare_for_app_quit_inner().expect("prepare for quit");
    let _events = wait_for_elicitation_cancel_receipt(&events_path, &request_id);
}

fn wait_for_harness_spawned(events_path: &Path) {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for harness_spawned");
        }
        let text = fs::read_to_string(events_path).expect("read events.jsonl");
        if text.contains("\"kind\":\"harness_spawned\"") {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_elicitation_cancel_receipt(events_path: &Path, request_id: &str) -> Vec<Value> {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for elicitation_resolved cancel receipt");
        }
        let text = fs::read_to_string(events_path).expect("read events.jsonl");
        let events: Vec<Value> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("event line"))
            .collect();
        if let Some(resolved) = events
            .iter()
            .find(|event| event["kind"] == "elicitation_resolved")
        {
            assert_eq!(resolved["payload"]["request_id"], request_id);
            assert_eq!(resolved["payload"]["action"], "cancel");
            return events;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_elicitation_requested(
    observer: &RecordingObserver,
    vault_root: &Path,
) -> (String, std::path::PathBuf) {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for elicitation_requested");
        }
        let events = observer.events.lock().expect("events");
        if let Some(event) = events.iter().find(|event| event.kind == "elicitation_requested") {
            let payload: Value = serde_json::from_str(&event.payload_json).expect("payload");
            let request_id = payload["request_id"]
                .as_str()
                .expect("request_id")
                .to_string();
            drop(events);
            let events_path = find_file(vault_root, "events.jsonl").expect("events.jsonl");
            return (request_id, events_path);
        }
        drop(events);
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_permission_requested(observer: &RecordingObserver) -> String {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for permission_requested");
        }
        let events = observer.events.lock().expect("events");
        if let Some(event) = events.iter().find(|event| event.kind == "permission_requested") {
            let payload: Value = serde_json::from_str(&event.payload_json).expect("payload");
            let request_id = payload["request_id"]
                .as_str()
                .unwrap_or("perm-1")
                .to_string();
            return request_id;
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

