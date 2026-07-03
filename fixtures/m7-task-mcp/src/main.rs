//! MCP fixture for Milestone 7 task protocol tests.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

static TASKS: OnceLock<Mutex<HashMap<String, TaskRecord>>> = OnceLock::new();
static ELICITATION_WAITERS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TaskRecord {
    kind: TaskKind,
    step: u8,
    status: &'static str,
    status_message: Option<String>,
    #[allow(dead_code)]
    created_at: String,
    last_updated_at: String,
}

#[derive(Debug, Clone, Copy)]
enum TaskKind {
    Progress,
    Cancelable,
    InputRequired,
}

fn tasks() -> &'static Mutex<HashMap<String, TaskRecord>> {
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn elicitation_waiters() -> &'static Mutex<HashMap<String, String>> {
    ELICITATION_WAITERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("2025-11-25T00:{:02}:00Z", secs % 60)
}

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
            Some("initialize") => Some(response(
                &message,
                json!({
                    "protocolVersion": "2025-11-25",
                    "capabilities": {
                        "tools": {"listChanged": false},
                        "elicitation": {"form": {}, "url": {}},
                        "tasks": {
                            "list": {},
                            "cancel": {},
                            "requests": {"tools": {"call": {}}}
                        },
                        "extensions": {
                            "io.modelcontextprotocol/tasks": {
                                "version": "1"
                            }
                        }
                    },
                    "serverInfo": {"name": "m7-task-mcp", "version": "0.1.0"}
                }),
            )),
            Some("tools/list") => Some(response(
                &message,
                json!({
                    "tools": [
                        {
                            "name": "progress_task",
                            "description": "Long-running task with progress updates",
                            "inputSchema": {"type": "object"},
                            "execution": {"taskSupport": "required"}
                        },
                        {
                            "name": "cancelable_task",
                            "description": "Task that can be cancelled",
                            "inputSchema": {"type": "object"},
                            "execution": {"taskSupport": "required"}
                        },
                        {
                            "name": "input_task",
                            "description": "Task that asks for mid-task input",
                            "inputSchema": {"type": "object"},
                            "execution": {"taskSupport": "required"}
                        }
                    ]
                }),
            )),
            Some("tools/call") => handle_tools_call(&message, &mut stdout, &mut input),
            Some("tasks/get") => handle_tasks_get(&message, &mut stdout, &mut input),
            Some("tasks/cancel") => handle_tasks_cancel(&message),
            Some("tasks/result") => handle_tasks_result(&message),
            Some("notifications/initialized") => continue,
            Some(method) => Some(error_response(&message, -32601, format!("unknown method: {method}"))),
            None => Some(error_response(&message, -32600, "missing method")),
        };

        if let Some(response) = response {
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
}

fn handle_tools_call(
    message: &Value,
    stdout: &mut io::Stdout,
    input: &mut impl BufRead,
) -> Option<Value> {
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
    let task_id = format!("task-{}", tool_name);
    let kind = match tool_name {
        "progress_task" => TaskKind::Progress,
        "cancelable_task" => TaskKind::Cancelable,
        "input_task" => TaskKind::InputRequired,
        _ => {
            return Some(error_response(message, -32602, "unknown tool"));
        }
    };
    let now = now_iso();
    tasks().lock().unwrap().insert(
        task_id.clone(),
        TaskRecord {
            kind,
            step: 0,
            status: "working",
            status_message: Some("Task started".to_string()),
            created_at: now.clone(),
            last_updated_at: now,
        },
    );
    if matches!(kind, TaskKind::InputRequired) {
        // Mid-task input is requested on the first tasks/get poll.
    }
    Some(response(
        message,
        json!({
            "task": task_payload(&task_id, "working", Some("Task started".to_string()), 100)
        }),
    ))
}

fn handle_tasks_get(
    message: &Value,
    stdout: &mut io::Stdout,
    input: &mut impl BufRead,
) -> Option<Value> {
    let task_id = message
        .pointer("/params/taskId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut tasks_guard = tasks().lock().unwrap();
    let Some(record) = tasks_guard.get_mut(task_id) else {
        return Some(error_response(message, -32602, "unknown task"));
    };
    if record.status == "cancelled" || record.status == "completed" || record.status == "failed" {
        return Some(response(message, task_payload(task_id, record.status, record.status_message.clone(), 100)));
    }
    match record.kind {
        TaskKind::Progress => {
            record.step += 1;
            if record.step >= 3 {
                record.status = "completed";
                record.status_message = Some("Done".to_string());
            } else {
                record.status = "working";
                record.status_message =
                    Some(format!("Step {}/3", record.step));
            }
        }
        TaskKind::Cancelable => {
            record.status = "working";
            record.status_message = Some("Still running".to_string());
        }
        TaskKind::InputRequired => {
            record.step += 1;
            if record.step == 1 {
                record.status = "input_required";
                record.status_message = Some("Waiting for input".to_string());
                let _ = request_elicitation(stdout, input, task_id);
            } else if elicitation_waiters().lock().unwrap().remove(task_id).is_some() {
                record.status = "completed";
                record.status_message = Some("Input received".to_string());
            } else {
                record.status = "input_required";
                record.status_message = Some("Waiting for input".to_string());
            }
        }
    }
    record.last_updated_at = now_iso();
    let status = record.status;
    let message_text = record.status_message.clone();
    Some(response(
        message,
        task_payload(task_id, status, message_text, 100),
    ))
}

fn handle_tasks_cancel(message: &Value) -> Option<Value> {
    let task_id = message
        .pointer("/params/taskId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut tasks_guard = tasks().lock().unwrap();
    let Some(record) = tasks_guard.get_mut(task_id) else {
        return Some(error_response(message, -32602, "unknown task"));
    };
    if matches!(record.status, "completed" | "failed" | "cancelled") {
        return Some(error_response(message, -32602, "task already terminal"));
    }
    record.status = "cancelled";
    record.status_message = Some("Cancelled by client".to_string());
    record.last_updated_at = now_iso();
    Some(response(
        message,
        task_payload(task_id, "cancelled", Some("Cancelled by client".to_string()), 100),
    ))
}

fn handle_tasks_result(message: &Value) -> Option<Value> {
    let task_id = message
        .pointer("/params/taskId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tasks_guard = tasks().lock().unwrap();
    let Some(record) = tasks_guard.get(task_id) else {
        return Some(error_response(message, -32602, "unknown task"));
    };
    if record.status != "completed" {
        return Some(error_response(message, -32602, "task not completed"));
    }
    Some(response(
        message,
        json!({
            "content": [{"type": "text", "text": "task result: ok"}],
            "isError": false,
            "_meta": {
                "io.modelcontextprotocol/related-task": {"taskId": task_id}
            }
        }),
    ))
}

fn request_elicitation(
    stdout: &mut io::Stdout,
    input: &mut impl BufRead,
    task_id: &str,
) -> Result<(), String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": format!("elicit-{task_id}"),
        "method": "elicitation/create",
        "params": {
            "message": "What is your name?",
            "requestedSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"]
            },
            "_meta": {
                "io.modelcontextprotocol/related-task": {"taskId": task_id}
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
    let accepted = response
        .pointer("/result/action")
        .and_then(Value::as_str)
        .is_some_and(|action| action == "accept");
    if accepted {
        elicitation_waiters()
            .lock()
            .unwrap()
            .insert(task_id.to_string(), "accepted".to_string());
    }
    Ok(())
}

fn task_payload(
    task_id: &str,
    status: &str,
    status_message: Option<String>,
    poll_interval: u64,
) -> Value {
    let now = now_iso();
    let mut payload = json!({
        "taskId": task_id,
        "status": status,
        "createdAt": now,
        "lastUpdatedAt": now,
        "pollInterval": poll_interval,
        "ttl": 60000
    });
    if let Some(message) = status_message {
        payload["statusMessage"] = json!(message);
    }
    payload
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
