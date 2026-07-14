use serde_json::{Value, json};

use crate::harness::{
    HarnessEvent, PermissionDetail, PermissionOption, ToolContent, ToolKind, ToolStatus,
    TurnEndReason,
};

pub struct OpenCodeEventState {
    pub session_id: String,
}

impl OpenCodeEventState {
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }
}

pub fn normalize_opencode_event(value: &Value, state: &OpenCodeEventState) -> Vec<HarnessEvent> {
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let properties = value.get("properties").unwrap_or(value);
    match event_type {
        "message.part.delta" => message_part_delta(properties, state),
        "message.part.updated" => message_part_updated(properties, state),
        "permission.asked" => permission_asked(properties, state).into_iter().collect(),
        "session.idle" => session_idle(properties, state),
        "session.error" => session_error(properties, state),
        "server.connected" => Vec::new(),
        _ => Vec::new(),
    }
}

fn session_matches(properties: &Value, state: &OpenCodeEventState) -> bool {
    let session_id = string_field(properties, &["sessionID", "sessionId", "session_id"]);
    session_id.is_empty() || session_id == state.session_id
}

fn message_part_delta(properties: &Value, state: &OpenCodeEventState) -> Vec<HarnessEvent> {
    if !session_matches(properties, state) {
        return Vec::new();
    }
    let field = string_field(properties, &["field"]);
    let delta = string_field(properties, &["delta"]);
    if delta.is_empty() {
        return Vec::new();
    }
    if field == "reasoning" {
        return vec![HarnessEvent::ThoughtDelta { text: delta }];
    }
    if field == "text" {
        return vec![HarnessEvent::TextDelta { text: delta }];
    }
    Vec::new()
}

fn message_part_updated(properties: &Value, state: &OpenCodeEventState) -> Vec<HarnessEvent> {
    if !session_matches(properties, state) {
        return Vec::new();
    }
    let part = properties.get("part").unwrap_or(properties);
    let part_type = string_field(part, &["type"]);
    match part_type.as_str() {
        "tool" | "tool-invocation" => tool_part_events(part),
        _ => Vec::new(),
    }
}

fn tool_part_events(part: &Value) -> Vec<HarnessEvent> {
    let id = string_field(part, &["callID", "callId", "id", "toolCallId"]);
    let name = string_field(part, &["tool", "name", "toolName"]);
    let status = string_field(part, &["status"]);
    let input = part
        .get("input")
        .or_else(|| part.get("state"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    if status.is_empty() || status == "running" || status == "pending" {
        return vec![HarnessEvent::ToolCallStarted {
            id: id.clone(),
            name: name.clone(),
            kind: map_tool_kind(&name),
            title: name,
            input,
        }];
    }

    let tool_status = match status.as_str() {
        "completed" | "done" => ToolStatus::Completed,
        "failed" | "error" => ToolStatus::Failed,
        _ => ToolStatus::InProgress,
    };
    let output = part.get("output").or_else(|| part.get("result"));
    vec![HarnessEvent::ToolCallProgress {
        id,
        status: tool_status,
        content: tool_output_content(output),
    }]
}

fn permission_asked(properties: &Value, state: &OpenCodeEventState) -> Option<HarnessEvent> {
    if !session_matches(properties, state) {
        return None;
    }
    let request_id = string_field(properties, &["id", "requestID", "requestId"]);
    if request_id.is_empty() {
        return None;
    }
    let permission = string_field(properties, &["permission"]);
    let action = if permission.is_empty() {
        "OpenCode permission".to_string()
    } else {
        format!("OpenCode permission: {permission}")
    };
    Some(HarnessEvent::PermissionRequested {
        request_id,
        action,
        detail: PermissionDetail::Other {
            value: properties.clone(),
        },
        options: default_permission_options(),
    })
}

fn session_idle(properties: &Value, state: &OpenCodeEventState) -> Vec<HarnessEvent> {
    if session_matches(properties, state) {
        vec![HarnessEvent::TurnEnded {
            reason: TurnEndReason::EndTurn,
        }]
    } else {
        Vec::new()
    }
}

fn session_error(properties: &Value, state: &OpenCodeEventState) -> Vec<HarnessEvent> {
    if !session_matches(properties, state) {
        return Vec::new();
    }
    let message = string_field(properties, &["message", "error"]);
    vec![
        HarnessEvent::Error {
            message: if message.is_empty() {
                "OpenCode session error".to_string()
            } else {
                message
            },
        },
        HarnessEvent::TurnEnded {
            reason: TurnEndReason::Failed,
        },
    ]
}

fn tool_output_content(value: Option<&Value>) -> Vec<ToolContent> {
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
        "read" | "glob" | "grep" | "list" => ToolKind::Read,
        "write" | "edit" | "patch" => ToolKind::Write,
        "bash" | "shell" | "command" => ToolKind::Execute,
        "search" => ToolKind::Search,
        "fetch" => ToolKind::Fetch,
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
    fn maps_text_and_reasoning_deltas() {
        let state = OpenCodeEventState::new("sess-1".into());
        let text = json!({
            "type": "message.part.delta",
            "properties": {
                "sessionID": "sess-1",
                "field": "text",
                "delta": "hello"
            }
        });
        let events = normalize_opencode_event(&text, &state);
        assert!(matches!(events[0], HarnessEvent::TextDelta { .. }));

        let reasoning = json!({
            "type": "message.part.delta",
            "properties": {
                "sessionID": "sess-1",
                "field": "reasoning",
                "delta": "thinking"
            }
        });
        let events = normalize_opencode_event(&reasoning, &state);
        assert!(matches!(events[0], HarnessEvent::ThoughtDelta { .. }));
    }

    #[test]
    fn maps_session_idle_to_turn_end() {
        let state = OpenCodeEventState::new("sess-1".into());
        let idle = json!({
            "type": "session.idle",
            "properties": { "sessionID": "sess-1" }
        });
        let events = normalize_opencode_event(&idle, &state);
        assert!(matches!(
            events[0],
            HarnessEvent::TurnEnded {
                reason: TurnEndReason::EndTurn
            }
        ));
    }

    #[test]
    fn maps_permission_asked() {
        let state = OpenCodeEventState::new("sess-1".into());
        let event = json!({
            "type": "permission.asked",
            "properties": {
                "sessionID": "sess-1",
                "id": "perm-1",
                "permission": "write"
            }
        });
        let events = normalize_opencode_event(&event, &state);
        assert!(matches!(
            &events[0],
            HarnessEvent::PermissionRequested { request_id, .. } if request_id == "perm-1"
        ));
    }
}
