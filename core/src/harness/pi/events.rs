use serde_json::{Value, json};

use crate::harness::{
    HarnessEvent, PermissionDetail, PermissionOption, ToolContent, ToolKind, ToolStatus,
    TurnEndReason,
};

pub fn normalize_pi_event(value: &Value) -> Vec<HarnessEvent> {
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match event_type {
        "message_update" => message_update_events(value),
        "tool_execution_start" => vec![tool_execution_started(value)],
        "tool_execution_update" => vec![tool_execution_update(value)],
        "tool_execution_end" => vec![tool_execution_end(value)],
        "agent_end" => vec![HarnessEvent::TurnEnded {
            reason: TurnEndReason::EndTurn,
        }],
        "extension_ui_request" => extension_ui_request(value).into_iter().collect(),
        "process_exit" => {
            let message = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Pi RPC process exited")
                .to_string();
            vec![
                HarnessEvent::Error { message },
                HarnessEvent::TurnEnded {
                    reason: TurnEndReason::Failed,
                },
            ]
        }
        _ => Vec::new(),
    }
}

fn message_update_events(value: &Value) -> Vec<HarnessEvent> {
    let message = value.get("message").unwrap_or(value);
    if message.get("role").and_then(Value::as_str) != Some("assistant") {
        return Vec::new();
    }
    let assistant_event = value
        .get("assistantMessageEvent")
        .or_else(|| value.get("assistant_message_event"));
    let Some(assistant_event) = assistant_event else {
        return Vec::new();
    };
    let kind = assistant_event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let delta = assistant_event
        .get("delta")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if delta.is_empty() {
        return Vec::new();
    }
    match kind {
        "text_delta" => vec![HarnessEvent::TextDelta {
            text: delta.to_string(),
        }],
        "thinking_delta" => vec![HarnessEvent::ThoughtDelta {
            text: delta.to_string(),
        }],
        _ => Vec::new(),
    }
}

fn tool_execution_started(value: &Value) -> HarnessEvent {
    let id = string_field(value, &["toolCallId", "tool_call_id"]);
    let name = string_field(value, &["toolName", "tool_name"]);
    let args = value
        .get("args")
        .or_else(|| value.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    HarnessEvent::ToolCallStarted {
        id,
        name: name.clone(),
        kind: map_tool_kind(&name),
        title: name,
        input: args,
    }
}

fn tool_execution_update(value: &Value) -> HarnessEvent {
    let id = string_field(value, &["toolCallId", "tool_call_id"]);
    let partial = value
        .get("partialResult")
        .or_else(|| value.get("partial_result"));
    HarnessEvent::ToolCallProgress {
        id,
        status: ToolStatus::InProgress,
        content: tool_result_content(partial),
    }
}

fn tool_execution_end(value: &Value) -> HarnessEvent {
    let id = string_field(value, &["toolCallId", "tool_call_id"]);
    let is_error = value
        .get("isError")
        .or_else(|| value.get("is_error"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let result = value.get("result");
    HarnessEvent::ToolCallProgress {
        id,
        status: if is_error {
            ToolStatus::Failed
        } else {
            ToolStatus::Completed
        },
        content: tool_result_content(result),
    }
}

fn extension_ui_request(value: &Value) -> Option<HarnessEvent> {
    let request_id = string_field(value, &["id"]);
    if request_id.is_empty() {
        return None;
    }
    let method = string_field(value, &["method"]);
    let message = string_field(value, &["message", "title"]);
    let action = if message.is_empty() {
        format!("Pi extension UI: {method}")
    } else {
        message
    };
    Some(HarnessEvent::PermissionRequested {
        request_id,
        action,
        detail: PermissionDetail::Other {
            value: value.clone(),
        },
        options: default_permission_options(),
    })
}

fn tool_result_content(value: Option<&Value>) -> Vec<ToolContent> {
    let Some(value) = value else {
        return Vec::new();
    };
    if let Some(text) = value.as_str() {
        if text.is_empty() {
            return Vec::new();
        }
        return vec![ToolContent::Text {
            text: text.to_string(),
        }];
    }
    vec![ToolContent::Json {
        value: value.clone(),
    }]
}

fn map_tool_kind(name: &str) -> ToolKind {
    match name.to_ascii_lowercase().as_str() {
        "read" | "read_file" | "glob" | "grep" => ToolKind::Read,
        "write" | "write_file" | "edit" | "apply_patch" => ToolKind::Write,
        "bash" | "execute" | "run_terminal_cmd" | "shell" => ToolKind::Execute,
        "search" | "web_search" => ToolKind::Search,
        "fetch" | "web_fetch" => ToolKind::Fetch,
        "think" => ToolKind::Think,
        other => ToolKind::Other(other.to_string()),
    }
}

fn default_permission_options() -> Vec<PermissionOption> {
    vec![
        PermissionOption {
            id: "allow_once".to_string(),
            label: "Allow once".to_string(),
        },
        PermissionOption {
            id: "deny".to_string(),
            label: "Deny".to_string(),
        },
    ]
}

fn string_field(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_string)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_text_and_thinking_deltas() {
        let text = json!({
            "type": "message_update",
            "message": { "role": "assistant" },
            "assistantMessageEvent": { "type": "text_delta", "delta": "hello" }
        });
        let events = normalize_pi_event(&text);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], HarnessEvent::TextDelta { .. }));

        let thinking = json!({
            "type": "message_update",
            "message": { "role": "assistant" },
            "assistantMessageEvent": { "type": "thinking_delta", "delta": "hmm" }
        });
        let events = normalize_pi_event(&thinking);
        assert!(matches!(events[0], HarnessEvent::ThoughtDelta { .. }));
    }

    #[test]
    fn maps_tool_execution_lifecycle() {
        let start = json!({
            "type": "tool_execution_start",
            "toolCallId": "t1",
            "toolName": "read",
            "args": { "path": "report.csv" }
        });
        let events = normalize_pi_event(&start);
        assert!(matches!(
            &events[0],
            HarnessEvent::ToolCallStarted { id, name, .. }
                if id == "t1" && name == "read"
        ));

        let end = json!({
            "type": "tool_execution_end",
            "toolCallId": "t1",
            "toolName": "read",
            "result": "ok",
            "isError": false
        });
        let events = normalize_pi_event(&end);
        assert!(matches!(
            events[0],
            HarnessEvent::ToolCallProgress {
                status: ToolStatus::Completed,
                ..
            }
        ));
    }

    #[test]
    fn maps_extension_ui_request_to_permission() {
        let event = json!({
            "type": "extension_ui_request",
            "id": "perm-1",
            "method": "confirm",
            "message": "Allow edit?"
        });
        let events = normalize_pi_event(&event);
        assert!(matches!(
            &events[0],
            HarnessEvent::PermissionRequested { request_id, .. } if request_id == "perm-1"
        ));
    }
}
