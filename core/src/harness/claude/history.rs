//! Parse Claude Code session jsonl files from `~/.claude/projects`.

use std::fs;
use std::path::Path;

use chrono::Utc;
use serde_json::{Value, json};

use crate::conversation::{ContentBlock, Id, Message, Role};
use crate::{CoreError, Result};

pub struct ParsedClaudeSession {
    pub session_id: String,
    pub cwd: String,
    pub title: Option<String>,
    pub messages: Vec<Message>,
}

pub fn parse_claude_session_file(path: &Path) -> Result<ParsedClaudeSession> {
    let content = fs::read_to_string(path)
        .map_err(|err| CoreError::Protocol(format!("failed to read Claude session file: {err}")))?;
    parse_claude_session_jsonl(&content)
}

pub fn parse_claude_session_jsonl(content: &str) -> Result<ParsedClaudeSession> {
    let mut session_id = None;
    let mut cwd = None;
    let mut title = None;
    let mut messages = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if entry.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        if session_id.is_none() {
            session_id = string_field(&entry, &["sessionId", "session_id"]);
        }
        if cwd.is_none() {
            cwd = string_field(&entry, &["cwd"]);
        }
        if let Some(message) = history_entry_to_message(&entry) {
            if title.is_none()
                && message.role == Role::User
                && let Some(ContentBlock::Text { text }) = message.content.first()
            {
                title = Some(truncate_title(text));
            }
            messages.push(message);
        }
    }

    let session_id = session_id
        .ok_or_else(|| CoreError::Protocol("Claude session file missing sessionId".to_string()))?;
    let cwd =
        cwd.ok_or_else(|| CoreError::Protocol("Claude session file missing cwd".to_string()))?;

    Ok(ParsedClaudeSession {
        session_id,
        cwd,
        title,
        messages,
    })
}

fn history_entry_to_message(entry: &Value) -> Option<Message> {
    let entry_type = entry.get("type").and_then(Value::as_str)?;
    match entry_type {
        "user" | "assistant" => {
            let message = entry.get("message")?;
            let role = match message.get("role").and_then(Value::as_str)? {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => return None,
            };
            let content = message_content_blocks(message.get("content")?, role == Role::Assistant);
            if content.is_empty() {
                return None;
            }
            Some(Message {
                id: Id::now_v7(),
                role,
                harness_id: None,
                content,
                created_at: Utc::now(),
            })
        }
        _ => None,
    }
}

fn message_content_blocks(content: &Value, assistant: bool) -> Vec<ContentBlock> {
    match content {
        Value::String(text) => {
            if text.trim().is_empty() {
                Vec::new()
            } else {
                vec![ContentBlock::Text { text: text.clone() }]
            }
        }
        Value::Array(items) => items
            .iter()
            .filter_map(|item| content_block_from_json(item, assistant))
            .collect(),
        _ => Vec::new(),
    }
}

fn content_block_from_json(item: &Value, assistant: bool) -> Option<ContentBlock> {
    let kind = item.get("type").and_then(Value::as_str)?;
    match kind {
        "text" if assistant => {
            item.get("text")
                .and_then(Value::as_str)
                .map(|text| ContentBlock::Text {
                    text: text.to_string(),
                })
        }
        "text" => item.get("text").and_then(Value::as_str).and_then(|text| {
            if text.trim().is_empty() {
                None
            } else {
                Some(ContentBlock::Text {
                    text: text.to_string(),
                })
            }
        }),
        "thinking" => item
            .get("thinking")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| ContentBlock::Thinking {
                text: text.to_string(),
            }),
        "tool_use" => {
            let id = string_field(item, &["id"]).unwrap_or_default();
            let name = string_field(item, &["name"]).unwrap_or_else(|| "tool".to_string());
            let input = item.get("input").cloned().unwrap_or_else(|| json!({}));
            Some(ContentBlock::ToolCall { id, name, input })
        }
        "tool_result" => {
            let call_id = string_field(item, &["tool_use_id"]).unwrap_or_default();
            let output = tool_result_output(item);
            Some(ContentBlock::ToolResult { call_id, output })
        }
        _ => None,
    }
}

fn tool_result_output(item: &Value) -> Value {
    if let Some(content) = item.get("content") {
        if content.is_string() {
            return json!({ "text": content });
        }
        return content.clone();
    }
    item.clone()
}

fn truncate_title(text: &str) -> String {
    let normalized = text.trim().replace('\n', " ");
    if normalized.chars().count() > 80 {
        normalized.chars().take(80).collect()
    } else {
        normalized
    }
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_and_assistant_lines() {
        let parsed = parse_claude_session_jsonl(
            r#"{"type":"user","sessionId":"s1","cwd":"/tmp","message":{"role":"user","content":"hello"}}
{"type":"assistant","sessionId":"s1","cwd":"/tmp","message":{"role":"assistant","content":[{"type":"text","text":"world"}]}}"#,
        )
        .unwrap();
        assert_eq!(parsed.session_id, "s1");
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.title.as_deref(), Some("hello"));
    }
}
