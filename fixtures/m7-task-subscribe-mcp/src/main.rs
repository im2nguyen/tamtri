//! MCP fixture for Milestone 7 task subscribe (RC) tests.

use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = Arc::new(Mutex::new(io::stdout()));
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
            Some("initialize") => Some(response(
                &message,
                json!({
                    "protocolVersion": "2026-07-28",
                    "capabilities": {
                        "tools": {"listChanged": false},
                        "tasks": {
                            "list": {},
                            "cancel": {},
                            "requests": {"tools": {"call": {}}}
                        },
                        "extensions": {
                            "io.modelcontextprotocol/tasks": {
                                "version": "1",
                                "subscribe": true
                            }
                        }
                    },
                    "serverInfo": {"name": "m7-task-subscribe-mcp", "version": "0.1.0"}
                }),
            )),
            Some("tools/list") => Some(response(
                &message,
                json!({
                    "tools": [{
                        "name": "subscribe_task",
                        "description": "Task that pushes status via notifications/tasks/status",
                        "inputSchema": {"type": "object"},
                        "execution": {"taskSupport": "required"}
                    }]
                }),
            )),
            Some("tools/call") => handle_tools_call(&message, Arc::clone(&stdout)),
            Some("notifications/initialized") => continue,
            Some(method) => Some(error_response(&message, -32601, format!("unknown method: {method}"))),
            None => Some(error_response(&message, -32600, "missing method")),
        };

        if let Some(response) = response {
            let mut out = stdout.lock().expect("stdout");
            if serde_json::to_writer(&mut *out, &response).is_err() {
                break;
            }
            if writeln!(out).is_err() {
                break;
            }
            if out.flush().is_err() {
                break;
            }
        }
    }
}

fn handle_tools_call(message: &Value, stdout: Arc<Mutex<io::Stdout>>) -> Option<Value> {
    let tool_name = message
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if message.pointer("/params/task").is_none() {
        return Some(error_response(
            message,
            -32601,
            "task augmentation required",
        ));
    }
    if tool_name != "subscribe_task" {
        return Some(error_response(message, -32602, "unknown tool"));
    }

    let task_id = "subscribe-task-1";
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(75));
        let _ = push_status_notification(&stdout, task_id, "working", "Step 1/2");
        thread::sleep(Duration::from_millis(75));
        let _ = push_status_notification(&stdout, task_id, "completed", "Done");
    });

    Some(response(
        message,
        json!({
            "task": {
                "taskId": task_id,
                "status": "working",
                "statusMessage": "Task started",
                "pollInterval": 60000,
                "ttl": 60000
            }
        }),
    ))
}

fn push_status_notification(
    stdout: &Arc<Mutex<io::Stdout>>,
    task_id: &str,
    status: &str,
    status_message: &str,
) -> io::Result<()> {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/tasks/status",
        "params": {
            "taskId": task_id,
            "status": status,
            "statusMessage": status_message
        }
    });
    let mut out = stdout.lock().expect("stdout");
    serde_json::to_writer(&mut *out, &notification)?;
    writeln!(out)?;
    out.flush()
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
