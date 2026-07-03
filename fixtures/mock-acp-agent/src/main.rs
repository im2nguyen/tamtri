use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut permission_id: Option<Value> = None;

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

        if message.get("method").and_then(Value::as_str) == Some("session/cancel") {
            continue;
        }

        if message.get("id") == Some(&json!("perm-1")) && message.get("method").is_none() {
            if let Some(prompt_id) = permission_id.take() {
                write_msg(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": prompt_id,
                        "result": {"stopReason": "end_turn"}
                    }),
                );
            }
        } else if let Some(id) = message.get("id").cloned() {
            match message.get("method").and_then(Value::as_str) {
                Some("initialize") => write_msg(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"agentCapabilities": {"streaming": true, "tools": true}}
                    }),
                ),
                Some("session/new") => write_msg(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"sessionId": "mock-session"}
                    }),
                ),
                Some("session/prompt") => {
                    emit_updates(&mut stdout);
                    let req_id = json!("perm-1");
                    permission_id = Some(id);
                    write_msg(
                        &mut stdout,
                        json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "method": "session/request_permission",
                            "params": {
                                "requestId": "perm-1",
                                "action": "edit",
                                "diff": {
                                    "path": "report.html",
                                    "change": "modified",
                                    "oldText": "",
                                    "newText": "<h1>ok</h1>"
                                },
                                "options": [
                                    {"id": "allow_once", "label": "Allow once"},
                                    {"id": "deny", "label": "Deny"}
                                ]
                            }
                        }),
                    );
                }
                _ => write_msg(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32601, "message": "method not found"}
                    }),
                ),
            }
        }
    }
}

fn emit_updates(stdout: &mut io::Stdout) {
    let updates = [
        json!({"type": "agent_thought_chunk", "text": "thinking"}),
        json!({"type": "agent_message_chunk", "text": "Hello"}),
        json!({"type": "agent_message_chunk", "text": " world"}),
        json!({
            "type": "tool_call",
            "id": "tool-1",
            "name": "Write",
            "kind": "write",
            "title": "Write report",
            "input": {"path": "report.html"}
        }),
        json!({
            "type": "tool_call_update",
            "toolCallId": "tool-1",
            "status": "completed",
            "diff": {
                "path": "report.html",
                "change": "modified",
                "oldText": "",
                "newText": "<h1>ok</h1>"
            }
        }),
    ];
    for update in updates {
        write_msg(
            stdout,
            json!({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": update
            }),
        );
    }
}

fn write_msg(stdout: &mut io::Stdout, value: Value) {
    let _ = serde_json::to_writer(&mut *stdout, &value);
    let _ = writeln!(stdout);
    let _ = stdout.flush();
}
