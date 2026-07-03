use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

/// MCP fixture that advertises 2026-07-28 RC Apps/Tasks extensions and may request sampling.
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
                    "protocolVersion": "2026-07-28",
                    "capabilities": {
                        "tools": {"listChanged": false},
                        "resources": {"listChanged": false},
                        "prompts": {"listChanged": false},
                        "sampling": {},
                        "extensions": {
                            "io.modelcontextprotocol/apps": {"version": "1"},
                            "io.modelcontextprotocol/tasks": {"version": "1"},
                            "io.example/unknown": {"version": "0"}
                        }
                    },
                    "serverInfo": {"name": "m7-rc-mcp", "version": "0.1.0"}
                }),
            ),
            Some("tools/list") => response(
                &message,
                json!({
                    "tools": [{
                        "name": "probe_sampling",
                        "description": "Requests sampling from the client",
                        "inputSchema": {"type": "object"}
                    }]
                }),
            ),
            Some("tools/call") => {
                let tool_name = message
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if tool_name == "probe_sampling" {
                    match request_sampling(&mut stdout, &mut input) {
                        Ok(result) => response(&message, result),
                        Err(err) => error_response(&message, -32000, err),
                    }
                } else {
                    response(
                        &message,
                        json!({
                            "content": [{"type": "text", "text": "ok"}],
                            "isError": false
                        }),
                    )
                }
            }
            Some("notifications/initialized") => continue,
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

fn request_sampling(stdout: &mut io::Stdout, input: &mut impl BufRead) -> Result<Value, String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": "sampling-1",
        "method": "sampling/create",
        "params": {
            "messages": [{
                "role": "user",
                "content": {"type": "text", "text": "Say hello"}
            }],
            "maxTokens": 16
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
        return Err("missing sampling response".to_string());
    }
    let response: Value = serde_json::from_str(line.trim_end()).map_err(|err| err.to_string())?;
    let declined = response.get("error").is_some();
    Ok(json!({
        "content": [{
            "type": "text",
            "text": if declined { "sampling declined" } else { "sampling accepted" }
        }],
        "isError": false,
        "structuredContent": {"samplingDeclined": declined}
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
