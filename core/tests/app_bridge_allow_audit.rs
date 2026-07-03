use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::config::{GatewayScope, GatewayServerConfig, GatewayTransport};
use tamtri_core::mcp::app_bridge::APP_BRIDGE_ALLOW_ONCE;

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<UiEvent>>,
}

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, event: UiEvent) {
        self.events.lock().expect("events").push(event);
    }
}

fn m7_app_server(command: &str) -> GatewayServerConfig {
    GatewayServerConfig {
        id: "m7-app".to_string(),
        display_name: "M7 App".to_string(),
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
fn app_bridge_allow_routes_through_gateway_and_audits() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer.clone())
        .expect("core");

    tamtri_core::config::replace_gateway_servers(
        temp.path(),
        vec![m7_app_server(env!("CARGO_BIN_EXE_m7-app-mcp"))],
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
            "App bridge allow".to_string(),
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
                r#"{"jsonrpc":"2.0","id":"bridge-allow","method":"tools/call","params":{"name":"show_app","arguments":{}}}"#
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

    let request_id = wait_for_bridge_consent(&observer);
    core.respond_app_bridge_consent(
        conversation.id.clone(),
        request_id.clone(),
        APP_BRIDGE_ALLOW_ONCE.to_string(),
    )
    .expect("allow bridge");

    wait_for_bridge_resolved(&observer, &request_id);

    let events_path = find_file(temp.path(), "events.jsonl").expect("events.jsonl");
    let text = fs::read_to_string(events_path).expect("read events");
    let events: Vec<Value> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("event line"))
        .collect();

    let requested = events
        .iter()
        .find(|event| event["kind"] == "app_bridge_consent_requested")
        .expect("consent requested receipt");
    assert_eq!(requested["payload"]["request_id"], request_id);
    assert_eq!(requested["payload"]["server_id"], "m7-app");

    let resolved = events
        .iter()
        .find(|event| event["kind"] == "app_bridge_consent_resolved")
        .expect("consent resolved receipt");
    assert_eq!(resolved["payload"]["request_id"], request_id);
    assert_eq!(resolved["payload"]["resolution"], APP_BRIDGE_ALLOW_ONCE);

    let serialized = serde_json::to_string(&events).expect("serialize");
    assert!(!serialized.to_ascii_lowercase().contains("secret"));
    assert!(!serialized.to_ascii_lowercase().contains("password"));
    assert!(!serialized.to_ascii_lowercase().contains("token"));

    let kinds: Vec<&str> = events
        .iter()
        .filter_map(|event| event["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"app_bridge_consent_requested"));
    assert!(kinds.contains(&"app_bridge_consent_resolved"));
}

fn wait_for_bridge_consent(observer: &RecordingObserver) -> String {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for app bridge consent");
        }
        let events = observer.events.lock().expect("events");
        if let Some(event) = events
            .iter()
            .find(|event| event.kind == "app_bridge_consent_requested")
        {
            let payload: Value = serde_json::from_str(&event.payload_json).expect("payload");
            return payload["request_id"]
                .as_str()
                .expect("request_id")
                .to_string();
        }
        drop(events);
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_bridge_resolved(observer: &RecordingObserver, request_id: &str) {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for app bridge resolved");
        }
        let events = observer.events.lock().expect("events");
        let ready = events.iter().any(|event| {
            if event.kind != "app_bridge_resolved" {
                return false;
            }
            let payload: Value =
                serde_json::from_str(&event.payload_json).unwrap_or(Value::Null);
            payload["request_id"].as_str() == Some(request_id)
        });
        if ready {
            return;
        }
        drop(events);
        std::thread::sleep(Duration::from_millis(25));
    }
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
