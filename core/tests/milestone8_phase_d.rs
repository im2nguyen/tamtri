use std::fs;
use std::sync::Arc;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::conversation::Id;
use tamtri_core::vault::fs::FilesystemVault;

struct RecordingObserver;

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, _event: UiEvent) {}
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
    fs::create_dir_all(dst).expect("mkdir");
    for entry in fs::read_dir(src).expect("read") {
        let entry = entry.expect("entry");
        let target = dst.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_dir_all(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy");
        }
    }
}

#[test]
fn vault_duplicate_issue_badge() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver);
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    let conversation = core
        .create_conversation(
            "Duplicate badge".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let vault = FilesystemVault::new(temp.path()).expect("vault");
    let conversation_id: Id = conversation.id.parse().expect("uuid");
    let original = vault.conversation_folder(conversation_id).expect("folder");
    let copy = original
        .parent()
        .expect("parent")
        .join("sync-conflicted-copy");
    copy_dir_all(&original, &copy);

    let issues = core.vault_issues().expect("issues");
    assert!(issues.iter().any(|issue| issue.kind == "duplicate_id"));
    assert!(issues.iter().any(|issue| issue.detail.contains("Duplicate conversation id")));
}
