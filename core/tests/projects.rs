use std::fs;
use std::io::Read;
use std::sync::Arc;

use tamtri_core::app::{ConversationObserver, TamtriCore, UiEvent};
use tamtri_core::conversation::Conversation;
use tamtri_core::daemon::dispatch::dispatch;
use tamtri_core::project::{UNFILED_PROJECT_NAME, unfiled_project_id};
use tamtri_core::protocol::method;
use tamtri_core::vault::ConversationVault;
use tamtri_core::vault::fs::FilesystemVault;
use zip::ZipArchive;

struct NoopObserver;

impl ConversationObserver for NoopObserver {
    fn on_event(&self, _event: UiEvent) {}
}

fn core(path: &std::path::Path) -> TamtriCore {
    TamtriCore::new(path.to_string_lossy().into_owned(), Arc::new(NoopObserver)).expect("core")
}

#[test]
fn project_crud_and_legible_storage() {
    let temp = tempfile::tempdir().expect("tempdir");
    let core = core(temp.path());

    let projects = core.list_projects().expect("list");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, unfiled_project_id().to_string());
    assert_eq!(projects[0].name, UNFILED_PROJECT_NAME);
    assert!(
        core.attach_project_root(
            projects[0].id.clone(),
            "Not allowed".into(),
            "/tmp/tamtri-unfiled-root".into(),
            "filesystem".into(),
            "conversation".into(),
        )
        .is_err()
    );

    let project = core.create_project("Client Work".into()).expect("create");
    let project_dir = fs::read_dir(temp.path().join("projects"))
        .expect("projects dir")
        .map(|entry| entry.expect("entry").path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(&project.id.replace('-', "")))
        })
        .expect("project folder");
    assert!(project_dir.join("meta.json").is_file());
    let raw = fs::read_to_string(project_dir.join("meta.json")).expect("meta");
    assert!(raw.contains("\"name\": \"Client Work\""));
    assert!(raw.contains("\"roots\": []"));

    let updated = core
        .update_project(project.id.clone(), "Renamed".into())
        .expect("update");
    assert_eq!(updated.name, "Renamed");

    let root = core
        .attach_project_root(
            project.id.clone(),
            "Reports".into(),
            "/tmp/tamtri-project-reports".into(),
            "filesystem".into(),
            "conversation".into(),
        )
        .expect("attach root");
    assert_eq!(root.origin.as_deref(), Some("project"));
    core.remove_project_root(project.id.clone(), root.id)
        .expect("remove root");

    core.delete_project(project.id.clone()).expect("delete");
    assert!(
        core.list_projects()
            .expect("list")
            .iter()
            .all(|candidate| candidate.id != project.id)
    );
}

#[test]
fn legacy_conversation_projects_to_unfiled_without_rewrite() {
    let temp = tempfile::tempdir().expect("tempdir");
    let vault = FilesystemVault::new(temp.path()).expect("vault");
    let conversation = Conversation::new("Legacy");
    vault.create(&conversation).expect("create");
    let meta_path = vault
        .conversation_folder(conversation.id)
        .expect("folder")
        .join("meta.json");
    let before = fs::read_to_string(&meta_path)
        .expect("before")
        .replace("\"schema_version\": 4", "\"schema_version\": 3");
    fs::write(&meta_path, &before).expect("write legacy meta");
    assert!(!before.contains("project_id"));

    let core = core(temp.path());
    let loaded = core
        .load_conversation(conversation.id.to_string())
        .expect("load");
    let unfiled_id = unfiled_project_id().to_string();
    assert_eq!(loaded.project_id.as_deref(), Some(unfiled_id.as_str()));
    let after = fs::read_to_string(meta_path).expect("after");
    assert_eq!(after, before);
}

#[test]
fn effective_roots_propagate_and_dedupe() {
    let temp = tempfile::tempdir().expect("tempdir");
    let core = core(temp.path());
    let project = core.create_project("Research".into()).expect("project");
    core.attach_project_root(
        project.id.clone(),
        "Shared".into(),
        "/tmp/tamtri-shared".into(),
        "filesystem".into(),
        "conversation".into(),
    )
    .expect("shared root");
    let conversation = core
        .create_conversation_in_project(
            project.id.clone(),
            "Analysis".into(),
            "mock".into(),
            "default".into(),
        )
        .expect("conversation");
    core.attach_root(
        conversation.id.clone(),
        "Duplicate".into(),
        "/tmp/tamtri-shared".into(),
        "filesystem".into(),
        "conversation".into(),
    )
    .expect("duplicate");
    core.attach_root(
        conversation.id.clone(),
        "Local".into(),
        "/tmp/tamtri-local".into(),
        "filesystem".into(),
        "conversation".into(),
    )
    .expect("local");

    let roots = core.list_roots(conversation.id.clone()).expect("roots");
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].name, "Shared");
    assert_eq!(roots[0].origin.as_deref(), Some("project"));
    assert_eq!(roots[1].name, "Local");

    let fork = core
        .fork_conversation(conversation.id, "other".into(), "model".into())
        .expect("fork");
    assert_eq!(fork.project_id, Some(project.id));
    assert_eq!(core.list_roots(fork.id).expect("fork roots").len(), 2);
}

#[test]
fn deleting_project_moves_conversations_to_unfiled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let core = core(temp.path());
    let project = core.create_project("Temporary".into()).expect("project");
    let conversation = core
        .create_conversation_in_project(
            project.id.clone(),
            "Keep me".into(),
            "mock".into(),
            "default".into(),
        )
        .expect("conversation");

    core.delete_project(project.id).expect("delete");
    let loaded = core
        .load_conversation(conversation.id)
        .expect("still exists");
    assert_eq!(loaded.project_id, Some(unfiled_project_id().to_string()));
}

#[test]
fn deleting_project_preserves_inherited_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let core = core(temp.path());
    let project = core.create_project("Temporary".into()).expect("project");
    core.attach_project_root(
        project.id.clone(),
        "Shared".into(),
        "/tmp/tamtri-delete-shared".into(),
        "filesystem".into(),
        "conversation".into(),
    )
    .expect("shared root");
    let conversation = core
        .create_conversation_in_project(
            project.id.clone(),
            "Keep roots".into(),
            "mock".into(),
            "default".into(),
        )
        .expect("conversation");

    let roots_before = core.list_roots(conversation.id.clone()).expect("roots before");
    assert_eq!(roots_before.len(), 1);
    assert_eq!(roots_before[0].origin.as_deref(), Some("project"));

    core.delete_project(project.id).expect("delete");

    let roots_after = core.list_roots(conversation.id.clone()).expect("roots after");
    assert_eq!(roots_after.len(), 1);
    assert_eq!(roots_after[0].origin.as_deref(), Some("project_snapshot"));
    assert!(roots_after[0].uri.ends_with("/tmp/tamtri-delete-shared"));

    let loaded = core
        .load_conversation(conversation.id)
        .expect("still exists");
    assert_eq!(loaded.project_id, Some(unfiled_project_id().to_string()));
}

#[test]
fn export_materializes_project_roots_and_import_is_unfiled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let core = core(temp.path());
    let project = core.create_project("Portable".into()).expect("project");
    core.attach_project_root(
        project.id.clone(),
        "Shared".into(),
        "/tmp/tamtri-portable-shared".into(),
        "filesystem".into(),
        "conversation".into(),
    )
    .expect("shared root");
    let conversation = core
        .create_conversation_in_project(
            project.id,
            "Bundle".into(),
            "mock".into(),
            "default".into(),
        )
        .expect("conversation");
    let bundle = temp.path().join("portable.tamtri");
    core.export_conversation_bundle(conversation.id, bundle.to_string_lossy().into_owned())
        .expect("export");

    let file = fs::File::open(&bundle).expect("bundle");
    let mut archive = ZipArchive::new(file).expect("zip");
    let mut meta = String::new();
    archive
        .by_name("meta.json")
        .expect("meta")
        .read_to_string(&mut meta)
        .expect("read meta");
    let value: serde_json::Value = serde_json::from_str(&meta).expect("json");
    assert!(value.get("project_id").is_none());
    assert_eq!(value["roots"][0]["origin"], "project_snapshot");

    let imported = core
        .import_bundle_or_folder_as_new(bundle.to_string_lossy().into_owned())
        .expect("import");
    assert_eq!(
        imported.conversation.project_id,
        Some(unfiled_project_id().to_string())
    );
    let roots = core
        .list_roots(imported.conversation.id)
        .expect("import roots");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].origin.as_deref(), Some("project_snapshot"));
}

#[test]
fn json_rpc_dispatch_exposes_project_methods() {
    let temp = tempfile::tempdir().expect("tempdir");
    let core = Arc::new(core(temp.path()));
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let created = dispatch(
            Arc::clone(&core),
            method::PROJECT_CREATE,
            Some(serde_json::json!({ "name": "RPC project" })),
        )
        .await
        .expect("create dispatch");
        let project_id = created["id"].as_str().expect("project id");

        let conversation = dispatch(
            Arc::clone(&core),
            method::PROJECT_CONVERSATION_CREATE,
            Some(serde_json::json!({
                "project_id": project_id,
                "title": "RPC conversation",
                "harness_id": "mock",
                "model_id": "default"
            })),
        )
        .await
        .expect("conversation dispatch");
        assert_eq!(conversation["project_id"], project_id);

        let moved = dispatch(
            Arc::clone(&core),
            method::CONVERSATION_MOVE_PROJECT,
            Some(serde_json::json!({
                "conversation_id": conversation["id"],
                "project_id": unfiled_project_id().to_string()
            })),
        )
        .await
        .expect("move dispatch");
        assert_eq!(moved["project_id"], unfiled_project_id().to_string());

        let projects = dispatch(Arc::clone(&core), method::PROJECT_LIST, None)
            .await
            .expect("list dispatch");
        assert_eq!(projects.as_array().expect("projects").len(), 2);
    });
}
