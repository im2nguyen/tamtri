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
            let _ = notify_gateway_cancelled(&mcp_servers);
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
                        "result": {
                            "agentCapabilities": {
                                "streaming": true,
                                "tools": true,
                                "models": [
                                    {"id": "mock", "displayName": "Mock Model"},
                                    {"id": "mock-fast", "displayName": "Mock Fast"}
                                ]
                            }
                        }
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
                    if let Some(cwd_path) = cwd.as_deref() {
                        let _ = std::fs::create_dir_all(cwd_path);
                        let marker =
                            std::path::Path::new(cwd_path).join(".session-mcp-servers.json");
                        let _ = std::fs::write(
                            marker,
                            serde_json::to_string(&mcp_servers)
                                .unwrap_or_else(|_| "[]".to_string()),
                        );
                        let _ = std::fs::write(
                            std::path::Path::new(cwd_path).join(".session-cwd.txt"),
                            cwd_path,
                        );
                    }
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
                    if let Some(cwd) = &cwd {
                        let _ = std::fs::create_dir_all(cwd);
                        let prompt_text = prompt_text(&message);
                        let _ = std::fs::write(
                            std::path::Path::new(cwd).join(".session-prompt-seed.txt"),
                            &prompt_text,
                        );
                        let _ = std::fs::write(
                            std::path::Path::new(cwd).join("report.html"),
                            "<!doctype html><html><body><h1>ok</h1></body></html>",
                        );
                    }
                    emit_updates(&mut stdout);
                    let prompt_text = prompt_text(&message);
                    if let Some(cwd_path) = cwd.as_deref() {
                        if std::env::var("MOCK_ACP_CALL_FAIL_TOOL")
                            .ok()
                            .is_some_and(|value| value == "1")
                        {
                            let _ = call_gateway_fail(&mcp_servers);
                        }
                        let _ = call_gateway_echo(&mcp_servers, std::path::Path::new(cwd_path));
                        let _ = call_gateway_probe_roots(&mcp_servers, std::path::Path::new(cwd_path));
                    }
                    if prompt_text.contains("form-elicit") {
                        let cwd_path = cwd.as_deref().map(std::path::Path::new);
                        let _ = call_gateway_elicit_form(&mcp_servers, cwd_path);
                    } else if prompt_text.contains("url-elicit") {
                        let cwd_path = cwd.as_deref().map(std::path::Path::new);
                        let _ = call_gateway_elicit_url(&mcp_servers, cwd_path);
                    }
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
    let skip_file_changed = std::env::var("MOCK_ACP_SKIP_FILE_CHANGED")
        .ok()
        .is_some_and(|value| value == "1");
    let tool_update = if skip_file_changed {
        json!({
            "type": "tool_call_update",
            "toolCallId": "tool-1",
            "status": "completed",
            "text": "done"
        })
    } else {
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
        })
    };
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
        tool_update,
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

fn call_gateway_fail(mcp_servers: &[Value]) -> Result<(), String> {
    if mcp_servers.is_empty() {
        return Ok(());
    }
    let tools = list_gateway_tools(mcp_servers)?;
    let tool_name = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("mock__fail"))
        .and_then(|tool| tool.get("name").and_then(Value::as_str))
        .ok_or_else(|| "gateway did not expose fail tool".to_string())?
        .to_string();
    let _ = call_gateway_tool(mcp_servers, &tool_name, json!({}));
    Ok(())
}

fn notify_gateway_cancelled(mcp_servers: &[Value]) -> Result<(), String> {
    if mcp_servers.is_empty() {
        return Ok(());
    }
    let mut session = GatewaySession::connect(mcp_servers)?;
    session.send_notification(
        "notifications/cancelled",
        json!({"requestId": "acp-tool-1"}),
    )
}

fn call_gateway_echo(mcp_servers: &[Value], cwd: &std::path::Path) -> Result<(), String> {
    if mcp_servers.is_empty() {
        return Ok(());
    }
    let mut session = GatewaySession::connect(mcp_servers)?;
    let tools = session.rpc_request(2, "tools/list", json!({}))?;
    let tools = tools
        .pointer("/tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tool_name = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("mock__echo"))
        .and_then(|tool| tool.get("name").and_then(Value::as_str))
        .ok_or_else(|| "gateway did not expose echo tool".to_string())?
        .to_string();
    let result = session.rpc_request(
        3,
        "tools/call",
        json!({
            "name": tool_name,
            "arguments": json!({"message": "gateway-echo-test"}),
            "_meta": {"toolCallId": "acp-tool-1"}
        }),
    )?;
    let message = result
        .pointer("/structuredContent/echo/message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if message == "gateway-echo-test" {
        std::fs::write(cwd.join(".gateway-echo-ok"), message)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn call_gateway_probe_roots(mcp_servers: &[Value], cwd: &std::path::Path) -> Result<(), String> {
    if mcp_servers.is_empty() {
        return Ok(());
    }
    let tools = list_gateway_tools(mcp_servers)?;
    let tool_name = tools
        .iter()
        .find(|tool| {
            tool.get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name.ends_with("__probe_roots"))
        })
        .and_then(|tool| tool.get("name").and_then(Value::as_str))
        .map(str::to_string);
    let Some(tool_name) = tool_name else {
        return Ok(());
    };
    let result = call_gateway_tool(mcp_servers, &tool_name, json!({}))?;
    let count = result
        .pointer("/structuredContent/roots/roots")
        .and_then(Value::as_array)
        .map(|roots| roots.len())
        .unwrap_or(0);
    if count > 0 {
        std::fs::write(cwd.join(".gateway-probe-roots-ok"), count.to_string())
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn prompt_text(message: &Value) -> String {
    message
        .pointer("/params/prompt")
        .and_then(|prompt| {
            if let Some(blocks) = prompt.as_array() {
                Some(
                    blocks
                        .iter()
                        .filter_map(|block| block.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join(""),
                )
            } else {
                prompt.as_str().map(str::to_string)
            }
        })
        .unwrap_or_default()
}

fn call_gateway_elicit_form(
    mcp_servers: &[Value],
    cwd: Option<&std::path::Path>,
) -> Result<(), String> {
    if mcp_servers.is_empty() {
        return Ok(());
    }
    let tools = list_gateway_tools(mcp_servers)?;
    let tool_name = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("mock__elicit"))
        .or_else(|| {
            tools.iter().find(|tool| {
                tool.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .ends_with("__elicit")
            })
        })
        .and_then(|tool| tool.get("name").and_then(Value::as_str))
        .ok_or_else(|| "gateway did not expose form elicit tool".to_string())?
        .to_string();
    let result = call_gateway_tool(mcp_servers, &tool_name, json!({}))?;
    let name = result
        .pointer("/structuredContent/name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if let Some(cwd) = cwd
        && !name.is_empty()
    {
        std::fs::write(cwd.join(".gateway-elicit-form-ok"), name)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn call_gateway_elicit_url(
    mcp_servers: &[Value],
    cwd: Option<&std::path::Path>,
) -> Result<(), String> {
    if mcp_servers.is_empty() {
        return Ok(());
    }
    let tools = list_gateway_tools(mcp_servers)?;
    let tool_name = match tools
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
        .and_then(|tool| tool.get("name").and_then(Value::as_str))
    {
        Some(name) => name.to_string(),
        None => {
            if let Some(cwd) = cwd {
                let names: Vec<_> = tools
                    .iter()
                    .filter_map(|tool| tool.get("name").and_then(Value::as_str))
                    .collect();
                let _ = std::fs::write(cwd.join(".gateway-elicit-debug"), names.join("\n"));
            }
            return Ok(());
        }
    };
    let _ = call_gateway_tool(mcp_servers, &tool_name, json!({}))?;
    Ok(())
}

fn list_gateway_tools(mcp_servers: &[Value]) -> Result<Vec<Value>, String> {
    let mut session = GatewaySession::connect(mcp_servers)?;
    let result = session.rpc_request(
        2,
        "tools/list",
        json!({}),
    )?;
    Ok(result
        .pointer("/tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

fn call_gateway_tool(mcp_servers: &[Value], tool_name: &str, arguments: Value) -> Result<Value, String> {
    let mut session = GatewaySession::connect(mcp_servers)?;
    session.rpc_request(
        3,
        "tools/call",
        json!({"name": tool_name, "arguments": arguments, "_meta": {"toolCallId": "acp-tool-1"}}),
    )
}

struct GatewaySession {
    child: std::process::Child,
    input: Box<dyn Write + Send>,
    reader: io::BufReader<Box<dyn std::io::Read + Send>>,
}

impl GatewaySession {
    fn connect(mcp_servers: &[Value]) -> Result<Self, String> {
        let Some(server) = mcp_servers.first() else {
            return Err("no gateway server configured".to_string());
        };
        if server.get("type").and_then(Value::as_str) != Some("stdio") {
            return Err("gateway echo test expects stdio transport".to_string());
        }
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
        let input = child.stdin.take().ok_or_else(|| "gateway stdin missing".to_string())?;
        let output = child.stdout.take().ok_or_else(|| "gateway stdout missing".to_string())?;
        let mut session = Self {
            child,
            input: Box::new(input),
            reader: io::BufReader::new(Box::new(output)),
        };
        session.rpc_request(
            1,
            "initialize",
            json!({"protocolVersion":"2025-11-25","clientInfo":{"name":"mock-acp-agent","version":"0.1.0"},"capabilities":{}}),
        )?;
        Ok(session)
    }

    fn rpc_request(
        &mut self,
        id: i64,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        rpc_request(&mut self.input, &mut self.reader, id, method, params)
    }

    fn send_notification(&mut self, method: &str, params: Value) -> Result<(), String> {
        let notification = json!({"jsonrpc": "2.0", "method": method, "params": params});
        serde_json::to_writer(&mut self.input, &notification).map_err(|err| err.to_string())?;
        writeln!(self.input).map_err(|err| err.to_string())?;
        self.input.flush().map_err(|err| err.to_string())
    }
}

impl Drop for GatewaySession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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
