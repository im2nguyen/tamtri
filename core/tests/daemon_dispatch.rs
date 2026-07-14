//! In-process parity tests for the daemon dispatcher: drive the wire methods
//! against a real `TamtriCore` over a temp vault and assert the results match
//! what the facade returns directly.

use serde_json::{json, Value};
use tamtri_core::daemon::Daemon;
use tamtri_core::protocol::method;
use tempfile::TempDir;

fn new_daemon() -> (Daemon, TempDir) {
    let dir = TempDir::new().expect("temp vault dir");
    let vault = dir.path().join("vault");
    let daemon = Daemon::new(vault.to_string_lossy().to_string()).expect("build daemon");
    (daemon, dir)
}

/// The core owns its own tokio runtime; dropping it from within an async context
/// panics. Drop it on a blocking thread, exactly as the daemon binary does on
/// shutdown.
async fn shutdown(daemon: Daemon) {
    tokio::task::spawn_blocking(move || drop(daemon))
        .await
        .expect("drop daemon off the async executor");
}

#[tokio::test]
async fn create_list_load_round_trip() {
    let (daemon, _dir) = new_daemon();

    let created = daemon
        .dispatch(
            method::CONVERSATION_CREATE,
            Some(json!({
                "title": "Report from CSV",
                "harness_id": "mock-acp",
                "model_id": "mock",
            })),
        )
        .await
        .expect("create conversation");
    let id = created["id"].as_str().expect("created id").to_string();
    assert_eq!(created["title"], json!("Report from CSV"));

    let list = daemon
        .dispatch(method::CONVERSATION_LIST, None)
        .await
        .expect("list conversations");
    let ids: Vec<&str> = list
        .as_array()
        .expect("list is array")
        .iter()
        .filter_map(|row| row["id"].as_str())
        .collect();
    assert!(ids.contains(&id.as_str()), "created conversation appears in list");

    let loaded = daemon
        .dispatch(method::CONVERSATION_LOAD, Some(json!({ "id": id })))
        .await
        .expect("load conversation");
    assert_eq!(loaded["id"], created["id"]);
    assert_eq!(loaded["title"], json!("Report from CSV"));

    shutdown(daemon).await;
}

#[tokio::test]
async fn unknown_method_is_method_not_found() {
    let (daemon, _dir) = new_daemon();
    let err = daemon
        .dispatch("does.not.exist", None)
        .await
        .expect_err("unknown method should error");
    assert_eq!(err.code, -32601);

    shutdown(daemon).await;
}

#[tokio::test]
async fn invalid_params_are_rejected() {
    let (daemon, _dir) = new_daemon();
    // conversation.load requires an `id`; sending the wrong shape is a param error.
    let err = daemon
        .dispatch(method::CONVERSATION_LOAD, Some(json!({ "wrong": "field" })))
        .await
        .expect_err("missing id should error");
    assert_eq!(err.code, -32602);

    shutdown(daemon).await;
}

#[tokio::test]
async fn infallible_scope_message_dispatches() {
    let (daemon, _dir) = new_daemon();
    let value = daemon
        .dispatch(method::SEARCH_SCOPE_MESSAGE, None)
        .await
        .expect("scope message");
    assert!(matches!(value, Value::String(_)));

    shutdown(daemon).await;
}

#[tokio::test]
async fn vault_path_dispatches() {
    let (daemon, _dir) = new_daemon();
    let value = daemon
        .dispatch(method::VAULT_PATH, None)
        .await
        .expect("vault path");
    assert!(value.as_str().is_some_and(|p| p.contains("vault")));

    shutdown(daemon).await;
}
