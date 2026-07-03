use serde::{Deserialize, Serialize};

use crate::{CoreError, Result};

pub const METHOD_NOT_FOUND: i64 = -32601;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(
        id: RequestId,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: RequestId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: RequestId, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncomingMessage {
    Response(JsonRpcResponse),
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

impl IncomingMessage {
    pub fn from_line(line: &str) -> Result<IncomingMessage> {
        let value: serde_json::Value = serde_json::from_str(line)?;
        let jsonrpc = value
            .get("jsonrpc")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Protocol("missing jsonrpc field".to_string()))?;
        if jsonrpc != "2.0" {
            return Err(CoreError::Protocol(format!(
                "unsupported jsonrpc version: {jsonrpc}"
            )));
        }

        let has_method = value.get("method").is_some();
        let has_id = value.get("id").is_some();
        let has_result = value.get("result").is_some();
        let has_error = value.get("error").is_some();

        match (has_method, has_id) {
            (true, true) => Ok(IncomingMessage::Request(serde_json::from_value(value)?)),
            (true, false) => Ok(IncomingMessage::Notification(serde_json::from_value(
                value,
            )?)),
            (false, true) => {
                if has_result == has_error {
                    return Err(CoreError::Protocol(
                        "response must carry exactly one of result or error".to_string(),
                    ));
                }
                Ok(IncomingMessage::Response(serde_json::from_value(value)?))
            }
            (false, false) => Err(CoreError::Protocol(
                "message has neither method nor id".to_string(),
            )),
        }
    }
}
