use std::fs;
use std::path::Path;
use std::sync::Arc;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::artifact::ArtifactSnapshotter;
use tamtri_core::harness::{Diff, FileChange};

struct RecordingObserver;

impl ConversationObserver for RecordingObserver {
    fn on_event(&self, _event: UiEvent) {}
}

fn find_file(root: &Path, name: &str) -> Option<std::path::PathBuf> {
    if root.file_name().is_some_and(|part| part == name) {
        return Some(root.to_path_buf());
    }
    if root.is_dir() {
        for entry in fs::read_dir(root).ok()? {
            let entry = entry.ok()?;
            if let Some(found) = find_file(&entry.path(), name) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn dropped_csv_stays_workdir_only() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(RecordingObserver);
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
    assert!(messages.is_empty());
    let convo_dir = find_file(temp.path(), "meta.json")
        .expect("conversation folder")
        .parent()
        .expect("parent")
        .to_path_buf();
    assert!(
        !convo_dir.join("attachments").exists()
            || fs::read_dir(convo_dir.join("attachments"))
                .map(|mut dir| dir.next().is_none())
                .unwrap_or(true)
    );
}

#[test]
fn report_html_snapshots_without_incidental_csv() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path().join("workdir");
    let convo = temp.path().join("conversation");
    fs::create_dir_all(&workdir).expect("workdir");
    fs::create_dir_all(convo.join("attachments")).expect("attachments");
    fs::write(workdir.join("sales.csv"), "a,b\n1,2\n").expect("csv");
    fs::write(workdir.join("report.html"), "<h1>report</h1>").expect("report");

    let snapshotter = ArtifactSnapshotter::new(&workdir, &convo);
    assert!(
        snapshotter
            .snapshot_file_changed(&Diff {
                path: "sales.csv".into(),
                change: FileChange::Modified,
                old_text: None,
                new_text: None,
            })
            .expect("csv snapshot")
            .is_none()
    );
    let report = snapshotter
        .snapshot_file_changed(&Diff {
            path: "report.html".into(),
            change: FileChange::Modified,
            old_text: None,
            new_text: None,
        })
        .expect("report snapshot")
        .expect("report artifact");
    assert_eq!(report.mime_type, "text/html");
    let snapshots = snapshotter
        .snapshot_referenced_paths(["report.html"])
        .expect("referenced");
    assert_eq!(snapshots.len(), 1);
}
