use serde_json::{Value, json};

use crate::harness::{
    HarnessEvent, ToolKind, ToolStatus, TurnEndReason,
};

pub fn stream_line_events(line: &Value) -> Vec<HarnessEvent> {
    match line.get("type").and_then(Value::as_str) {
        Some("system") if line.get("subtype").and_then(Value::as_str) == Some("init") => {
            let session_id = string_field(line, &["session_id", "sessionId"]);
            if session_id.is_empty() {
                return Vec::new();
            }
            let cwd = string_field(line, &["cwd"]);
            vec![HarnessEvent::NativeSessionBound {
                provider: "claude".to_string(),
                session_id,
                cwd: if cwd.is_empty() { None } else { Some(cwd) },
            }]
        }
        Some("assistant") => assistant_events(line),
        Some("user") => user_events(line),
        Some("result") => {
            let mut events = Vec::new();
            if line.get("is_error").and_then(Value::as_bool) == Some(true)
                && let Some(message) = line.get("result").and_then(Value::as_str)
            {
                events.push(HarnessEvent::Error {
                    message: message.to_string(),
                });
            }
            events.push(HarnessEvent::TurnEnded {
                reason: map_result_reason(line),
            });
            events
        }
        Some("stream_event") => stream_event_events(line),
        _ => Vec::new(),
    }
}

fn assistant_events(line: &Value) -> Vec<HarnessEvent> {
    let Some(content) = line.pointer("/message/content") else {
        return Vec::new();
    };
    let mut events = Vec::new();
    if let Value::Array(items) = content {
        for item in items {
            if let Some(event) = assistant_block_event(item) {
                events.push(event);
            }
        }
    }
    events
}

fn assistant_block_event(item: &Value) -> Option<HarnessEvent> {
    match item.get("type").and_then(Value::as_str)? {
        "text" => item.get("text").and_then(Value::as_str).map(|text| {
            HarnessEvent::TextDelta {
                text: text.to_string(),
            }
        }),
        "thinking" => item
            .get("thinking")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| HarnessEvent::ThoughtDelta {
                text: text.to_string(),
            }),
        "tool_use" => {
            let id = string_field(item, &["id"]);
            let name = string_field(item, &["name"]);
            Some(HarnessEvent::ToolCallStarted {
                id: id.clone(),
                name: name.clone(),
                kind: map_tool_kind(&name),
                title: name,
                input: item.get("input").cloned().unwrap_or_else(|| json!({})),
            })
        }
        _ => None,
    }
}

fn user_events(line: &Value) -> Vec<HarnessEvent> {
    let Some(content) = line.pointer("/message/content") else {
        return Vec::new();
    };
    let Value::Array(items) = content else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some("tool_result") {
                return None;
            }
            let id = string_field(item, &["tool_use_id"]);
            let output = item
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            Some(HarnessEvent::ToolCallProgress {
                id,
                status: ToolStatus::Completed,
                content: vec![crate::harness::ToolContent::Text { text: output }],
            })
        })
        .collect()
}

fn stream_event_events(line: &Value) -> Vec<HarnessEvent> {
    let Some(event) = line.get("event") else {
        return Vec::new();
    };
    match event.get("type").and_then(Value::as_str) {
        Some("content_block_delta") => {
            let Some(delta) = event.get("delta") else {
                return Vec::new();
            };
            match delta.get("type").and_then(Value::as_str) {
                Some("text_delta") => delta
                    .get("text")
                    .and_then(Value::as_str)
                    .map(|text| {
                        vec![HarnessEvent::TextDelta {
                            text: text.to_string(),
                        }]
                    })
                    .unwrap_or_default(),
                Some("thinking_delta") => delta
                    .get("thinking")
                    .and_then(Value::as_str)
                    .map(|text| {
                        vec![HarnessEvent::ThoughtDelta {
                            text: text.to_string(),
                        }]
                    })
                    .unwrap_or_default(),
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

fn map_result_reason(line: &Value) -> TurnEndReason {
    if line.get("is_error").and_then(Value::as_bool) == Some(true) {
        return TurnEndReason::Failed;
    }
    match line.get("stop_reason").and_then(Value::as_str) {
        Some("max_tokens") => TurnEndReason::MaxTokens,
        Some("cancelled" | "canceled") => TurnEndReason::Cancelled,
        _ => TurnEndReason::EndTurn,
    }
}

fn map_tool_kind(name: &str) -> ToolKind {
    match name {
        "Bash" | "Execute" => ToolKind::Execute,
        "Read" => ToolKind::Read,
        "Write" | "Edit" | "NotebookEdit" => ToolKind::Edit,
        "WebSearch" => ToolKind::Search,
        "WebFetch" => ToolKind::Fetch,
        other => ToolKind::Other(other.to_string()),
    }
}

fn string_field(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_assistant_text_block() {
        let events = stream_line_events(&json!({
            "type": "assistant",
            "message": {
                "content": [{"type": "text", "text": "hello"}]
            }
        }));
        assert_eq!(
            events,
            vec![HarnessEvent::TextDelta {
                text: "hello".to_string()
            }]
        );
    }
}
