use std::path::PathBuf;
use std::time::Duration;

use tamtri_core::conversation::{ContentBlock, Message, Role, WorkingDir};
use tamtri_core::harness::acp::{AdapterKind, AgentLaunchSpec};
use tamtri_core::harness::pi::PiNativeAdapter;
use tamtri_core::harness::{
    ContextSeed, ConversationContext, HarnessAdapter, HarnessEvent, TurnEndReason, TurnInput,
};

fn adapter_from_env() -> Option<PiNativeAdapter> {
    let command = std::env::var("TAMTRI_PI_COMMAND").ok()?;
    let args = std::env::var("TAMTRI_PI_ARGS")
        .ok()
        .map(|raw| raw.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    Some(PiNativeAdapter::new(AgentLaunchSpec {
        id: "pi-native".into(),
        display_name: "Pi".into(),
        command,
        args,
        env: Vec::new(),
        adapter: AdapterKind::PiNative,
        enabled: true,
    }))
}

fn ctx_in(path: PathBuf) -> ConversationContext {
    ConversationContext {
        seed: ContextSeed::FreshTranscript {
            messages: Vec::new(),
        },
        working_dir: WorkingDir::VaultLocal,
        working_dir_path: path,
        roots: Vec::new(),
        mcp_servers: Vec::new(),
        model_id: String::new(),
        native_session: None,
        tool_catalog: Vec::new(),
    }
}

fn user_turn(text: &str) -> TurnInput {
    TurnInput {
        user_message: Message {
            id: uuid::Uuid::now_v7(),
            role: Role::User,
            harness_id: None,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            created_at: chrono::Utc::now(),
        },
    }
}

#[tokio::test]
async fn pi_native_run_emits_text_and_turn_end() {
    let Some(adapter) = adapter_from_env() else {
        eprintln!("skip: set TAMTRI_PI_COMMAND to run Pi integration tests");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let mut run = adapter
        .run(
            ctx_in(temp.path().to_path_buf()),
            user_turn("Reply with exactly: tamtri-pi-ok"),
        )
        .await
        .expect("pi run");

    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    let mut saw_text = false;
    let mut saw_turn_end = false;

    while std::time::Instant::now() < deadline {
        tokio::select! {
            event = run.events.recv() => {
                let Some(event) = event else { break; };
                match event {
                    HarnessEvent::TextDelta { text } if text.contains("tamtri-pi-ok") => {
                        saw_text = true;
                    }
                    HarnessEvent::TurnEnded { reason: TurnEndReason::EndTurn } => {
                        saw_turn_end = true;
                        break;
                    }
                    HarnessEvent::TurnEnded { reason: TurnEndReason::Failed } => {
                        panic!("pi turn failed");
                    }
                    HarnessEvent::Error { message } => {
                        panic!("pi error: {message}");
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }

    assert!(saw_turn_end, "expected TurnEnded");
    assert!(saw_text, "expected assistant text containing tamtri-pi-ok");
}
