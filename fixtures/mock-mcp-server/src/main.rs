use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message.get("id").is_none() {
            continue;
        }

        let response = match message.get("method").and_then(Value::as_str) {
            Some("initialize") => response(
                &message,
                json!({
                    "protocolVersion": "2025-11-25",
                    "capabilities": {
                        "tools": {"listChanged": false},
                        "resources": {"listChanged": false},
                        "prompts": {"listChanged": false}
                    },
                    "serverInfo": {"name": "mock-mcp-server", "version": "0.1.0"}
                }),
            ),
            Some("tools/list") => response(
                &message,
                json!({
                    "tools": [{
                        "name": "echo",
                        "description": "Echoes its arguments",
                        "inputSchema": {"type": "object"}
                    }]
                }),
            ),
            Some("tools/call") => {
                let arguments = message
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                response(
                    &message,
                    json!({
                        "content": [{"type": "text", "text": arguments.to_string()}],
                        "isError": false,
                        "structuredContent": {"echo": arguments}
                    }),
                )
            }
            Some("resources/list") => response(
                &message,
                json!({
                    "resources": [{
                        "uri": "mock://report",
                        "name": "Report",
                        "description": "A mock resource",
                        "mimeType": "text/plain"
                    }]
                }),
            ),
            Some("resources/read") => response(
                &message,
                json!({
                    "contents": [{
                        "uri": message.pointer("/params/uri").cloned().unwrap_or_else(|| json!("mock://report")),
                        "mimeType": "text/plain",
                        "text": "mock resource"
                    }]
                }),
            ),
            Some("prompts/list") => response(
                &message,
                json!({
                    "prompts": [{
                        "name": "summarize",
                        "description": "Summarize something",
                        "arguments": [{"name": "topic", "required": true}]
                    }]
                }),
            ),
            Some("prompts/get") => response(
                &message,
                json!({
                    "description": "Summarize something",
                    "messages": [{
                        "role": "user",
                        "content": {"type": "text", "text": "Summarize the topic."}
                    }]
                }),
            ),
            Some(method) => error_response(&message, -32601, format!("unknown method: {method}")),
            None => error_response(&message, -32600, "missing method"),
        };

        if serde_json::to_writer(&mut stdout, &response).is_err() {
            break;
        }
        if writeln!(stdout).is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
    }
}

fn response(request: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request.get("id").cloned().unwrap_or(Value::Null),
        "result": result
    })
}

fn error_response(request: &Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request.get("id").cloned().unwrap_or(Value::Null),
        "error": {"code": code, "message": message.into()}
    })
}
