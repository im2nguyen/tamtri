use std::fs;
use std::sync::Arc;

use pretty_assertions::assert_eq;
use serde_json::json;
use tamtri_core::config::{GatewayConfig, GatewayScope, GatewayServerConfig, GatewayTransport};
use tamtri_core::conversation::{
    Conversation, Root, RootKind, RootOrigin, RootScope, attach_root,
    filesystem_root_requires_bookmark, is_path_under_any_root, missing_bookmark_error_state,
    remove_root,
};
use tamtri_core::mcp::gateway::{McpGateway, NoCredentials};
use tamtri_core::vault::ConversationVault;
use tamtri_core::vault::fs::FilesystemVault;

fn stdio_server(id: &str, command: &str) -> GatewayServerConfig {
    GatewayServerConfig {
        id: id.to_string(),
        display_name: id.to_string(),
        enabled: true,
        scope: GatewayScope::Project,
        transport: GatewayTransport::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        timeout_secs: None,
        credentials: Vec::new(),
        oauth: None,
    }
}

fn sample_root(uri: &str) -> Root {
    Root {
        id: "root-1".to_string(),
        name: "Data".to_string(),
        uri: uri.to_string(),
        kind: RootKind::Filesystem,
        scope: RootScope::Conversation,
        origin: RootOrigin::Conversation,
    }
}

#[test]
fn root_attach_persists_ref_not_bookmark() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = FilesystemVault::new(dir.path()).expect("vault");
    let mut conversation = Conversation::new("Roots");
    let root = attach_root(
        &mut conversation,
        "Reports",
        "/tmp/tamtri-reports",
        RootKind::Filesystem,
        RootScope::Conversation,
    )
    .expect("attach");
    vault.create(&conversation).expect("create");

    let loaded = vault.load(conversation.id).expect("load");
    assert_eq!(loaded.roots.len(), 1);
    assert_eq!(loaded.roots[0].id, root.id);
    assert!(loaded.roots[0].uri.starts_with("file://"));

    let conversation_dir = fs::read_dir(dir.path().join("conversations"))
        .expect("read conversations")
        .map(|entry| entry.expect("entry").path())
        .find(|path| path.join("meta.json").exists())
        .expect("conversation folder");
    let meta_raw = fs::read_to_string(conversation_dir.join("meta.json")).expect("meta");
    assert!(!meta_raw.contains("bookmark"));
    assert!(!meta_raw.contains("Bookmark"));
    assert!(meta_raw.contains("Reports"));
}

#[test]
fn root_missing_bookmark_surfaces_error_state() {
    let root = sample_root("file:///tmp/tamtri-data");
    assert!(filesystem_root_requires_bookmark(&root));

    let error_state = missing_bookmark_error_state(&root, false).expect("error state");
    assert!(error_state.contains("Re-pick"));
    assert!(error_state.contains("Data"));
    assert!(missing_bookmark_error_state(&root, true).is_none());

    let kb_root = Root {
        id: "kb-1".to_string(),
        name: "Docs".to_string(),
        uri: "kb://team/docs".to_string(),
        kind: RootKind::KnowledgeBase,
        scope: RootScope::Conversation,
        origin: RootOrigin::Conversation,
    };
    assert!(!filesystem_root_requires_bookmark(&kb_root));
    assert!(missing_bookmark_error_state(&kb_root, false).is_none());
}

#[tokio::test]
async fn roots_exposed_to_downstream_server() {
    let command = env!("CARGO_BIN_EXE_m7-roots-mcp");
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![stdio_server("m7roots", command)],
        },
        Arc::new(NoCredentials),
        None,
    )
    .unwrap();
    let temp = tempfile::tempdir().expect("tempdir");
    let root_path = temp.path().join("data");
    fs::create_dir_all(&root_path).expect("mkdir");
    let root_uri = format!("file://{}", root_path.to_string_lossy());
    gateway.set_roots(vec![sample_root(&root_uri)]).await;

    let result = gateway
        .call_tool("m7roots__probe_roots", json!({}))
        .await
        .expect("probe_roots");
    let structured = result.structured_content.expect("structured");
    let roots = structured["roots"]["roots"]
        .as_array()
        .expect("roots array");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0]["uri"], json!(root_uri));
    assert_eq!(roots[0]["name"], json!("Data"));
}

#[test]
fn fork_copies_root_refs() {
    let mut conversation = Conversation::new("parent");
    attach_root(
        &mut conversation,
        "Docs",
        "file:///tmp/docs",
        RootKind::Filesystem,
        RootScope::Conversation,
    )
    .expect("attach");
    let fork = conversation.fork();
    assert_eq!(fork.roots, conversation.roots);
}

#[test]
fn remove_root_updates_conversation() {
    let mut conversation = Conversation::new("remove");
    let root = attach_root(
        &mut conversation,
        "Docs",
        "file:///tmp/docs",
        RootKind::Filesystem,
        RootScope::Conversation,
    )
    .expect("attach");
    remove_root(&mut conversation, &root.id).expect("remove");
    assert!(conversation.roots.is_empty());
}

#[test]
fn path_outside_root_denied() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root_path = temp.path().join("allowed");
    fs::create_dir_all(&root_path).expect("mkdir");
    let root_uri = format!("file://{}", root_path.to_string_lossy());
    let roots = vec![sample_root(&root_uri)];
    assert!(
        is_path_under_any_root(&root_path.join("report.csv").to_string_lossy(), &roots).unwrap()
    );
    assert!(!is_path_under_any_root("/etc/passwd", &roots).unwrap());
}

#[tokio::test]
async fn roots_listed_emitted_when_downstream_lists_roots() {
    let command = env!("CARGO_BIN_EXE_m7-roots-mcp");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![stdio_server("m7roots", command)],
        },
        Arc::new(NoCredentials),
        Some(tx),
    )
    .unwrap();
    let temp = tempfile::tempdir().expect("tempdir");
    let root_path = temp.path().join("data");
    fs::create_dir_all(&root_path).expect("mkdir");
    let root_uri = format!("file://{}", root_path.to_string_lossy());
    gateway.set_roots(vec![sample_root(&root_uri)]).await;

    let gateway_for_call = Arc::new(gateway);
    let call_gateway = Arc::clone(&gateway_for_call);
    let call_task = tokio::spawn(async move {
        call_gateway
            .call_tool("m7roots__probe_roots", json!({}))
            .await
    });

    let listed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("gateway event");
            if let tamtri_core::mcp::gateway::GatewayEvent::RootsListed { server_id, count } = event
            {
                return (server_id, count);
            }
        }
    })
    .await
    .expect("roots_listed event timed out");

    assert_eq!(listed.0, "m7roots");
    assert_eq!(listed.1, 1);
    call_task.await.expect("call task").expect("probe_roots");
}

#[tokio::test]
async fn downstream_validate_path_respects_roots() {
    let command = env!("CARGO_BIN_EXE_m7-roots-mcp");
    let gateway = McpGateway::new(
        GatewayConfig {
            default_call_timeout_secs: 300,
            servers: vec![stdio_server("m7roots", command)],
        },
        Arc::new(NoCredentials),
        None,
    )
    .unwrap();
    let temp = tempfile::tempdir().expect("tempdir");
    let root_path = temp.path().join("data");
    fs::create_dir_all(&root_path).expect("mkdir");
    let inside = root_path.join("report.csv");
    fs::write(&inside, b"ok").expect("write");
    let root_uri = format!("file://{}", root_path.to_string_lossy());
    gateway.set_roots(vec![sample_root(&root_uri)]).await;

    let allowed = gateway
        .call_tool(
            "m7roots__validate_path",
            json!({"path": inside.to_string_lossy()}),
        )
        .await
        .expect("validate inside");
    assert_eq!(
        allowed.structured_content.expect("structured")["allowed"],
        json!(true)
    );

    let denied = gateway
        .call_tool("m7roots__validate_path", json!({"path": "/etc/passwd"}))
        .await
        .expect("validate outside");
    assert_eq!(
        denied.structured_content.expect("structured")["allowed"],
        json!(false)
    );
}
