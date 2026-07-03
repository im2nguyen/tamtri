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
            Some("tools/list") => {
                if should_hang_on_list() {
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(3600));
                    }
                }
                response(
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
                            },
                            {
                                "name": "elicit_url",
                                "description": "Elicits via URL handoff then echoes the action",
                                "inputSchema": {"type": "object"}
                            }
                        ]
                    }),
                )
            }
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
                } else if tool_name == "elicit_url" {
                    match elicit_url_then_echo(&mut stdout, &mut input) {
                        Ok(result) => response(&message, result),
                        Err(err) => error_response(&message, -32000, err),
                    }
                } else {
                    let arguments = message
                        .pointer("/params/arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    if should_emit_progress() {
                        emit_progress_and_log(&mut stdout);
                    }
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
            Some("resources/list") => {
                let cursor = message
                    .pointer("/params/cursor")
                    .and_then(Value::as_str);
                if cursor == Some("page2") {
                    response(
                        &message,
                        json!({
                            "resources": [{
                                "uri": "mock://appendix",
                                "name": "Appendix",
                                "description": "Second page resource",
                                "mimeType": "text/plain"
                            }]
                        }),
                    )
                } else {
                    response(
                        &message,
                        json!({
                            "resources": [{
                                "uri": "mock://report",
                                "name": "Report",
                                "description": "A mock resource",
                                "mimeType": "text/plain"
                            }],
                            "nextCursor": "page2"
                        }),
                    )
                }
            }
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
            Some("prompts/list") => {
                let cursor = message
                    .pointer("/params/cursor")
                    .and_then(Value::as_str);
                if cursor == Some("page2") {
                    response(
                        &message,
                        json!({
                            "prompts": [{
                                "name": "outline",
                                "description": "Outline something",
                                "arguments": [{"name": "topic", "required": true}]
                            }]
                        }),
                    )
                } else {
                    response(
                        &message,
                        json!({
                            "prompts": [{
                                "name": "summarize",
                                "description": "Summarize something",
                                "arguments": [{"name": "topic", "required": true}]
                            }],
                            "nextCursor": "page2"
                        }),
                    )
                }
            }
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

fn elicit_url_then_echo(stdout: &mut io::Stdout, input: &mut impl BufRead) -> Result<Value, String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": "elicit-url-1",
        "method": "elicitation/create",
        "params": {
            "mode": "url",
            "message": "Sign in to continue",
            "url": "https://example.com/oauth/authorize?client_id=demo&state=abc"
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
    Ok(json!({
        "content": [{"type": "text", "text": format!("url elicitation {action}")}],
        "isError": false,
        "structuredContent": {"elicitation": action}
    }))
}

fn should_emit_progress() -> bool {
    std::env::var("MOCK_MCP_EMIT_PROGRESS")
        .ok()
        .is_some_and(|value| value == "1")
}

fn emit_progress_and_log(stdout: &mut io::Stdout) {
    for notification in [
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {"progress": 0.5, "message": "halfway"}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/message",
            "params": {"level": "info", "data": "working"}
        }),
    ] {
        let _ = serde_json::to_writer(&mut *stdout, &notification);
        let _ = writeln!(stdout);
        let _ = stdout.flush();
    }
}

fn should_hang_on_list() -> bool {
    let Some(marker) = std::env::var("MOCK_MCP_LIST_MARKER").ok() else {
        return false;
    };
    if std::path::Path::new(&marker).exists() {
        return false;
    }
    std::fs::write(&marker, "hung").is_ok()
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
