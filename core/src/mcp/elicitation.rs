use serde_json::{Map, Value, json};

use crate::conversation::{ContentBlock, ElicitationAction, ElicitationMode};
use crate::mcp::protocol::ElicitationCreateParams;
use crate::mcp::url_handoff::{redact_url_for_audit, validate_handoff_url};
use crate::{CoreError, Result};

pub fn parse_create_params(params: Value) -> Result<ElicitationCreateParams> {
    serde_json::from_value(params)
        .map_err(|err| CoreError::Protocol(format!("invalid elicitation/create params: {err}")))
}

pub fn elicitation_mode(params: &ElicitationCreateParams) -> ElicitationMode {
    match params.mode.as_deref().unwrap_or("form") {
        "url" => ElicitationMode::Url,
        _ => ElicitationMode::Form,
    }
}

pub fn elicitation_request_id(params: &ElicitationCreateParams, rpc_id: &str) -> String {
    params
        .elicitation_id
        .clone()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| rpc_id.to_string())
}

pub fn schema_looks_secret(schema: &Value) -> bool {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return false;
    };
    properties
        .iter()
        .any(|(name, property)| text_looks_secret(name) || field_descriptor_looks_secret(property))
}

fn field_descriptor_looks_secret(property: &Value) -> bool {
    for key in ["title", "description", "format"] {
        if let Some(text) = property.get(key).and_then(Value::as_str)
            && text_looks_secret(text)
        {
            return true;
        }
    }
    false
}

pub fn text_looks_secret(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    [
        "password",
        "secret",
        "api_key",
        "api key",
        "apikey",
        "access_token",
        "access token",
        "refresh_token",
        "refresh token",
        "private_key",
        "private key",
        "bearer",
        "credential",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

pub fn sanitize_transcript_data(data: &Value) -> Value {
    match data {
        Value::Object(map) => {
            let mut clean = Map::new();
            for (key, value) in map {
                if key.to_ascii_lowercase().contains("secret")
                    || key.to_ascii_lowercase().contains("password")
                    || key.to_ascii_lowercase().contains("token")
                    || key.to_ascii_lowercase().contains("api_key")
                {
                    clean.insert(key.clone(), Value::String("[redacted]".to_string()));
                } else {
                    clean.insert(key.clone(), sanitize_transcript_data(value));
                }
            }
            Value::Object(clean)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize_transcript_data).collect()),
        _ => data.clone(),
    }
}

pub fn result_for_action(action: ElicitationAction, data: Option<Value>) -> Value {
    let action = match action {
        ElicitationAction::Accept => "accept",
        ElicitationAction::Decline => "decline",
        ElicitationAction::Cancel => "cancel",
    };
    match data {
        Some(content) if action == "accept" => json!({ "action": action, "content": content }),
        _ => json!({ "action": action }),
    }
}

pub fn origin_tool_call_id_from_meta(meta: Option<&Value>) -> Option<String> {
    let meta = meta?;
    for key in [
        "toolCallId",
        "tool_call_id",
        "originToolCallId",
        "origin_tool_call_id",
    ] {
        if let Some(value) = meta.get(key).and_then(Value::as_str) {
            return Some(value.to_string());
        }
    }
    None
}

pub fn schema_is_renderable(schema: &Value) -> bool {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return false;
    };
    for property in properties.values() {
        if !property_is_renderable(property) {
            return false;
        }
    }
    true
}

fn property_is_renderable(property: &Value) -> bool {
    let ty = property
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("string");
    match ty {
        "string" | "number" | "integer" | "boolean" => true,
        "array" => property
            .get("items")
            .is_none_or(|items| items.get("type").and_then(Value::as_str) != Some("object")),
        "object" => false,
        _ => false,
    }
}

pub fn validate_elicitation_url(raw: &str) -> Result<String> {
    let validated = validate_handoff_url(raw)?;
    Ok(validated.url)
}

pub fn audit_safe_elicitation_url(raw: &str) -> String {
    redact_url_for_audit(raw)
}

pub fn elicitation_request_block(
    request_id: String,
    server_id: String,
    origin_tool_call_id: Option<String>,
    mode: ElicitationMode,
    message: String,
    schema: Option<Value>,
    url: Option<String>,
) -> ContentBlock {
    ContentBlock::ElicitationRequest {
        request_id,
        server_id: Some(server_id),
        origin_tool_call_id,
        mode,
        message,
        schema,
        url: url.as_ref().map(|value| audit_safe_elicitation_url(value)),
    }
}

pub fn elicitation_response_block(
    request_id: String,
    action: ElicitationAction,
    data: Option<Value>,
) -> ContentBlock {
    ContentBlock::ElicitationResponse {
        request_id,
        action,
        data: data.map(|value| sanitize_transcript_data(&value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_schema_is_rejected() {
        let schema = json!({
            "type": "object",
            "properties": {
                "api_key": { "type": "string", "title": "API key" }
            }
        });
        assert!(schema_looks_secret(&schema));
    }

    #[test]
    fn benign_schema_is_allowed() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "title": "Display name" }
            }
        });
        assert!(!schema_looks_secret(&schema));
    }

    #[test]
    fn transcript_data_redacts_secret_keys() {
        let data = json!({ "name": "octocat", "api_key": "abc123" });
        let sanitized = sanitize_transcript_data(&data);
        assert_eq!(sanitized["name"], "octocat");
        assert_eq!(sanitized["api_key"], "[redacted]");
    }

    #[test]
    fn elicitation_url_audit_redacts_query() {
        let redacted = audit_safe_elicitation_url("https://example.com/auth?secret=1");
        assert_eq!(redacted, "https://example.com/auth");
    }
}
