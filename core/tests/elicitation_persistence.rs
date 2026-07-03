use std::sync::Arc;

use chrono::Utc;
use serde_json::json;
use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::conversation::{
    ContentBlock, Conversation, ElicitationAction, ElicitationMode, Id, Message, Role,
};
use tamtri_core::vault::ConversationVault;
use tamtri_core::vault::fs::FilesystemVault;

#[derive(Default)]
struct NoopObserver;

impl ConversationObserver for NoopObserver {
    fn on_event(&self, _event: UiEvent) {}
}

#[test]
fn elicitation_persists_request_and_response_blocks() {
    let temp = tempfile::tempdir().unwrap();
    let vault = FilesystemVault::new(temp.path().to_path_buf()).unwrap();
    let mut conversation = Conversation::new("Elicitation");
    conversation.active_harness_id = Some("mock-acp".to_string());
    conversation.model_id = Some("mock".to_string());
    vault.create(&conversation).unwrap();

    let request = ContentBlock::ElicitationRequest {
        request_id: "req-1".to_string(),
        server_id: Some("mock".to_string()),
        origin_tool_call_id: Some("tool-9".to_string()),
        mode: ElicitationMode::Form,
        message: "What is your name?".to_string(),
        schema: Some(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        })),
        url: None,
    };
    let response = ContentBlock::ElicitationResponse {
        request_id: "req-1".to_string(),
        action: ElicitationAction::Accept,
        data: Some(json!({"name": "tamtri"})),
    };
    let message = Message {
        id: Id::now_v7(),
        role: Role::Assistant,
        harness_id: Some("mock-acp".to_string()),
        content: vec![request, response],
        created_at: Utc::now(),
    };
    vault.append_message(conversation.id, &message).unwrap();

    let loaded = vault.load(conversation.id).unwrap();
    assert_eq!(loaded.messages.len(), 1);
    assert_eq!(loaded.messages[0].content.len(), 2);
    match &loaded.messages[0].content[0] {
        ContentBlock::ElicitationRequest {
            origin_tool_call_id,
            ..
        } => assert_eq!(origin_tool_call_id.as_deref(), Some("tool-9")),
        other => panic!("expected request block, got {other:?}"),
    }
}

#[test]
fn reload_shows_historical_elicitation() {
    let temp = tempfile::tempdir().unwrap();
    let observer = Arc::new(NoopObserver);
    let core = TamtriCore::new(
        temp.path().to_string_lossy().into_owned(),
        observer,
    )
    .expect("core");
    let conversation = core
        .create_conversation(
            "History".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let vault = FilesystemVault::new(temp.path().to_path_buf()).unwrap();
    let id: Id = conversation.id.parse().expect("conversation id");
    let message = Message {
        id: Id::now_v7(),
        role: Role::Assistant,
        harness_id: Some("mock-acp".to_string()),
        content: vec![
            ContentBlock::ElicitationRequest {
                request_id: "req-history".to_string(),
                server_id: Some("mock".to_string()),
                origin_tool_call_id: None,
                mode: ElicitationMode::Url,
                message: "Sign in".to_string(),
                schema: None,
                url: Some("https://example.com/oauth".to_string()),
            },
            ContentBlock::ElicitationResponse {
                request_id: "req-history".to_string(),
                action: ElicitationAction::Decline,
                data: None,
            },
        ],
        created_at: Utc::now(),
    };
    vault.append_message(id, &message).unwrap();

    let loaded = core.load_conversation(conversation.id).expect("load");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&loaded.transcript_json).expect("transcript");
    assert_eq!(messages.len(), 1);
    let blocks = messages[0]["content"].as_array().unwrap();
    assert_eq!(blocks[0]["type"], "elicitation_request");
    assert_eq!(blocks[1]["type"], "elicitation_response");
    assert_eq!(blocks[1]["action"], "decline");
}
