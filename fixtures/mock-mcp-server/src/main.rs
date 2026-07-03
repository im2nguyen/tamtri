use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut stdout = io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        if matches!(input.read_line(&mut line), Ok(0) | Err(_)) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(line.trim_end()) else {
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
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echoes its arguments",
                            "inputSchema": {"type": "object"}
                        },
                        {
                            "name": "elicit",
                            "description": "Elicits a name then echoes it",
                            "inputSchema": {"type": "object"}
                        }
                    ]
                }),
            ),
            Some("tools/call") => {
                let tool_name = message
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if tool_name == "elicit" {
                    match elicit_then_echo(&mut stdout, &mut input) {
                        Ok(result) => response(&message, result),
                        Err(err) => error_response(&message, -32000, err),
                    }
                } else {
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

fn elicit_then_echo(stdout: &mut io::Stdout, input: &mut impl BufRead) -> Result<Value, String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": "elicit-1",
        "method": "elicitation/create",
        "params": {
            "mode": "form",
            "message": "What name should the echo use?",
            "requestedSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "title": "Name"
                    }
                },
                "required": ["name"]
            }
        }
    });
    serde_json::to_writer(&mut *stdout, &request).map_err(|err| err.to_string())?;
    writeln!(stdout).map_err(|err| err.to_string())?;
    stdout.flush().map_err(|err| err.to_string())?;

    let mut line = String::new();
    input
        .read_line(&mut line)
        .map_err(|err| err.to_string())?;
    if line.trim().is_empty() {
        return Err("missing elicitation response".to_string());
    }
    let response: Value = serde_json::from_str(line.trim_end()).map_err(|err| err.to_string())?;
    let action = response
        .pointer("/result/action")
        .and_then(Value::as_str)
        .unwrap_or("cancel");
    if action != "accept" {
        return Ok(json!({
            "content": [{"type": "text", "text": format!("elicitation {action}")}],
            "isError": false,
            "structuredContent": {"elicitation": action}
        }));
    }
    let name = response
        .pointer("/result/content/name")
        .and_then(Value::as_str)
        .unwrap_or("anonymous");
    Ok(json!({
        "content": [{"type": "text", "text": format!("hello {name}")}],
        "isError": false,
        "structuredContent": {"name": name}
    }))
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
