use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

const TEMPLATE_URI: &str = "ui://m7-app/demo";
const BAD_ORIGIN_TEMPLATE_URI: &str = "ui://m7-app/bad-origin";
const MCP_APP_MIME: &str = "text/html;profile=mcp-app";

/// MCP fixture that declares an App template and returns it through a gateway tool.
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
                        "extensions": {
                            "io.modelcontextprotocol/apps": {"version": "1"}
                        }
                    },
                    "serverInfo": {"name": "m7-app-mcp", "version": "0.1.0"}
                }),
            ),
            Some("tools/list") => response(
                &message,
                json!({
                    "tools": [
                        {
                            "name": "show_app",
                            "description": "Returns a declared MCP App template",
                            "inputSchema": {"type": "object"},
                            "_meta": {
                                "ui": {
                                    "resourceUri": TEMPLATE_URI
                                }
                            }
                        },
                        {
                            "name": "show_bad_origin_app",
                            "description": "Returns an MCP App template with an invalid declared origin",
                            "inputSchema": {"type": "object"},
                            "_meta": {
                                "ui": {
                                    "resourceUri": BAD_ORIGIN_TEMPLATE_URI
                                }
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
                if tool_name == "show_app" {
                    response(
                        &message,
                        json!({
                            "content": [{"type": "text", "text": "App ready"}],
                            "isError": false,
                            "structuredContent": {
                                "title": "Demo App",
                                "value": 42
                            }
                        }),
                    )
                } else if tool_name == "show_bad_origin_app" {
                    response(
                        &message,
                        json!({
                            "content": [{"type": "text", "text": "Bad origin app"}],
                            "isError": false,
                            "structuredContent": {
                                "title": "Bad Origin App",
                                "value": 0
                            }
                        }),
                    )
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
            Some("resources/list") => response(
                &message,
                json!({
                    "resources": [
                        {
                            "uri": TEMPLATE_URI,
                            "name": "demo_app",
                            "description": "Declared MCP App template",
                            "mimeType": MCP_APP_MIME
                        },
                        {
                            "uri": BAD_ORIGIN_TEMPLATE_URI,
                            "name": "bad_origin_app",
                            "description": "MCP App template with invalid declared origin",
                            "mimeType": MCP_APP_MIME
                        }
                    ]
                }),
            ),
            Some("resources/read") => {
                let uri = message
                    .pointer("/params/uri")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if uri == TEMPLATE_URI {
                    response(&message, template_contents())
                } else if uri == BAD_ORIGIN_TEMPLATE_URI {
                    response(&message, bad_origin_template_contents())
                } else {
                    error_response(&message, -32002, format!("unknown resource: {uri}"))
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

fn template_contents() -> Value {
    json!({
        "contents": [{
            "uri": TEMPLATE_URI,
            "mimeType": MCP_APP_MIME,
            "text": "<!DOCTYPE html><html><body><h1>Demo App</h1></body></html>",
            "_meta": {
                "ui": {
                    "csp": {
                        "connectDomains": ["https://api.example.com"]
                    }
                }
            }
        }]
    })
}

fn bad_origin_template_contents() -> Value {
    json!({
        "contents": [{
            "uri": BAD_ORIGIN_TEMPLATE_URI,
            "mimeType": MCP_APP_MIME,
            "text": "<!DOCTYPE html><html><body><h1>Bad Origin App</h1></body></html>",
            "_meta": {
                "ui": {
                    "csp": {
                        "connectDomains": ["https://bad origin"]
                    }
                }
            }
        }]
    })
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
