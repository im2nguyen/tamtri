//! Parse Codex rollout session jsonl files from `~/.codex/sessions`.

use std::fs;
use std::path::Path;

use chrono::Utc;
use serde_json::Value;

use crate::conversation::{ContentBlock, Id, Message, Role};
use crate::{CoreError, Result};

pub struct ParsedCodexSession {
    pub session_id: String,
    pub cwd: String,
    pub title: Option<String>,
    pub messages: Vec<Message>,
}

pub fn parse_codex_session_file(path: &Path) -> Result<ParsedCodexSession> {
    let content = fs::read_to_string(path).map_err(|err| {
        CoreError::Protocol(format!("failed to read Codex session file: {err}"))
    })?;
    parse_codex_session_jsonl(&content)
}

pub fn parse_codex_session_jsonl(content: &str) -> Result<ParsedCodexSession> {
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
        let entry_type = entry.get("type").and_then(Value::as_str).unwrap_or_default();
        match entry_type {
            "session_meta" => {
                if let Some(payload) = entry.get("payload") {
                    if session_id.is_none() {
                        session_id = string_field(payload, &["session_id", "id"]);
                    }
                    if cwd.is_none() {
                        cwd = string_field(payload, &["cwd"]);
                    }
                }
            }
            "response_item" => {
                if let Some(message) = response_item_to_message(entry.get("payload")) {
                    if title.is_none()
                        && message.role == Role::User
                        && let Some(ContentBlock::Text { text }) = message.content.first()
                    {
                        title = Some(truncate_title(text));
                    }
                    messages.push(message);
                }
            }
            _ => {}
        }
    }

    let session_id = session_id.ok_or_else(|| {
        CoreError::Protocol("Codex session file missing session_id".to_string())
    })?;
    let cwd = cwd.ok_or_else(|| CoreError::Protocol("Codex session file missing cwd".to_string()))?;

    Ok(ParsedCodexSession {
        session_id,
        cwd,
        title,
        messages,
    })
}

fn response_item_to_message(payload: Option<&Value>) -> Option<Message> {
    let payload = payload?;
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    let role = match payload.get("role").and_then(Value::as_str)? {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "developer" | "system" => return None,
        _ => return None,
    };
    let content = message_content(payload.get("content")?, &role);
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

fn message_content(content: &Value, role: &Role) -> Vec<ContentBlock> {
    let Value::Array(items) = content else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| content_block_from_item(item, role))
        .collect()
}

fn content_block_from_item(item: &Value, role: &Role) -> Option<ContentBlock> {
    let kind = item.get("type").and_then(Value::as_str)?;
    match (role, kind) {
        (Role::User, "input_text") => {
            let text = item.get("text").and_then(Value::as_str)?;
            if is_codex_system_user_text(text) {
                return None;
            }
            Some(ContentBlock::Text {
                text: text.to_string(),
            })
        }
        (Role::Assistant, "output_text") => item.get("text").and_then(Value::as_str).map(|text| {
            ContentBlock::Text {
                text: text.to_string(),
            }
        }),
        _ => None,
    }
}

fn is_codex_system_user_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("<environment_context")
        || trimmed.starts_with("<permissions instructions>")
        || trimmed.starts_with("<developer")
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
    fn parses_session_meta_and_user_assistant_messages() {
        let parsed = parse_codex_session_jsonl(
            r#"{"type":"session_meta","payload":{"session_id":"thread-1","cwd":"/tmp/work"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Compare repos"}]}}
{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Here is the comparison."}]}}"#,
        )
        .unwrap();
        assert_eq!(parsed.session_id, "thread-1");
        assert_eq!(parsed.cwd, "/tmp/work");
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.title.as_deref(), Some("Compare repos"));
    }

    #[test]
    fn skips_environment_context_user_noise() {
        let parsed = parse_codex_session_jsonl(
            r#"{"type":"session_meta","payload":{"session_id":"thread-1","cwd":"/tmp"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>\n  <cwd>/tmp</cwd>\n</environment_context>"}]}}
{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"real prompt"}]}}"#,
        )
        .unwrap();
        assert_eq!(parsed.messages.len(), 1);
        assert_eq!(
            parsed.messages[0].content,
            vec![ContentBlock::Text {
                text: "real prompt".to_string()
            }]
        );
    }
}
