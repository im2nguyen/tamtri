//! tamtri-native MCP tools exposed on the gateway (`tamtri__*`).

use serde_json::{Value, json};

use crate::mcp::gateway::gateway_exposed_tool_name;
use crate::mcp::protocol::Tool;

pub const TAMTRI_SERVER_ID: &str = "tamtri";

pub const TOOL_ORCHESTRATION_RUN: &str = "orchestration_run";
pub const TOOL_ORCHESTRATION_STATUS: &str = "orchestration_status";
pub const TOOL_ORCHESTRATION_CANCEL: &str = "orchestration_cancel";
pub const TOOL_ORCHESTRATION_HANDOFF: &str = "orchestration_handoff";

pub fn exposed_orchestration_tools() -> Vec<(String, Tool)> {
    vec![
        (
            gateway_exposed_tool_name(TAMTRI_SERVER_ID, TOOL_ORCHESTRATION_RUN),
            Tool {
                name: TOOL_ORCHESTRATION_RUN.to_string(),
                description: Some(
                    "Start a background orchestration recipe from the current conversation.".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "recipe_id": { "type": "string" },
                        "inputs_json": { "type": "string" }
                    },
                    "required": ["recipe_id"]
                }),
                meta: None,
            },
        ),
        (
            gateway_exposed_tool_name(TAMTRI_SERVER_ID, TOOL_ORCHESTRATION_STATUS),
            Tool {
                name: TOOL_ORCHESTRATION_STATUS.to_string(),
                description: Some("Read orchestration run status by run id.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": { "type": "string" }
                    },
                    "required": ["run_id"]
                }),
                meta: None,
            },
        ),
        (
            gateway_exposed_tool_name(TAMTRI_SERVER_ID, TOOL_ORCHESTRATION_CANCEL),
            Tool {
                name: TOOL_ORCHESTRATION_CANCEL.to_string(),
                description: Some("Cancel a running orchestration run.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": { "type": "string" }
                    },
                    "required": ["run_id"]
                }),
                meta: None,
            },
        ),
        (
            gateway_exposed_tool_name(TAMTRI_SERVER_ID, TOOL_ORCHESTRATION_HANDOFF),
            Tool {
                name: TOOL_ORCHESTRATION_HANDOFF.to_string(),
                description: Some(
                    "Fork the current conversation to another harness with a briefing message (handoff recipe)."
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "harness_id": { "type": "string" },
                        "model_id": { "type": "string" },
                        "message": { "type": "string" }
                    },
                    "required": ["harness_id", "model_id", "message"]
                }),
                meta: None,
            },
        ),
    ]
}

pub fn is_native_tool(exposed_name: &str) -> bool {
    exposed_name.starts_with(&format!("{TAMTRI_SERVER_ID}__"))
}

pub fn native_original_name(exposed_name: &str) -> Option<&str> {
    exposed_name.strip_prefix(&format!("{TAMTRI_SERVER_ID}__"))
}

pub fn tool_result_text(text: impl Into<String>) -> crate::mcp::protocol::CallToolResult {
    crate::mcp::protocol::CallToolResult {
        content: vec![json!({ "type": "text", "text": text.into() })],
        is_error: None,
        structured_content: None,
    }
}

pub fn tool_result_structured(value: Value) -> crate::mcp::protocol::CallToolResult {
    crate::mcp::protocol::CallToolResult {
        content: vec![json!({ "type": "text", "text": value.to_string() })],
        is_error: None,
        structured_content: Some(value),
    }
}

pub fn tool_result_error(message: impl Into<String>) -> crate::mcp::protocol::CallToolResult {
    crate::mcp::protocol::CallToolResult {
        content: vec![json!({ "type": "text", "text": message.into() })],
        is_error: Some(true),
        structured_content: None,
    }
}
