use std::fs;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::conversation::{
    ContentBlock, Id, Message, Role, TaskStatus,
};
use tamtri_core::vault::ConversationVault;
use tamtri_core::vault::fs::FilesystemVault;

#[derive(Default)]
struct NoopObserver;

impl ConversationObserver for NoopObserver {
    fn on_event(&self, _event: UiEvent) {}
}

#[test]
fn tamtri_core_loads_task_ref_with_origin_tool_call_id() {
    let temp = tempfile::tempdir().unwrap();
    let observer = Arc::new(NoopObserver);
    let core = TamtriCore::new(
        temp.path().to_string_lossy().into_owned(),
        observer,
    )
    .expect("core");
    let conversation = core
        .create_conversation(
            "Task replay".to_string(),
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
            ContentBlock::ToolCall {
                id: "tool-2".to_string(),
                name: "tasks__progress_task".to_string(),
                input: serde_json::json!({}),
            },
            ContentBlock::TaskRef {
                task_id: "task-1".to_string(),
                status: TaskStatus::Completed,
                title: Some("Import CSV".to_string()),
                result_summary: Some("Done".to_string()),
                origin_tool_call_id: Some("tool-2".to_string()),
            },
        ],
        created_at: Utc::now(),
    };
    vault.append_message(id, &message).unwrap();

    let messages_path = find_file(temp.path(), "messages.jsonl").expect("messages.jsonl");
    let messages_text = fs::read_to_string(&messages_path).expect("read messages.jsonl");
    assert!(messages_text.contains("origin_tool_call_id"));
    assert!(messages_text.contains("tool-2"));

    let loaded = core.load_conversation(conversation.id).expect("load");
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&loaded.transcript_json).expect("transcript");
    assert_eq!(messages.len(), 1);
    let blocks = messages[0]["content"].as_array().unwrap();
    assert_eq!(blocks[1]["type"], "task_ref");
    assert_eq!(blocks[1]["origin_tool_call_id"], "tool-2");
    assert_eq!(blocks[1]["title"], "Import CSV");
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
