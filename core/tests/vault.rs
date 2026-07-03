use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use pretty_assertions::assert_eq;
use serde_json::json;
use tamtri_core::conversation::{
    ARTIFACT_INLINE_MAX_BYTES, ContentBlock, Conversation, ElicitationAction, ElicitationMode,
    Message, Role, TaskStatus, message_from_line, message_to_line,
};
use tamtri_core::error::CoreError;
use tamtri_core::vault::events::{Event, EventKind};
use tamtri_core::vault::fs::FilesystemVault;
use tamtri_core::vault::{ConversationVault, VaultIssue};

fn vault() -> (tempfile::TempDir, FilesystemVault) {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = FilesystemVault::new(dir.path()).expect("vault");
    (dir, vault)
}

fn message(text: &str) -> Message {
    Message {
        id: uuid::Uuid::now_v7(),
        role: Role::User,
        harness_id: None,
        content: vec![ContentBlock::Text {
            text: text.to_string(),
        }],
        created_at: Utc::now(),
    }
}

fn all_blocks_message() -> Message {
    Message {
        id: uuid::Uuid::now_v7(),
        role: Role::Assistant,
        harness_id: Some("acp:test".to_string()),
        created_at: Utc::now(),
        content: vec![
            ContentBlock::Text { text: "hi".into() },
            ContentBlock::Thinking {
                text: "thinking".into(),
            },
            ContentBlock::ToolCall {
                id: "call-1".into(),
                name: "echo".into(),
                input: json!({"value": 1}),
            },
            ContentBlock::ToolResult {
                call_id: "call-1".into(),
                output: json!({"content": [{"type": "text", "text": "ok"}]}),
            },
            ContentBlock::AppResource {
                uri: "ui://app".into(),
                template_ref: "template".into(),
                state: json!({"open": true}),
                server_id: Some("mock".into()),
                origin_tool_call_id: Some("tool-1".into()),
            },
            ContentBlock::artifact(
                "attachments/report.html",
                "text/html",
                12,
                "abc",
                Some("<h1>ok</h1>".into()),
            )
            .unwrap(),
            ContentBlock::ElicitationRequest {
                request_id: "ask-1".into(),
                server_id: Some("mock".into()),
                origin_tool_call_id: Some("tool-1".into()),
                mode: ElicitationMode::Form,
                message: "confirm".into(),
                schema: Some(json!({"type": "object"})),
                url: None,
            },
            ContentBlock::ElicitationResponse {
                request_id: "ask-1".into(),
                action: ElicitationAction::Accept,
                data: Some(json!({"ok": true})),
            },
            ContentBlock::TaskRef {
                task_id: "task-1".into(),
                status: TaskStatus::Running,
                title: None,
                result_summary: None,
                origin_tool_call_id: Some("tool-1".into()),
            },
        ],
    }
}

fn conversation_dir(root: &Path, id: uuid::Uuid) -> std::path::PathBuf {
    fs::read_dir(root.join("conversations"))
        .expect("read conversations")
        .map(|entry| entry.expect("entry").path())
        .find(|path| {
            let meta = fs::read_to_string(path.join("meta.json")).expect("meta");
            meta.contains(&id.to_string())
        })
        .expect("conversation folder")
}

#[test]
fn meta_message_round_trip() {
    let (_dir, vault) = vault();
    let mut c = Conversation::new("Report From Data");
    c.push_message(all_blocks_message());

    vault.create(&c).unwrap();
    assert_eq!(vault.load(c.id).unwrap(), c);
}

#[test]
fn messages_jsonl_is_append_only() {
    let (dir, vault) = vault();
    let c = Conversation::new("Append");
    vault.create(&c).unwrap();
    let first = message("one");
    let second = message("two");
    let third = message("three");

    vault.append_message(c.id, &first).unwrap();
    vault.append_message(c.id, &second).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let before = fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = before.lines().collect();
    assert_eq!(lines.len(), 2);

    vault.append_message(c.id, &third).unwrap();
    let after = fs::read_to_string(path).unwrap();
    let after_lines: Vec<_> = after.lines().collect();
    assert_eq!(after_lines.len(), 3);
    assert_eq!(after_lines[0], lines[0]);
    assert_eq!(after_lines[1], lines[1]);
}

#[test]
fn meta_is_versioned() {
    let (dir, vault) = vault();
    let c = Conversation::new("Versioned");
    vault.create(&c).unwrap();
    let meta: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(conversation_dir(dir.path(), c.id).join("meta.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(meta["schema_version"], json!(1));
}

#[test]
fn load_rejects_future_version() {
    let (dir, vault) = vault();
    let c = Conversation::new("Future");
    vault.create(&c).unwrap();
    let meta_path = conversation_dir(dir.path(), c.id).join("meta.json");
    let mut meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
    meta["schema_version"] = json!(999);
    fs::write(meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

    assert!(matches!(
        vault.load(c.id),
        Err(CoreError::UnsupportedSchemaVersion(999))
    ));
}

#[test]
fn content_block_tagging() {
    for block in all_blocks_message().content {
        let value = serde_json::to_value(&block).unwrap();
        assert!(value.get("type").is_some());
        let round_trip: ContentBlock = serde_json::from_value(value).unwrap();
        assert_eq!(round_trip, block);
    }
}

#[test]
fn fork_new_id_and_backpointer() {
    let c = Conversation::new("Fork");
    let fork = c.fork();
    assert_ne!(fork.id, c.id);
    assert_eq!(fork.forked_from, Some(c.id));
}

#[test]
fn fork_is_deep_copy() {
    let mut c = Conversation::new("Fork copy");
    c.push_message(message("original"));
    let mut fork = c.fork();
    fork.push_message(message("fork"));
    assert_eq!(c.messages.len(), 1);
    assert_eq!(fork.messages.len(), 2);
}

#[test]
fn import_folder_as_new_assigns_new_id() {
    let (dir, source) = vault();
    let mut c = Conversation::new("Import me");
    c.forked_from = Some(uuid::Uuid::now_v7());
    c.push_message(message("hello"));
    source.create(&c).unwrap();
    let src = conversation_dir(dir.path(), c.id);

    let (_target_dir, target) = vault();
    let imported = target.import_folder_as_new(&src).unwrap();

    assert_ne!(imported.id, c.id);
    assert_eq!(imported.forked_from, None);
    assert_eq!(imported.messages, c.messages);
}

#[test]
fn import_rejects_malformed_artifact() {
    let (dir, source) = vault();
    let c = Conversation::new("Import traversal");
    source.create(&c).unwrap();
    let src = conversation_dir(dir.path(), c.id);
    let mut bad = message("bad");
    bad.content = vec![ContentBlock::Artifact {
        path: "attachments/../secrets.txt".into(),
        mime_type: "text/plain".into(),
        size: 1,
        sha256: "abc".into(),
        inline: Some("x".into()),
    }];
    let raw = serde_json::to_string(&bad).unwrap();
    fs::write(src.join("messages.jsonl"), format!("{raw}\n")).unwrap();

    let (_target_dir, target) = vault();
    assert!(matches!(
        target.import_folder_as_new(&src),
        Err(CoreError::MalformedVault(_))
    ));
}

#[test]
fn import_folder_as_new_copies_attachments() {
    let (dir, source) = vault();
    let c = Conversation::new("Import attachments");
    source.create(&c).unwrap();
    let src = conversation_dir(dir.path(), c.id);
    fs::write(
        src.join("attachments/report.html"),
        b"<h1>report</h1>",
    )
    .unwrap();

    let (target_dir, target) = vault();
    let imported = target.import_folder_as_new(&src).unwrap();
    let dst = conversation_dir(target_dir.path(), imported.id);

    assert_eq!(
        fs::read(dst.join("attachments/report.html")).unwrap(),
        b"<h1>report</h1>"
    );
}

#[test]
fn save_meta_round_trip() {
    let (_dir, vault) = vault();
    let mut c = Conversation::new("Save meta");
    vault.create(&c).unwrap();
    c.title = "Updated title".into();
    c.updated_at = Utc::now();
    vault.save_meta(&c).unwrap();

    let loaded = vault.load(c.id).unwrap();
    assert_eq!(loaded.title, "Updated title");
    assert_eq!(loaded.updated_at, c.updated_at);
}

#[test]
fn issues_reports_unreadable_folder() {
    let (dir, vault) = vault();
    let c = Conversation::new("Bad meta");
    vault.create(&c).unwrap();
    let conv_dir = conversation_dir(dir.path(), c.id);
    fs::write(conv_dir.join("meta.json"), "{not json").unwrap();

    assert!(vault.issues().unwrap().iter().any(|issue| {
        matches!(
            issue,
            VaultIssue::UnreadableFolder { path, .. } if *path == conv_dir
        )
    }));
}

#[test]
fn issues_reports_torn_tail() {
    let (dir, vault) = vault();
    let c = Conversation::new("Torn issue");
    vault.create(&c).unwrap();
    vault.append_message(c.id, &message("first")).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    write!(file, "{{\"id\":\"not done\"").unwrap();

    assert!(vault.issues().unwrap().iter().any(|issue| {
        matches!(issue, VaultIssue::TornTailDetected { id } if *id == c.id)
    }));
}

#[test]
fn vault_list_newest_first() {
    let (_dir, vault) = vault();
    let mut older = Conversation::new("Older");
    let mut newer = Conversation::new("Newer");
    let mut newest = Conversation::new("Newest");
    older.updated_at = Utc::now() - ChronoDuration::seconds(30);
    newer.updated_at = Utc::now() - ChronoDuration::seconds(20);
    newest.updated_at = Utc::now() - ChronoDuration::seconds(10);
    vault.create(&older).unwrap();
    vault.create(&newer).unwrap();
    vault.create(&newest).unwrap();

    let titles: Vec<_> = vault.list().unwrap().into_iter().map(|s| s.title).collect();
    assert_eq!(titles, vec!["Newest", "Newer", "Older"]);
}

#[test]
fn vault_delete_then_load_not_found() {
    let (_dir, vault) = vault();
    let c = Conversation::new("Delete");
    vault.create(&c).unwrap();
    vault.delete(c.id).unwrap();
    assert!(matches!(vault.load(c.id), Err(CoreError::NotFound(id)) if id == c.id));
}

#[test]
fn artifact_path_under_attachments() {
    assert!(
        ContentBlock::artifact(
            "attachments/a.txt",
            "text/plain",
            1,
            "abc",
            Some("x".into())
        )
        .is_ok()
    );
    assert!(
        ContentBlock::artifact("attachments", "text/plain", 1, "abc", Some("x".into())).is_err()
    );
    assert!(ContentBlock::artifact("/attachments/a.txt", "text/plain", 1, "abc", None).is_err());
}

#[test]
fn load_tolerates_torn_final_line() {
    let (dir, vault) = vault();
    let c = Conversation::new("Torn");
    vault.create(&c).unwrap();
    let first = message("first");
    vault.append_message(c.id, &first).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let before = fs::read(&path).unwrap();
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    write!(file, "{{\"id\":\"not done\"").unwrap();

    let loaded = vault.load(c.id).unwrap();
    assert_eq!(loaded.messages, vec![first]);
    assert_eq!(
        fs::read(&path).unwrap(),
        [before, b"{\"id\":\"not done\"".to_vec()].concat()
    );
}

#[test]
fn load_ignores_terminated_json_without_final_newline() {
    let (dir, vault) = vault();
    let c = Conversation::new("Torn parseable");
    vault.create(&c).unwrap();
    let first = message("first");
    vault.append_message(c.id, &first).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let before = fs::read(&path).unwrap();
    let parseable_but_unterminated = message_to_line(&message("not committed")).unwrap();
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    write!(file, "{parseable_but_unterminated}").unwrap();

    let loaded = vault.load(c.id).unwrap();
    assert_eq!(loaded.messages, vec![first]);
    assert_eq!(
        fs::read(&path).unwrap(),
        [before, parseable_but_unterminated.into_bytes()].concat()
    );
}

#[test]
fn append_repairs_torn_tail_on_disk() {
    let (dir, vault) = vault();
    let c = Conversation::new("Repair");
    vault.create(&c).unwrap();
    let first = message("first");
    let second = message("second");
    vault.append_message(c.id, &first).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    write!(file, "{{\"id\":\"not done\"").unwrap();

    vault.append_message(c.id, &second).unwrap();
    let text = fs::read_to_string(path).unwrap();
    let loaded: Vec<_> = text
        .lines()
        .map(|line| message_from_line(line).unwrap())
        .collect();
    assert_eq!(loaded, vec![first, second]);
}

#[test]
fn malformed_interior_line_is_hard_error() {
    let (dir, vault) = vault();
    let c = Conversation::new("Corrupt");
    vault.create(&c).unwrap();
    vault.append_message(c.id, &message("one")).unwrap();
    vault.append_message(c.id, &message("two")).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let mut text = fs::read_to_string(&path).unwrap();
    text.insert_str(text.find('\n').unwrap() + 1, "{bad}\n");
    fs::write(path, text).unwrap();

    assert!(matches!(
        vault.load(c.id),
        Err(CoreError::MalformedVault(_))
    ));
}

#[test]
fn renamed_folder_still_loads() {
    let (dir, vault) = vault();
    let c = Conversation::new("Rename");
    vault.create(&c).unwrap();
    let original = conversation_dir(dir.path(), c.id);
    let renamed = original.parent().unwrap().join("whatever-the-user-wants");
    fs::rename(original, renamed).unwrap();

    assert_eq!(vault.load(c.id).unwrap().id, c.id);
    assert_eq!(vault.list().unwrap()[0].id, c.id);
}

#[test]
fn duplicate_id_tiebreaks_by_folder_name() {
    let (dir, vault) = vault();
    let mut c = Conversation::new("Tiebreak");
    let ts = Utc::now();
    c.updated_at = ts;
    c.created_at = ts;
    vault.create(&c).unwrap();
    let original = conversation_dir(dir.path(), c.id);
    let copy_a = original.parent().unwrap().join("aaa-duplicate");
    let copy_b = original.parent().unwrap().join("zzz-duplicate");
    copy_dir_all(&original, &copy_a);
    copy_dir_all(&original, &copy_b);

    let expected_winner = [original.as_path(), copy_a.as_path(), copy_b.as_path()]
        .into_iter()
        .min()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    assert_eq!(vault.list().unwrap().len(), 1);
    assert_eq!(vault.load(c.id).unwrap().title, "Tiebreak");
    assert!(vault.issues().unwrap().iter().any(|issue| {
        matches!(
            issue,
            VaultIssue::DuplicateId { id, winner, losers }
                if *id == c.id
                    && winner.file_name().unwrap().to_string_lossy() == expected_winner
                    && losers.len() == 2
        )
    }));
}

#[test]
fn duplicate_id_resolves_to_newest() {
    let (dir, vault) = vault();
    let mut c = Conversation::new("Duplicate");
    c.updated_at = Utc::now() - ChronoDuration::seconds(60);
    vault.create(&c).unwrap();
    let original = conversation_dir(dir.path(), c.id);
    let copy = original.parent().unwrap().join("conflicted-copy");
    copy_dir_all(&original, &copy);

    let mut newer = c.clone();
    newer.title = "Duplicate newer".into();
    newer.updated_at = Utc::now();
    let meta_path = copy.join("meta.json");
    fs::write(
        meta_path,
        tamtri_core::conversation::ConversationMeta::from_conversation(&newer)
            .to_json_pretty()
            .unwrap(),
    )
    .unwrap();

    assert_eq!(vault.list().unwrap().len(), 1);
    assert_eq!(vault.load(c.id).unwrap().title, "Duplicate newer");
    assert!(vault.issues().unwrap().iter().any(|issue| {
        matches!(issue, VaultIssue::DuplicateId { id, losers, .. } if *id == c.id && losers.len() == 1)
    }));
}

#[test]
fn read_succeeds_while_write_lock_held() {
    let (dir, vault) = vault();
    let c = Conversation::new("Locked read");
    vault.create(&c).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.try_lock().unwrap();

    assert!(vault.load(c.id).is_ok());
    assert_eq!(vault.list().unwrap().len(), 1);
}

#[test]
fn contended_write_returns_busy() {
    let (dir, vault) = vault();
    let c = Conversation::new("Busy");
    vault.create(&c).unwrap();
    let path = conversation_dir(dir.path(), c.id).join("messages.jsonl");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.try_lock().unwrap();

    assert!(matches!(
        vault.append_message(c.id, &message("blocked")),
        Err(CoreError::ConversationBusy(id)) if id == c.id
    ));
}

#[test]
fn parallel_writes_to_different_conversations() {
    let (dir, vault) = vault();
    let a = Conversation::new("A");
    let b = Conversation::new("B");
    vault.create(&a).unwrap();
    vault.create(&b).unwrap();
    let va = vault.clone();
    let vb = vault.clone();
    let a_id = a.id;
    let b_id = b.id;

    let t1 = thread::spawn(move || va.append_message(a_id, &message("a")));
    let t2 = thread::spawn(move || vb.append_message(b_id, &message("b")));

    t1.join().unwrap().unwrap();
    t2.join().unwrap().unwrap();
    assert_eq!(
        FilesystemVault::new(dir.path())
            .unwrap()
            .load(a.id)
            .unwrap()
            .messages
            .len(),
        1
    );
    assert_eq!(
        FilesystemVault::new(dir.path())
            .unwrap()
            .load(b.id)
            .unwrap()
            .messages
            .len(),
        1
    );
}

#[test]
fn artifact_inline_respects_threshold() {
    assert!(
        ContentBlock::artifact(
            "attachments/too-big.txt",
            "text/plain",
            (ARTIFACT_INLINE_MAX_BYTES + 1) as u64,
            "abc",
            Some("x".repeat(ARTIFACT_INLINE_MAX_BYTES + 1)),
        )
        .is_err()
    );
    assert!(
        ContentBlock::artifact(
            "attachments/image.png",
            "image/png",
            1,
            "abc",
            Some("not text".to_string()),
        )
        .is_err()
    );

    let block = ContentBlock::Artifact {
        path: "attachments/too-big.txt".into(),
        mime_type: "text/plain".into(),
        size: 1,
        sha256: "abc".into(),
        inline: Some("x".repeat(ARTIFACT_INLINE_MAX_BYTES + 1)),
    };
    assert!(
        message_to_line(&Message {
            id: uuid::Uuid::now_v7(),
            role: Role::Assistant,
            harness_id: None,
            content: vec![block],
            created_at: Utc::now(),
        })
        .is_err()
    );
}

#[test]
fn artifact_inline_violation_rejected_on_load() {
    let (dir, vault) = vault();
    let c = Conversation::new("Bad artifact load");
    vault.create(&c).unwrap();
    let dir = conversation_dir(dir.path(), c.id);
    let mut bad = message("bad");
    bad.content = vec![ContentBlock::Artifact {
        path: "attachments/too-big.txt".into(),
        mime_type: "text/plain".into(),
        size: 1,
        sha256: "abc".into(),
        inline: Some("x".repeat(ARTIFACT_INLINE_MAX_BYTES + 1)),
    }];
    let raw = serde_json::to_string(&bad).unwrap();
    fs::write(dir.join("messages.jsonl"), format!("{raw}\n")).unwrap();

    assert!(matches!(
        vault.load(c.id),
        Err(CoreError::MalformedVault(_))
    ));
}

#[test]
fn artifact_path_traversal_rejected() {
    assert!(
        ContentBlock::artifact(
            "attachments/../secrets.txt",
            "text/plain",
            1,
            "abc",
            Some("x".into()),
        )
        .is_err()
    );
    assert!(
        ContentBlock::artifact(
            "attachments/./a.txt",
            "text/plain",
            1,
            "abc",
            Some("x".into()),
        )
        .is_err()
    );

    let (dir, vault) = vault();
    let c = Conversation::new("Traversal");
    vault.create(&c).unwrap();
    let dir = conversation_dir(dir.path(), c.id);
    let mut bad = message("bad");
    bad.content = vec![ContentBlock::Artifact {
        path: "attachments/../secrets.txt".into(),
        mime_type: "text/plain".into(),
        size: 1,
        sha256: "abc".into(),
        inline: Some("x".into()),
    }];
    let raw = serde_json::to_string(&bad).unwrap();
    fs::write(dir.join("messages.jsonl"), format!("{raw}\n")).unwrap();

    assert!(matches!(
        vault.load(c.id),
        Err(CoreError::MalformedVault(_))
    ));
}

#[test]
fn events_jsonl_receipts_append_and_read() {
    let (_dir, vault) = vault();
    let c = Conversation::new("Events");
    vault.create(&c).unwrap();
    let event = Event::new(
        EventKind::TurnStarted,
        serde_json::json!({"harness_id": "mock-acp"}),
    );

    vault.append_event(c.id, &event).unwrap();
    let events = vault.read_events(c.id).unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::TurnStarted);
    assert_eq!(events[0].payload["harness_id"], "mock-acp");
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
        }
    }
    thread::sleep(Duration::from_millis(2));
}
