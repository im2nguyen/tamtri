use chrono::Utc;
use serde_json::json;

use crate::Result;
use crate::conversation::{ContentBlock, Id, Message, Role};
use crate::harness::{Diff, FileChange, HarnessEvent, ToolContent, ToolStatus};

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
                id, name, input, ..
            } => {
                self.flush_deltas();
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
                tool_call_id,
                diff,
                ..
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
                options,
                ..
            } => {
                self.flush_deltas();
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
            | HarnessEvent::ModeChanged { .. } => {
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
        if !self.referenced_paths.iter().any(|existing| existing == path) {
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
}
