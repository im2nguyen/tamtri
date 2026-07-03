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
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), Arc::clone(&observer) as Arc<dyn ConversationObserver>)
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
            "Run".to_string(),
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
    assert!(Path::new(temp.path()).join("conversations").exists());

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
    assert!(
        snapshotted.iter().any(|event| {
            event["payload"]["tool_call_id"].as_str() == Some("tool-1")
        }),
        "expected artifact_snapshotted receipt with tool_call_id"
    );
    let kinds: Vec<_> = events
        .iter()
        .map(|event| event["kind"].as_str().unwrap_or_default())
        .collect();
    for expected in [
        "turn_started",
        "harness_spawned",
        "permission_requested",
        "permission_resolved",
        "harness_exited",
        "turn_ended",
    ] {
        assert!(
            kinds.contains(&expected),
            "missing events.jsonl receipt: {expected}"
        );
    }
}

#[test]
fn events_jsonl_full_run_receipts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), Arc::clone(&observer) as Arc<dyn ConversationObserver>)
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
            "Receipts".to_string(),
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

    let events_path = find_file(temp.path(), "events.jsonl").expect("events");
    let events_text = fs::read_to_string(events_path).expect("read events");
    let events: Vec<serde_json::Value> = events_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("event line"))
        .collect();
    let kinds: Vec<_> = events
        .iter()
        .map(|event| event["kind"].as_str().unwrap_or_default())
        .collect();
    for expected in [
        "turn_started",
        "harness_spawned",
        "permission_requested",
        "permission_resolved",
        "harness_exited",
        "turn_ended",
    ] {
        assert!(
            kinds.contains(&expected),
            "missing events.jsonl receipt: {expected}"
        );
    }
}

#[test]
fn events_jsonl_gateway_receipts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");

    let mock_mcp = env!("CARGO_BIN_EXE_mock-mcp-server").to_string();
    let mut server = gateway_mock_server(&mock_mcp);
    server.credentials = vec![tamtri_core::config::CredentialBinding {
        credential_ref: "keychain://mock".to_string(),
        target: tamtri_core::config::CredentialTarget::EnvVar {
            name: "MOCK_TOKEN".to_string(),
        },
    }];
    if let tamtri_core::config::GatewayTransport::Stdio { ref mut env, .. } = server.transport {
        env.push(("MOCK_MCP_EMIT_PROGRESS".to_string(), "1".to_string()));
    }
    tamtri_core::config::replace_gateway_servers(temp.path(), vec![server]).expect("save config");
    core.set_gateway_credential("keychain://mock".to_string(), "super-secret".to_string())
        .expect("credential");

    core.register_acp_agent(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("agent");

    let conversation = core
        .create_conversation(
            "Gateway receipts".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "go".to_string())
        .expect("send");

    let workdir = core
        .conversation_workdir_path(conversation.id.clone())
        .expect("workdir");
    let marker = std::path::Path::new(&workdir).join(".gateway-echo-ok");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if marker.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(marker.is_file(), "gateway tool call did not complete");

    core.cancel_run(conversation.id).ok();
    std::thread::sleep(Duration::from_millis(300));

    let events_path = find_file(temp.path(), "events.jsonl").expect("events");
    let events_text = fs::read_to_string(events_path).expect("read events");
    assert!(
        !events_text.contains("super-secret"),
        "events.jsonl must not contain raw credential values"
    );

    let events: Vec<serde_json::Value> = events_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("event line"))
        .collect();
    let kinds: Vec<_> = events
        .iter()
        .map(|event| event["kind"].as_str().unwrap_or_default())
        .collect();

    for expected in ["gateway_tool_routed", "gateway_credential_injected"] {
        assert!(
            kinds.contains(&expected),
            "missing events.jsonl receipt: {expected}; kinds={kinds:?}"
        );
    }

    let injected = events
        .iter()
        .find(|event| event["kind"] == "gateway_credential_injected")
        .expect("credential receipt");
    assert_eq!(
        injected["payload"]["credential_ref"],
        "keychain://mock"
    );
    assert_eq!(injected["payload"]["target_kind"], "env_var");
}

#[test]
fn fork_into_harness_updates_model_and_harness() {
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
    core.register_acp_agent(
        "other-acp".to_string(),
        "Other ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
    )
    .expect("other agent");

    let parent = core
        .create_conversation(
            "Parent".to_string(),
            "mock-acp".to_string(),
            "model-a".to_string(),
        )
        .expect("conversation");
    let parent_id = parent.id.clone();

    let fork = core
        .fork_conversation(
            parent_id.clone(),
            "other-acp".to_string(),
            "model-b".to_string(),
        )
        .expect("fork");

    assert_ne!(fork.id, parent_id);
    assert_eq!(fork.active_harness_id.as_deref(), Some("other-acp"));
    assert_eq!(fork.model_id.as_deref(), Some("model-b"));
    assert_eq!(fork.forked_from.as_deref(), Some(parent_id.as_str()));

    let reloaded_parent = core.load_conversation(parent_id).expect("reload parent");
    assert_eq!(
        reloaded_parent.active_harness_id.as_deref(),
        Some("mock-acp")
    );
    assert_eq!(reloaded_parent.model_id.as_deref(), Some("model-a"));
    assert!(reloaded_parent.forked_from.is_none());

    let parent_messages: Vec<serde_json::Value> =
        serde_json::from_str(&reloaded_parent.transcript_json).expect("parent transcript");
    let fork_messages: Vec<serde_json::Value> =
        serde_json::from_str(&fork.transcript_json).expect("fork transcript");
    assert_eq!(fork_messages, parent_messages);
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

fn gateway_mock_server(command: &str) -> tamtri_core::config::GatewayServerConfig {
    tamtri_core::config::GatewayServerConfig {
        id: "mock".to_string(),
        display_name: "Mock".to_string(),
        enabled: true,
        scope: tamtri_core::config::GatewayScope::User,
        transport: tamtri_core::config::GatewayTransport::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        timeout_secs: Some(30),
        credentials: Vec::new(),
        oauth: None,
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
