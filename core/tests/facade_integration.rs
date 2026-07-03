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
fn crash_mid_turn_loses_only_uncommitted() {
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
            "Crash".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "hello".to_string())
        .expect("send");
    wait_for_streaming_event(observer.as_ref());
    core.cancel_run(conversation.id.clone()).expect("cancel");
    wait_for_turn_end(observer.as_ref());
    std::thread::sleep(Duration::from_millis(200));

    let loaded = core.load_conversation(conversation.id).expect("load");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&loaded.transcript_json).expect("transcript");
    assert_eq!(messages.len(), 1, "vault should keep only the committed user message");
    assert_eq!(messages[0]["role"], "user");
}

#[test]
fn turn_commits_exactly_one_message() {
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
    assert_eq!(messages.len(), 2, "vault gains user + one assistant message");
    let assistant_messages: Vec<_> = messages
        .iter()
        .filter(|message| message["role"] == "assistant")
        .collect();
    assert_eq!(
        assistant_messages.len(),
        1,
        "vault gains exactly one assistant message per completed turn"
    );
    let assistant = assistant_messages[0];
    let content = assistant["content"].as_array().expect("content");
    let block_types: Vec<_> = content
        .iter()
        .filter_map(|block| block["type"].as_str())
        .collect();
    for expected in ["thinking", "text", "tool_call", "tool_result"] {
        assert!(
            block_types.contains(&expected),
            "missing committed block type: {expected}; got {block_types:?}"
        );
    }
    assert!(Path::new(temp.path()).join("conversations").exists());
}

#[test]
fn relaunch_transcript_from_messages_jsonl() {
    let temp = tempfile::tempdir().expect("tempdir");
    let vault_path = temp.path().to_string_lossy().into_owned();
    let (conversation_id, expected_assistant_content) = {
        let observer = Arc::new(RecordingObserver::default());
        let core = TamtriCore::new(vault_path.clone(), Arc::clone(&observer) as Arc<dyn ConversationObserver>)
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
                "Relaunch".to_string(),
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

        let loaded = core.load_conversation(conversation.id.clone()).expect("load");
        let before: Vec<serde_json::Value> =
            serde_json::from_str(&loaded.transcript_json).expect("transcript");
        assert_eq!(before.len(), 2);
        let assistant_content = before
            .iter()
            .find(|message| message["role"] == "assistant")
            .and_then(|message| message.get("content"))
            .cloned()
            .expect("assistant content");
        (conversation.id.clone(), assistant_content)
    };

    let observer = Arc::new(RecordingObserver::default());
    let reloaded_core = TamtriCore::new(vault_path, Arc::clone(&observer) as Arc<dyn ConversationObserver>)
        .expect("reloaded core");
    let loaded = reloaded_core
        .load_conversation(conversation_id)
        .expect("load after relaunch");
    let after: Vec<serde_json::Value> =
        serde_json::from_str(&loaded.transcript_json).expect("transcript");

    assert_eq!(after.len(), 2);
    assert_eq!(after[0]["role"], "user");
    assert_eq!(after[1]["role"], "assistant");
    assert_eq!(
        after[1]["content"], expected_assistant_content,
        "assistant content blocks should redraw from messages.jsonl alone"
    );
    assert!(
        observer.events.lock().expect("events").is_empty(),
        "relaunch redraw must not require a live stream"
    );
}

#[test]
fn referenced_only_paths_snapshot_without_file_changed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), Arc::clone(&observer) as Arc<dyn ConversationObserver>)
        .expect("core");
    core.register_acp_agent_with_env(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
        vec![tamtri_core::app::GatewayEnvVarDto {
            name: "MOCK_ACP_SKIP_FILE_CHANGED".to_string(),
            value: "1".to_string(),
        }],
    )
    .expect("agent");
    let conversation = core
        .create_conversation(
            "Referenced only".to_string(),
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

    let loaded = core.load_conversation(conversation.id.clone()).expect("load");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&loaded.transcript_json).expect("transcript");
    assert_eq!(messages.len(), 2);
    let assistant = &messages[1];
    let artifact = assistant["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["type"] == "artifact")
        .expect("artifact block from referenced_paths snapshot");
    assert_eq!(artifact["mime_type"], "text/html");
    assert!(artifact["inline"].as_str().unwrap().contains("<h1>ok</h1>"));
    let attachment_path = artifact["path"].as_str().unwrap();
    assert!(attachment_path.starts_with("attachments/"));
    assert!(
        find_file(temp.path(), attachment_path).is_some(),
        "attachment file should exist in vault"
    );

    let workdir_report = find_file(temp.path(), "workdir/report.html").unwrap();
    fs::write(workdir_report, "<h1>mutated</h1>").unwrap();
    let reloaded = core.load_conversation(conversation.id).expect("reload");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&reloaded.transcript_json).expect("transcript");
    let artifact = messages[1]["content"]
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
    assert_eq!(
        snapshotted.len(),
        1,
        "referenced-only snapshot should produce exactly one receipt"
    );
    assert!(
        snapshotted[0]["payload"]["tool_call_id"].is_null(),
        "referenced_paths snapshot should omit tool_call_id"
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
    assert_eq!(snapshotted[0]["payload"]["mime_type"].as_str(), Some("text/html"));
    assert!(snapshotted[0]["payload"]["sha256"].as_str().is_some());
}

#[test]
fn events_vs_transcript_permission_resolution() {
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
            "Permission audit".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    core.send_message(conversation.id.clone(), "hello".to_string())
        .expect("send");
    let request_id = wait_for_permission_requested(observer.as_ref());
    core.respond_permission(
        conversation.id.clone(),
        request_id.clone(),
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

    let requested = events
        .iter()
        .find(|event| event["kind"] == "permission_requested")
        .expect("permission_requested receipt");
    assert_eq!(requested["payload"]["request_id"], request_id);
    assert_eq!(requested["payload"]["action"], "edit");
    assert!(
        requested["payload"]["detail"]["diff"]["new_text"]
            .as_str()
            .is_some_and(|text| text.contains("<h1>ok</h1>")),
        "events.jsonl must carry full permission diff detail"
    );

    let resolved = events
        .iter()
        .find(|event| event["kind"] == "permission_resolved")
        .expect("permission_resolved receipt");
    assert_eq!(resolved["payload"]["request_id"], request_id);
    assert_eq!(resolved["payload"]["option_id"], "allow_once");

    let messages_path = find_file(temp.path(), "messages.jsonl").expect("messages");
    let messages_text = fs::read_to_string(messages_path).expect("read messages");
    let messages: Vec<serde_json::Value> = messages_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("message line"))
        .collect();
    let assistant = messages
        .iter()
        .find(|message| message["role"] == "assistant")
        .expect("assistant message");
    let permission_blocks: Vec<_> = assistant["content"]
        .as_array()
        .expect("content")
        .iter()
        .filter(|block| {
            block["type"] == "tool_result"
                && block["output"]["permission"].is_object()
        })
        .collect();
    assert_eq!(
        permission_blocks.len(),
        2,
        "transcript should carry compact permission request + resolution blocks"
    );
    for block in &permission_blocks {
        let permission = &block["output"]["permission"];
        assert!(permission.get("diff").is_none());
        assert!(permission.get("old_text").is_none());
        assert!(permission.get("new_text").is_none());
        assert!(permission.get("detail").is_none());
    }
    assert_eq!(
        permission_blocks[0]["output"]["permission"]["status"], "requested"
    );
    assert_eq!(
        permission_blocks[1]["output"]["permission"]["status"], "resolved"
    );
    assert_eq!(
        permission_blocks[1]["output"]["permission"]["selected_option"], "allow_once"
    );
}

#[test]
fn events_jsonl_receipts() {
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
        "tool_call_started",
        "tool_call_completed",
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
    let core = TamtriCore::new(
        temp.path().to_string_lossy().into_owned(),
        Arc::clone(&observer) as Arc<dyn ConversationObserver>,
    )
    .expect("core");

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
        env.push(("MOCK_MCP_EXIT_AFTER_LIST_COUNT".to_string(), "3".to_string()));
    }
    tamtri_core::config::replace_gateway_servers(temp.path(), vec![server]).expect("save config");
    core.set_gateway_credential("keychain://mock".to_string(), "super-secret".to_string())
        .expect("credential");

    core.register_acp_agent_with_env(
        "mock-acp".to_string(),
        "Mock ACP".to_string(),
        env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
        Vec::new(),
        vec![
            tamtri_core::app::GatewayEnvVarDto {
                name: "MOCK_ACP_CALL_FAIL_TOOL".to_string(),
                value: "1".to_string(),
            },
            tamtri_core::app::GatewayEnvVarDto {
                name: "MOCK_ACP_GATEWAY_RELIST".to_string(),
                value: "1".to_string(),
            },
        ],
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

    for expected in [
        "gateway_server_connected",
        "gateway_server_disconnected",
        "gateway_tool_routed",
        "gateway_credential_injected",
        "gateway_progress",
        "gateway_log",
        "gateway_downstream_error",
        "gateway_cancellation",
    ] {
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

    let disconnected = events
        .iter()
        .find(|event| event["kind"] == "gateway_server_disconnected")
        .expect("disconnect receipt");
    assert_eq!(disconnected["payload"]["server_id"], "mock");
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
            format!("Parent {}", uuid::Uuid::now_v7().simple()),
            "mock-acp".to_string(),
            "model-a".to_string(),
        )
        .expect("conversation");
    let parent_id = parent.id.clone();
    core.load_conversation(parent_id.clone())
        .expect("parent exists before fork");

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

    let listed = core.list_conversations().expect("list conversations");
    let listed_ids: Vec<_> = listed.iter().map(|s| s.id.as_str()).collect();
    assert!(
        listed.iter().any(|summary| summary.id == parent_id),
        "parent conversation should remain in vault after fork; listed={listed_ids:?} parent={parent_id}"
    );
    assert!(
        listed.iter().any(|summary| summary.id == fork.id),
        "fork conversation should be listed"
    );

    let reloaded_fork = core.load_conversation(fork.id.clone()).expect("reload fork");
    assert_eq!(
        reloaded_fork.active_harness_id.as_deref(),
        Some("other-acp")
    );
    assert_eq!(reloaded_fork.model_id.as_deref(), Some("model-b"));

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
fn seed_renders_prior_transcript() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer.clone()).expect("core");
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
            "Parent seed".to_string(),
            "mock-acp".to_string(),
            "model-a".to_string(),
        )
        .expect("conversation");
    let parent_id = parent.id.clone();

    core.send_message(parent_id.clone(), "parent context line".to_string())
        .expect("send parent");
    let request_id = wait_for_permission_requested(observer.as_ref());
    core.respond_permission(parent_id.clone(), request_id, "allow_once".to_string())
        .expect("permission");
    wait_for_turn_end(observer.as_ref());
    std::thread::sleep(Duration::from_millis(200));

    let fork = core
        .fork_conversation(
            parent_id,
            "other-acp".to_string(),
            "model-b".to_string(),
        )
        .expect("fork");

    core.send_message(fork.id.clone(), "fork follow up".to_string())
        .expect("send fork");
    let workdir = core
        .conversation_workdir_path(fork.id.clone())
        .expect("workdir");
    let seed_path = Path::new(&workdir).join(".session-prompt-seed.txt");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if seed_path.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(seed_path.is_file(), "mock ACP should write session/prompt seed");

    let seed = fs::read_to_string(&seed_path).expect("seed");
    assert!(
        seed.contains("parent context line"),
        "seed should include parent user message; seed={seed:?}"
    );
    assert!(
        seed.contains("fork follow up"),
        "seed should include the new user message; seed={seed:?}"
    );

    core.cancel_run(fork.id).ok();
}

#[test]
fn gateway_credential_reload_smoke_after_set_and_fresh_core() {
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
    tamtri_core::config::replace_gateway_servers(temp.path(), vec![server]).expect("save config");

    let servers = core.list_gateway_servers().expect("servers");
    assert_eq!(servers[0].missing_credential_refs, vec!["keychain://mock"]);

    core.set_gateway_credential("keychain://mock".to_string(), "first-value".to_string())
        .expect("set credential");
    assert_eq!(
        core.export_gateway_credential("keychain://mock".to_string())
            .expect("export"),
        Some("first-value".to_string())
    );

    let servers = core.refresh_gateway_capabilities().expect("refresh");
    assert!(servers[0].missing_credential_refs.is_empty());

    // Fresh core simulates relaunch before keychain preload (reloadGatewayServers).
    let observer2 = Arc::new(RecordingObserver::default());
    let reloaded = TamtriCore::new(
        temp.path().to_string_lossy().into_owned(),
        observer2,
    )
    .expect("reloaded core");
    let servers = reloaded.list_gateway_servers().expect("servers");
    assert_eq!(servers[0].missing_credential_refs, vec!["keychain://mock"]);

    reloaded
        .set_gateway_credential("keychain://mock".to_string(), "reloaded-value".to_string())
        .expect("reload credential");
    assert_eq!(
        reloaded
            .export_gateway_credential("keychain://mock".to_string())
            .expect("export after reload"),
        Some("reloaded-value".to_string())
    );
    let servers = reloaded.refresh_gateway_capabilities().expect("refresh");
    assert!(servers[0].missing_credential_refs.is_empty());
}

#[test]
fn copy_csv_to_workdir_without_harness_reference_does_not_create_artifact_block() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver::default());
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    let conversation = core
        .create_conversation(
            "Drop CSV".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let source = temp.path().join("sales.csv");
    fs::write(&source, "region,revenue\nNorth,10\n").expect("source");
    let copied = core
        .copy_file_to_workdir(conversation.id.clone(), source.to_string_lossy().into_owned())
        .expect("copy");
    assert_eq!(copied, "sales.csv");

    let loaded = core.load_conversation(conversation.id.clone()).expect("load");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&loaded.transcript_json).expect("transcript");
    assert!(messages.is_empty(), "copy alone should not append transcript messages");
    for message in &messages {
        let content = message["content"].as_array().cloned().unwrap_or_default();
        assert!(
            !content.iter().any(|block| block["type"] == "artifact"),
            "workdir copy without harness reference must not create artifact blocks"
        );
    }

    let workdir = core
        .conversation_workdir_path(conversation.id.clone())
        .expect("workdir");
    assert!(Path::new(&workdir).join("sales.csv").exists());

    let convo_dir = find_file(temp.path(), "meta.json")
        .expect("conversation folder")
        .parent()
        .expect("parent")
        .to_path_buf();
    assert!(
        !convo_dir.join("attachments").exists()
            || fs::read_dir(convo_dir.join("attachments"))
                .map(|mut dir| dir.next().is_none())
                .unwrap_or(true),
        "copy without harness reference must not snapshot attachments"
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

fn wait_for_streaming_event(observer: &RecordingObserver) {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for streaming harness event");
        }
        let events = observer.events.lock().expect("events");
        if events.iter().any(|event| {
            matches!(
                event.kind.as_str(),
                "text_delta" | "thought_delta" | "tool_call_started"
            )
        }) {
            return;
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
