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
                        "roots": {"listChanged": false}
                    },
                    "serverInfo": {"name": "m7-roots-mcp", "version": "0.1.0"}
                }),
            ),
            Some("tools/list") => response(
                &message,
                json!({
                    "tools": [
                        {
                            "name": "probe_roots",
                            "description": "Requests roots/list from the client and returns the result",
                            "inputSchema": {"type": "object"}
                        },
                        {
                            "name": "validate_path",
                            "description": "Checks whether a path is under a client root",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": {"type": "string"}
                                },
                                "required": ["path"]
                            }
                        }
                    ]
                }),
            ),
            Some("tools/call") => {
                let tool_name = message
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match tool_name {
                    "probe_roots" => match probe_roots(&mut stdout, &mut input) {
                        Ok(result) => response(&message, result),
                        Err(err) => error_response(&message, -32000, err),
                    },
                    "validate_path" => {
                        let path = message
                            .pointer("/params/arguments/path")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        match validate_path(&mut stdout, &mut input, path) {
                            Ok(result) => response(&message, result),
                            Err(err) => error_response(&message, -32000, err),
                        }
                    }
                    _ => error_response(&message, -32601, format!("unknown tool: {tool_name}")),
                }
            }
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

fn probe_roots(stdout: &mut io::Stdout, input: &mut impl BufRead) -> Result<Value, String> {
    let roots = request_roots_list(stdout, input)?;
    Ok(json!({
        "content": [{"type": "text", "text": roots.to_string()}],
        "isError": false,
        "structuredContent": {"roots": roots}
    }))
}

fn validate_path(
    stdout: &mut io::Stdout,
    input: &mut impl BufRead,
    path: &str,
) -> Result<Value, String> {
    let roots = request_roots_list(stdout, input)?;
    let allowed = path_under_any_root(path, &roots);
    Ok(json!({
        "content": [{"type": "text", "text": format!("allowed={allowed}")}],
        "isError": false,
        "structuredContent": {"path": path, "allowed": allowed}
    }))
}

fn request_roots_list(stdout: &mut io::Stdout, input: &mut impl BufRead) -> Result<Value, String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": "roots-1",
        "method": "roots/list",
        "params": {}
    });
    serde_json::to_writer(&mut *stdout, &request).map_err(|err| err.to_string())?;
    writeln!(stdout).map_err(|err| err.to_string())?;
    stdout.flush().map_err(|err| err.to_string())?;

    let mut line = String::new();
    input
        .read_line(&mut line)
        .map_err(|err| err.to_string())?;
    if line.trim().is_empty() {
        return Err("missing roots/list response".to_string());
    }
    let response: Value = serde_json::from_str(line.trim_end()).map_err(|err| err.to_string())?;
    response
        .get("result")
        .cloned()
        .ok_or_else(|| "roots/list response missing result".to_string())
}

fn path_under_any_root(path: &str, roots_result: &Value) -> bool {
    let Some(roots) = roots_result.get("roots").and_then(Value::as_array) else {
        return false;
    };
    let path = normalize_path(path);
    for root in roots {
        let Some(uri) = root.get("uri").and_then(Value::as_str) else {
            continue;
        };
        let root_path = normalize_path(uri.trim_start_matches("file://"));
        if path.starts_with(&root_path) {
            return true;
        }
    }
    false
}

fn normalize_path(path: &str) -> String {
    path.trim_end_matches('/').to_string()
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
