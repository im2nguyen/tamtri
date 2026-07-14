use serde_json::{Value, json};

use crate::harness::{
    HarnessEvent, PermissionDetail, PermissionOption, ToolKind, ToolStatus, TurnEndReason,
};

const APPROVAL_METHODS: &[&str] = &[
    "item/commandExecution/requestApproval",
    "item/fileChange/requestApproval",
    "item/tool/requestUserInput",
    "tool/requestUserInput",
];

pub fn notification_events(method: &str, params: &Value) -> Vec<HarnessEvent> {
    match method {
        "item/agentMessage/delta" => text_delta(params)
            .map(|text| vec![HarnessEvent::TextDelta { text }])
            .unwrap_or_default(),
        "item/reasoning/summaryTextDelta" => text_delta(params)
            .map(|text| vec![HarnessEvent::ThoughtDelta { text }])
            .unwrap_or_default(),
        "turn/completed" => {
            let mut events = Vec::new();
            if let Some(message) = turn_error_message(params) {
                events.push(HarnessEvent::Error { message });
            }
            events.push(HarnessEvent::TurnEnded {
                reason: map_turn_status(params),
            });
            events
        }
        "item/started" => map_item_started(params).into_iter().collect(),
        "item/completed" => map_item_completed(params).into_iter().collect(),
        "item/commandExecution/terminalInteraction" => map_terminal_output(params)
            .map(|event| vec![event])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub fn is_approval_request(method: &str) -> bool {
    APPROVAL_METHODS.contains(&method)
}

pub fn permission_event(method: &str, params: &Value) -> HarnessEvent {
    let item_id = string_field(params, &["itemId"]);
    let request_id = if item_id.is_empty() {
        "permission-unknown".to_string()
    } else {
        format!("permission-{item_id}")
    };

    let (action, detail) = match method {
        "item/commandExecution/requestApproval" => {
            let command = string_field(params, &["command"]);
            (
                if command.is_empty() {
                    "Run command".to_string()
                } else {
                    format!("Run command: {command}")
                },
                PermissionDetail::Command { command },
            )
        }
        "item/fileChange/requestApproval" => (
            "Apply file changes".to_string(),
            PermissionDetail::Other {
                value: params.clone(),
            },
        ),
        _ => (
            "Tool input required".to_string(),
            PermissionDetail::Other {
                value: params.clone(),
            },
        ),
    };

    HarnessEvent::PermissionRequested {
        request_id,
        action,
        detail,
        options: default_permission_options(),
    }
}

pub fn permission_decision(option_id: &str) -> &'static str {
    match option_id {
        "allow_once" | "allow" => "accept",
        "deny" => "decline",
        _ => "cancel",
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

fn text_delta(params: &Value) -> Option<String> {
    params
        .get("delta")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn turn_error_message(params: &Value) -> Option<String> {
    params
        .pointer("/turn/error/message")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn map_turn_status(params: &Value) -> TurnEndReason {
    match params
        .pointer("/turn/status")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "cancelled" | "canceled" | "interrupted" => TurnEndReason::Cancelled,
        "failed" | "error" => TurnEndReason::Failed,
        "max_tokens" | "max_output_tokens" => TurnEndReason::MaxTokens,
        _ => TurnEndReason::EndTurn,
    }
}

fn map_item_started(params: &Value) -> Option<HarnessEvent> {
    let item = params.get("item")?;
    let id = string_field(item, &["id"]);
    if id.is_empty() {
        return None;
    }
    let item_type = string_field(item, &["type"]);
    if item_type.is_empty() {
        return None;
    }
    if !is_tool_item_type(&item_type) {
        return None;
    }
    Some(HarnessEvent::ToolCallStarted {
        id: id.clone(),
        name: item_type.clone(),
        kind: map_tool_kind(&item_type),
        title: item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(&item_type)
            .to_string(),
        input: item.get("input").cloned().unwrap_or_else(|| json!({})),
    })
}

fn map_item_completed(params: &Value) -> Option<HarnessEvent> {
    let item = params.get("item")?;
    let id = string_field(item, &["id"]);
    if id.is_empty() || !is_tool_item_type(&string_field(item, &["type"])) {
        return None;
    }
    Some(HarnessEvent::ToolCallProgress {
        id,
        status: ToolStatus::Completed,
        content: Vec::new(),
    })
}

fn map_terminal_output(params: &Value) -> Option<HarnessEvent> {
    let item_id = string_field(params, &["itemId"]);
    if item_id.is_empty() {
        return None;
    }
    let chunk = params
        .get("chunk")
        .or_else(|| params.get("output"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Some(HarnessEvent::TerminalOutput {
        tool_call_id: item_id,
        chunk,
    })
}

fn is_tool_item_type(item_type: &str) -> bool {
    matches!(
        item_type,
        "commandExecution" | "fileChange" | "mcpToolCall" | "webSearch" | "collabAgentToolCall"
    )
}

fn map_tool_kind(item_type: &str) -> ToolKind {
    match item_type {
        "commandExecution" => ToolKind::Execute,
        "fileChange" => ToolKind::Edit,
        "webSearch" => ToolKind::Search,
        "mcpToolCall" => ToolKind::Fetch,
        other => ToolKind::Other(other.to_string()),
    }
}

fn string_field(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_agent_message_delta() {
        let events = notification_events(
            "item/agentMessage/delta",
            &json!({"itemId": "m1", "delta": "hello"}),
        );
        assert_eq!(
            events,
            vec![HarnessEvent::TextDelta {
                text: "hello".to_string()
            }]
        );
    }

    #[test]
    fn maps_turn_completed_failed() {
        let events = notification_events(
            "turn/completed",
            &json!({"turn": {"status": "failed", "error": {"message": "boom"}}}),
        );
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], HarnessEvent::Error { .. }));
        assert!(matches!(
            events[1],
            HarnessEvent::TurnEnded {
                reason: TurnEndReason::Failed
            }
        ));
    }

    #[test]
    fn maps_command_approval_request() {
        let event = permission_event(
            "item/commandExecution/requestApproval",
            &json!({"itemId": "x1", "command": "ls"}),
        );
        assert!(matches!(
            event,
            HarnessEvent::PermissionRequested {
                request_id,
                ..
            } if request_id == "permission-x1"
        ));
    }
}
