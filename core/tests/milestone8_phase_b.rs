use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};

struct RecordingObserver;

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, _event: UiEvent) {}
}

#[test]
fn workdir_preview_reads_live_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver);
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    let conversation = core
        .create_conversation(
            "Live preview".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let source = temp.path().join("notes.txt");
    fs::write(&source, "draft v1").expect("source");
    let copied = core
        .copy_file_to_workdir(conversation.id.clone(), source.to_string_lossy().into_owned())
        .expect("copy");
    assert_eq!(copied, "notes.txt");

    let first = core
        .read_workdir_file(conversation.id.clone(), "notes.txt".to_string())
        .expect("first read");
    assert_eq!(String::from_utf8(first.data).expect("utf8"), "draft v1");

    let workdir = core
        .conversation_workdir_path(conversation.id.clone())
        .expect("workdir path");
    fs::write(Path::new(&workdir).join("notes.txt"), "draft v2").expect("rewrite");

    let second = core
        .read_workdir_file(conversation.id.clone(), "notes.txt".to_string())
        .expect("second read");
    assert_eq!(String::from_utf8(second.data).expect("utf8"), "draft v2");
}

#[test]
fn conversation_folder_path_resolves_for_loaded_conversation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver);
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    let conversation = core
        .create_conversation(
            "Folder path".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let folder = core
        .conversation_folder_path(conversation.id.clone())
        .expect("folder path");
    assert!(Path::new(&folder).join("meta.json").exists());
}

#[test]
fn busy_conversation_state_surfaces_on_contended_write() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver);
    let core = TamtriCore::new(temp.path().to_string_lossy().into_owned(), observer).expect("core");
    let conversation = core
        .create_conversation(
            "Busy".to_string(),
            "mock-acp".to_string(),
            "mock".to_string(),
        )
        .expect("conversation");

    let id = conversation.id.clone();
    let vault = temp.path().join("conversations");
    let folder = fs::read_dir(&vault)
        .expect("read conversations")
        .find_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let raw = fs::read_to_string(path.join("meta.json")).ok()?;
            raw.contains(&id).then_some(path)
        })
        .expect("conversation folder");
    let lock_path = folder.join("messages.jsonl");
    let lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("open messages");
    lock_file.lock().expect("hold lock");

    let core_for_thread = TamtriCore::new(temp.path().to_string_lossy().into_owned(), Arc::new(RecordingObserver))
        .expect("second core");
    core_for_thread
        .register_acp_agent(
            "mock-acp".to_string(),
            "Mock ACP".to_string(),
            env!("CARGO_BIN_EXE_mock-acp-agent").to_string(),
            vec![],
        )
        .expect("register agent");
    let id_for_thread = id.clone();
    let handle = thread::spawn(move || core_for_thread.send_message(id_for_thread, "hello".to_string()));

    thread::sleep(Duration::from_millis(100));
    let err = handle.join().expect("join").expect_err("busy");
    let message = err.to_string();
    assert!(
        message.contains("conversation is being written by another process"),
        "unexpected error: {message}"
    );

    drop(lock_file);
}
