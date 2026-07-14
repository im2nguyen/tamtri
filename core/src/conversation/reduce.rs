use chrono::Utc;
use serde_json::json;

use crate::Result;
use crate::conversation::{ContentBlock, Id, Message, Role};
use crate::harness::{
    Diff, FileChange, HarnessEvent, PermissionDetail, ToolContent, ToolKind, ToolStatus,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedFileChange {
    pub tool_call_id: String,
    pub diff: Diff,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReducedTurn {
    pub message: Message,
    pub file_changes: Vec<RecordedFileChange>,
    pub referenced_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TurnReducer {
    harness_id: String,
    blocks: Vec<ContentBlock>,
    text_buffer: String,
    thought_buffer: String,
    file_changes: Vec<RecordedFileChange>,
    referenced_paths: Vec<String>,
}

impl TurnReducer {
    pub fn new(harness_id: impl Into<String>) -> Self {
        Self {
            harness_id: harness_id.into(),
            blocks: Vec::new(),
            text_buffer: String::new(),
            thought_buffer: String::new(),
            file_changes: Vec::new(),
            referenced_paths: Vec::new(),
        }
    }

    pub fn apply(&mut self, event: &HarnessEvent) -> Result<()> {
        match event {
            HarnessEvent::TextDelta { text } => {
                self.flush_thought();
                self.text_buffer.push_str(text);
            }
            HarnessEvent::ThoughtDelta { text } => {
                self.flush_text();
                self.thought_buffer.push_str(text);
            }
            HarnessEvent::ToolCallStarted {
                id,
                name,
                kind,
                input,
                ..
            } => {
                self.flush_deltas();
                if matches!(kind, ToolKind::Write | ToolKind::Edit)
                    && let Some(path) = path_from_tool_input(input)
                {
                    self.push_referenced_path(&path);
                }
                self.blocks.push(ContentBlock::ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
            }
            HarnessEvent::ToolCallProgress {
                id,
                status,
                content,
            } => {
                self.flush_deltas();
                if matches!(status, ToolStatus::Completed | ToolStatus::Failed) {
                    for path in renderable_paths_from_tool_content(content) {
                        self.push_referenced_path(&path);
                    }
                    self.blocks.push(ContentBlock::ToolResult {
                        call_id: id.clone(),
                        output: tool_output(content, status),
                    });
                }
            }
            HarnessEvent::FileChanged {
                tool_call_id, diff, ..
            } => {
                if diff.change != FileChange::Deleted {
                    self.push_referenced_path(&diff.path);
                }
                self.file_changes.push(RecordedFileChange {
                    tool_call_id: tool_call_id.clone(),
                    diff: diff.clone(),
                });
            }
            HarnessEvent::PermissionRequested {
                request_id,
                action,
                detail,
                options,
                ..
            } => {
                self.flush_deltas();
                if let PermissionDetail::FileEdit { diff } = detail
                    && diff.change != FileChange::Deleted
                {
                    self.push_referenced_path(&diff.path);
                }
                self.blocks.push(ContentBlock::ToolResult {
                    call_id: request_id.clone(),
                    output: json!({
                        "permission": {
                            "action": action,
                            "options": options,
                            "status": "requested"
                        }
                    }),
                });
            }
            HarnessEvent::PermissionResolved {
                request_id,
                option_id,
            } => {
                self.flush_deltas();
                self.blocks.push(ContentBlock::ToolResult {
                    call_id: request_id.clone(),
                    output: json!({
                        "permission": {
                            "selected_option": option_id,
                            "status": "resolved"
                        }
                    }),
                });
            }
            HarnessEvent::TerminalOutput {
                tool_call_id,
                chunk,
            } => {
                self.flush_deltas();
                self.blocks.push(ContentBlock::ToolResult {
                    call_id: tool_call_id.clone(),
                    output: json!({ "terminal": chunk }),
                });
            }
            HarnessEvent::Error { message } => {
                self.flush_deltas();
                self.blocks.push(ContentBlock::ToolResult {
                    call_id: "harness_error".to_string(),
                    output: json!({ "error": message }),
                });
            }
            HarnessEvent::TurnEnded { .. }
            | HarnessEvent::PlanUpdated { .. }
            | HarnessEvent::ModeChanged { .. }
            | HarnessEvent::NativeSessionBound { .. } => {
                self.flush_deltas();
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> ReducedTurn {
        self.flush_deltas();
        ReducedTurn {
            message: Message {
                id: Id::now_v7(),
                role: Role::Assistant,
                harness_id: Some(self.harness_id),
                content: self.blocks,
                created_at: Utc::now(),
            },
            file_changes: self.file_changes,
            referenced_paths: self.referenced_paths,
        }
    }

    fn push_referenced_path(&mut self, path: &str) {
        if !crate::artifact::is_deliverable_snapshot_path(path) {
            return;
        }
        if !self
            .referenced_paths
            .iter()
            .any(|existing| existing == path)
        {
            self.referenced_paths.push(path.to_string());
        }
    }

    fn flush_deltas(&mut self) {
        self.flush_text();
        self.flush_thought();
    }

    fn flush_text(&mut self) {
        if !self.text_buffer.is_empty() {
            self.blocks.push(ContentBlock::Text {
                text: std::mem::take(&mut self.text_buffer),
            });
        }
    }

    fn flush_thought(&mut self) {
        if !self.thought_buffer.is_empty() {
            self.blocks.push(ContentBlock::Thinking {
                text: std::mem::take(&mut self.thought_buffer),
            });
        }
    }
}

fn tool_output(content: &[ToolContent], status: &ToolStatus) -> serde_json::Value {
    json!({
        "status": status,
        "content": content,
    })
}

fn renderable_paths_from_tool_content(content: &[ToolContent]) -> Vec<String> {
    let mut paths = Vec::new();
    for item in content {
        match item {
            ToolContent::Diff { diff } if diff.change != FileChange::Deleted => {
                paths.push(diff.path.clone());
            }
            ToolContent::ResourceRef { uri } => {
                if let Some(path) = workdir_path_from_resource_uri(uri) {
                    paths.push(path);
                }
            }
            _ => {}
        }
    }
    paths
}

fn path_from_tool_input(input: &serde_json::Value) -> Option<String> {
    for key in ["path", "file_path", "filePath"] {
        let Some(path) = input.get(key).and_then(serde_json::Value::as_str) else {
            continue;
        };
        if is_relative_workdir_path(path) {
            return Some(path.to_string());
        }
    }
    None
}

fn is_relative_workdir_path(path: &str) -> bool {
    let parsed = std::path::Path::new(path);
    if parsed.is_absolute() {
        return false;
    }
    for component in parsed.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return false;
        }
    }
    !path.is_empty()
}

fn workdir_path_from_resource_uri(uri: &str) -> Option<String> {
    let path = uri.strip_prefix("file://")?;
    let path = path.strip_prefix("localhost").unwrap_or(path);
    let path = path.trim_start_matches('/');
    let parsed = std::path::Path::new(path);
    if parsed.is_absolute() {
        return None;
    }
    for component in parsed.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return None;
        }
    }
    if path.is_empty() {
        return None;
    }
    Some(path.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::harness::{Diff, FileChange, ToolContent};

    #[test]
    fn reducer_collapses_text_and_thinking_deltas() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ThoughtDelta { text: "a".into() })
            .unwrap();
        reducer
            .apply(&HarnessEvent::ThoughtDelta { text: "b".into() })
            .unwrap();
        reducer
            .apply(&HarnessEvent::TextDelta { text: "hi".into() })
            .unwrap();
        reducer
            .apply(&HarnessEvent::TextDelta { text: "!".into() })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(
            reduced.message.content,
            vec![
                ContentBlock::Thinking { text: "ab".into() },
                ContentBlock::Text { text: "hi!".into() }
            ]
        );
    }

    #[test]
    fn reducer_pairs_tool_call_and_result() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallStarted {
                id: "tool-1".into(),
                name: "echo".into(),
                kind: crate::harness::ToolKind::Other("echo".into()),
                title: "Echo".into(),
                input: json!({"x": 1}),
            })
            .unwrap();
        reducer
            .apply(&HarnessEvent::ToolCallProgress {
                id: "tool-1".into(),
                status: ToolStatus::Completed,
                content: vec![ToolContent::Text { text: "ok".into() }],
            })
            .unwrap();
        let reduced = reducer.finish();
        assert!(matches!(
            &reduced.message.content[0],
            ContentBlock::ToolCall { id, .. } if id == "tool-1"
        ));
        assert!(matches!(
            &reduced.message.content[1],
            ContentBlock::ToolResult { call_id, .. } if call_id == "tool-1"
        ));
    }

    #[test]
    fn reducer_preserves_multiline_execute_tool_text() {
        let output_text =
            "Execution complete\n\nOutput:\nsales.csv rows: 30\nexternal_refs_found: False";
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallProgress {
                id: "tool-1".into(),
                status: ToolStatus::Completed,
                content: vec![ToolContent::Text {
                    text: output_text.to_string(),
                }],
            })
            .unwrap();
        let reduced = reducer.finish();
        let ContentBlock::ToolResult { output, .. } = &reduced.message.content[0] else {
            panic!("expected tool result");
        };
        assert_eq!(output["content"][0]["text"].as_str(), Some(output_text));
    }

    #[test]
    fn reducer_records_file_changed_without_artifact_block() {
        let diff = Diff {
            path: "report.html".into(),
            change: FileChange::Modified,
            old_text: None,
            new_text: Some("<h1>ok</h1>".into()),
        };
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::FileChanged {
                tool_call_id: "tool-1".into(),
                path: diff.path.clone(),
                change: diff.change.clone(),
                diff: diff.clone(),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(
            reduced.file_changes,
            vec![RecordedFileChange {
                tool_call_id: "tool-1".into(),
                diff: diff.clone(),
            }]
        );
        assert_eq!(reduced.referenced_paths, vec!["report.html".to_string()]);
        assert!(reduced.message.content.is_empty());
    }

    #[test]
    fn reducer_interleaves_thinking_and_text_blocks() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ThoughtDelta { text: "hmm".into() })
            .unwrap();
        reducer
            .apply(&HarnessEvent::TextDelta {
                text: "Hello".into(),
            })
            .unwrap();
        reducer
            .apply(&HarnessEvent::ThoughtDelta {
                text: " wait".into(),
            })
            .unwrap();
        reducer
            .apply(&HarnessEvent::TextDelta {
                text: " world".into(),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(
            reduced.message.content,
            vec![
                ContentBlock::Thinking { text: "hmm".into() },
                ContentBlock::Text {
                    text: "Hello".into()
                },
                ContentBlock::Thinking {
                    text: " wait".into()
                },
                ContentBlock::Text {
                    text: " world".into()
                },
            ]
        );
    }

    #[test]
    fn reducer_permission_compact_form_omits_full_diff() {
        let diff = Diff {
            path: "report.html".into(),
            change: FileChange::Modified,
            old_text: Some("<h1>old</h1>".into()),
            new_text: Some("<h1>new</h1>".into()),
        };
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::PermissionRequested {
                request_id: "perm-1".into(),
                action: "edit".into(),
                detail: crate::harness::PermissionDetail::FileEdit { diff: diff.clone() },
                options: vec![crate::harness::PermissionOption {
                    id: "allow-once".into(),
                    label: "Allow once".into(),
                }],
            })
            .unwrap();
        reducer
            .apply(&HarnessEvent::PermissionResolved {
                request_id: "perm-1".into(),
                option_id: "allow-once".into(),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.message.content.len(), 2);
        for block in &reduced.message.content {
            let ContentBlock::ToolResult { output, .. } = block else {
                panic!("expected permission tool result");
            };
            assert!(output.get("permission").is_some());
            assert!(output.get("diff").is_none());
            assert!(output["permission"].get("diff").is_none());
            assert!(output["permission"].get("old_text").is_none());
            assert!(output["permission"].get("new_text").is_none());
        }
        assert_eq!(
            reduced.message.content[0],
            ContentBlock::ToolResult {
                call_id: "perm-1".into(),
                output: json!({
                    "permission": {
                        "action": "edit",
                        "options": [{"id": "allow-once", "label": "Allow once"}],
                        "status": "requested"
                    }
                }),
            }
        );
        assert_eq!(
            reduced.message.content[1],
            ContentBlock::ToolResult {
                call_id: "perm-1".into(),
                output: json!({
                    "permission": {
                        "selected_option": "allow-once",
                        "status": "resolved"
                    }
                }),
            }
        );
    }

    #[test]
    fn reducer_collects_paths_from_write_tool_input() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallStarted {
                id: "tool-1".into(),
                name: "Write".into(),
                kind: ToolKind::Write,
                title: "Write report".into(),
                input: json!({"path": "report.html"}),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.referenced_paths, vec!["report.html".to_string()]);
    }

    #[test]
    fn reducer_collects_paths_from_write_tool_input_file_path_key() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallStarted {
                id: "tool-1".into(),
                name: "Write".into(),
                kind: ToolKind::Write,
                title: "Write report".into(),
                input: json!({"file_path": "report.html"}),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.referenced_paths, vec!["report.html".to_string()]);
    }

    #[test]
    fn reducer_collects_paths_from_write_tool_input_file_path_camel_case_key() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallStarted {
                id: "tool-1".into(),
                name: "Write".into(),
                kind: ToolKind::Write,
                title: "Write report".into(),
                input: json!({"filePath": "report.html"}),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.referenced_paths, vec!["report.html".to_string()]);
    }

    #[test]
    fn reducer_collects_renderable_paths_from_completed_tool_output() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallProgress {
                id: "tool-1".into(),
                status: ToolStatus::Completed,
                content: vec![ToolContent::Diff {
                    diff: Diff {
                        path: "report.html".into(),
                        change: FileChange::Created,
                        old_text: None,
                        new_text: Some("<h1>ok</h1>".into()),
                    },
                }],
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.referenced_paths, vec!["report.html".to_string()]);
    }

    #[test]
    fn reducer_collects_paths_from_edit_tool_input() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallStarted {
                id: "tool-1".into(),
                name: "Edit".into(),
                kind: ToolKind::Edit,
                title: "Edit report".into(),
                input: json!({"path": "notes.md"}),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.referenced_paths, vec!["notes.md".to_string()]);
    }

    #[test]
    fn reducer_collects_paths_from_resource_ref_in_tool_output() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallProgress {
                id: "tool-1".into(),
                status: ToolStatus::Completed,
                content: vec![ToolContent::ResourceRef {
                    uri: "file://nested/report.html".into(),
                }],
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(
            reduced.referenced_paths,
            vec!["nested/report.html".to_string()]
        );
    }

    #[test]
    fn reducer_skips_deleted_diff_in_tool_output() {
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallProgress {
                id: "tool-1".into(),
                status: ToolStatus::Completed,
                content: vec![ToolContent::Diff {
                    diff: Diff {
                        path: "report.html".into(),
                        change: FileChange::Deleted,
                        old_text: Some("<h1>ok</h1>".into()),
                        new_text: None,
                    },
                }],
            })
            .unwrap();
        let reduced = reducer.finish();
        assert!(reduced.referenced_paths.is_empty());
    }

    #[test]
    fn reducer_skips_deleted_file_changed_path() {
        let diff = Diff {
            path: "report.html".into(),
            change: FileChange::Deleted,
            old_text: Some("<h1>ok</h1>".into()),
            new_text: None,
        };
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::FileChanged {
                tool_call_id: "tool-1".into(),
                path: diff.path.clone(),
                change: diff.change.clone(),
                diff: diff.clone(),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert!(reduced.referenced_paths.is_empty());
        assert_eq!(reduced.file_changes.len(), 1);
    }

    #[test]
    fn reducer_deduplicates_referenced_paths_from_multiple_sources() {
        let diff = Diff {
            path: "report.html".into(),
            change: FileChange::Modified,
            old_text: None,
            new_text: Some("<h1>ok</h1>".into()),
        };
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::ToolCallStarted {
                id: "tool-1".into(),
                name: "Write".into(),
                kind: ToolKind::Write,
                title: "Write report".into(),
                input: json!({"path": "report.html"}),
            })
            .unwrap();
        reducer
            .apply(&HarnessEvent::ToolCallProgress {
                id: "tool-1".into(),
                status: ToolStatus::Completed,
                content: vec![ToolContent::Diff { diff: diff.clone() }],
            })
            .unwrap();
        reducer
            .apply(&HarnessEvent::FileChanged {
                tool_call_id: "tool-1".into(),
                path: diff.path.clone(),
                change: diff.change.clone(),
                diff,
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.referenced_paths, vec!["report.html".to_string()]);
    }

    #[test]
    fn permission_file_edit_tracks_referenced_paths() {
        let diff = Diff {
            path: "report.html".into(),
            change: FileChange::Created,
            old_text: None,
            new_text: Some("<html></html>".into()),
        };
        let mut reducer = TurnReducer::new("acp:test");
        reducer
            .apply(&HarnessEvent::PermissionRequested {
                request_id: "perm-1".into(),
                action: "edit".into(),
                detail: PermissionDetail::FileEdit { diff: diff.clone() },
                options: Vec::new(),
            })
            .unwrap();
        let reduced = reducer.finish();
        assert_eq!(reduced.referenced_paths, vec!["report.html".to_string()]);
        assert!(reduced.file_changes.is_empty());
    }
}
