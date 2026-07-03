use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut permission_id: Option<Value> = None;
    let mut cwd: Option<String> = None;
    let mut mcp_servers: Vec<Value> = Vec::new();

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
                Some("session/new") => {
                    cwd = message
                        .pointer("/params/cwd")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    mcp_servers = message
                        .pointer("/params/mcpServers")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    write_msg(
                        &mut stdout,
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {"sessionId": "mock-session"}
                        }),
                    )
                }
                Some("session/prompt") => {
                    let _ = call_gateway_elicit_url(&mcp_servers);
                    if let Some(cwd) = &cwd {
                        let _ = std::fs::create_dir_all(cwd);
                        let _ = std::fs::write(
                            std::path::Path::new(cwd).join("report.html"),
                            "<!doctype html><html><body><h1>ok</h1></body></html>",
                        );
                    }
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

fn call_gateway_elicit_url(mcp_servers: &[Value]) -> Result<(), String> {
    let Some(server) = mcp_servers.first() else {
        return Ok(());
    };
    if server.get("type").and_then(Value::as_str) != Some("stdio") {
        return Ok(());
    };
    let command = server
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| "mcp server missing command".to_string())?;
    let args = server
        .get("args")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();

    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| format!("spawn gateway failed: {err}"))?;
    let mut input = child.stdin.take().ok_or_else(|| "gateway stdin missing".to_string())?;
    let output = child.stdout.take().ok_or_else(|| "gateway stdout missing".to_string())?;
    let mut reader = io::BufReader::new(output);

    rpc_request(
        &mut input,
        &mut reader,
        1,
        "initialize",
        json!({"protocolVersion":"2025-11-25","clientInfo":{"name":"mock-acp-agent","version":"0.1.0"},"capabilities":{}}),
    )?;

    let tools = rpc_request(&mut input, &mut reader, 2, "tools/list", json!({}))?;
    let tool_name = tools
        .pointer("/tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool.get("name").and_then(Value::as_str) == Some("mock__elicit_url"))
                .or_else(|| {
                    tools.iter().find(|tool| {
                        tool.get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .contains("elicit_url")
                    })
                })
        })
        .and_then(|tool| tool.get("name").and_then(Value::as_str))
        .ok_or_else(|| "gateway did not expose elicit_url tool".to_string())?
        .to_string();

    let _ = rpc_request(
        &mut input,
        &mut reader,
        3,
        "tools/call",
        json!({"name": tool_name, "arguments": {}, "_meta": {"toolCallId": "acp-tool-1"}}),
    )?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

fn rpc_request(
    stdin: &mut impl Write,
    stdout: &mut impl BufRead,
    id: i64,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let request = json!({"jsonrpc":"2.0","id": id, "method": method, "params": params});
    serde_json::to_writer(&mut *stdin, &request).map_err(|err| err.to_string())?;
    writeln!(stdin).map_err(|err| err.to_string())?;
    stdin.flush().map_err(|err| err.to_string())?;

    let mut line = String::new();
    loop {
        line.clear();
        stdout.read_line(&mut line).map_err(|err| err.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line.trim_end()).map_err(|err| err.to_string())?;
        if value.get("id") == Some(&json!(id)) {
            if let Some(error) = value.get("error") {
                return Err(error.to_string());
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
        if value.get("method").and_then(Value::as_str) == Some("elicitation/create") {
            // The gateway should surface this elicitation to the UI via core.
            // We wait for the eventual tool/call result instead of responding directly here.
            continue;
        }
    }
}
