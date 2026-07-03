use std::fs;
use std::path::{Path, PathBuf};

use tamtri_core::conversation::reduce::TurnReducer;
use tamtri_core::conversation::ContentBlock;
use tamtri_core::harness::HarnessEvent;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reducer")
}

fn load_events(path: &Path) -> Vec<HarnessEvent> {
    let raw = fs::read_to_string(path).expect("read events fixture");
    serde_json::from_str(&raw).expect("parse events fixture")
}

fn load_expected_blocks(path: &Path) -> Vec<ContentBlock> {
    let raw = fs::read_to_string(path).expect("read expected fixture");
    serde_json::from_str(&raw).expect("parse expected fixture")
}

fn reduce_fixture(case: &str) {
    let dir = fixtures_dir();
    let events = load_events(&dir.join(format!("{case}.events.json")));
    let expected = load_expected_blocks(&dir.join(format!("{case}.expected.json")));

    let mut reducer = TurnReducer::new("acp:test");
    for event in &events {
        reducer.apply(event).expect("apply harness event");
    }
    let reduced = reducer.finish();
    assert_eq!(reduced.message.content, expected, "fixture case: {case}");
}

#[test]
fn reducer_golden_files() {
    for case in [
        "text_thinking_collapse",
        "tool_call_pair",
        "file_changed_no_artifact",
        "permission_compact",
        "interleaved_thinking",
        "terminal_output",
        "harness_error",
        "lifecycle_no_blocks",
    ] {
        reduce_fixture(case);
    }
}
