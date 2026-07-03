use std::sync::Arc;

use serde_json::{Value, json};

use crate::Result;
use crate::mcp::capabilities::upstream_gateway_capabilities;
use crate::mcp::gateway::McpGateway;
use crate::mcp::protocol::{Implementation, MCP_PROTOCOL_VERSION};
use crate::rpc::dispatch::{InboundMessage, RpcConnection};
use crate::rpc::jsonrpc::JsonRpcError;
use crate::rpc::transport::Transport;

pub async fn serve_gateway_transport(
    transport: Box<dyn Transport>,
    gateway: Arc<McpGateway>,
) -> Result<()> {
    let (handle, mut inbound) = RpcConnection::start(transport);
    while let Some(message) = inbound.recv().await {
        match message {
            InboundMessage::Request(req) => {
                let result = handle_gateway_request(&gateway, &req.method, req.params).await;
                handle.respond(req.id, result).await?;
            }
            InboundMessage::Notification(note) => {
                if matches!(
                    note.method.as_str(),
                    "notifications/cancelled"
                        | "notifications/cancelledRequest"
                        | "$/cancelRequest"
                ) {
                    gateway.agent_cancelled(note.params.unwrap_or_else(|| json!({})));
                }
                tracing::debug!("agent-facing MCP notification {}", note.method);
            }
        }
    }
    Ok(())
}

pub(crate) async fn handle_gateway_request(
    gateway: &McpGateway,
    method: &str,
    params: Option<Value>,
) -> std::result::Result<Value, JsonRpcError> {
    match method {
        "initialize" => {
            let capabilities = upstream_gateway_capabilities();
            Ok(json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": capabilities,
                "serverInfo": Implementation {
                    name: "tamtri-gateway".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                }
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => gateway
            .list_tools()
            .await
            .map(|tools| {
                json!({
                    "tools": tools.into_iter().map(|tool| {
                        let mut exposed = tool.tool;
                        exposed.name = tool.exposed_name;
                        exposed
                    }).collect::<Vec<_>>()
                })
            })
            .map_err(to_jsonrpc_error),
        "tools/call" => {
            let params = params.unwrap_or_else(|| json!({}));
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let meta = params.get("_meta").cloned();
            gateway
                .call_tool_with_meta(name, arguments, meta)
                .await
                .map(|result| serde_json::to_value(result).unwrap_or_else(|_| json!({})))
                .map_err(to_jsonrpc_error)
        }
        "resources/list" => gateway
            .list_resources()
            .await
            .map(|resources| {
                json!({
                    "resources": resources.into_iter().map(|resource| {
                        let mut exposed = resource.resource;
                        exposed.uri = resource.exposed_uri;
                        exposed
                    }).collect::<Vec<_>>()
                })
            })
            .map_err(to_jsonrpc_error),
        "resources/read" => {
            let params = params.unwrap_or_else(|| json!({}));
            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .unwrap_or_default();
            gateway
                .read_resource(uri)
                .await
                .map(|result| serde_json::to_value(result).unwrap_or_else(|_| json!({})))
                .map_err(to_jsonrpc_error)
        }
        "prompts/list" => gateway
            .list_prompts()
            .await
            .map(|prompts| {
                json!({
                    "prompts": prompts.into_iter().map(|prompt| {
                        let mut exposed = prompt.prompt;
                        exposed.name = prompt.exposed_name;
                        exposed
                    }).collect::<Vec<_>>()
                })
            })
            .map_err(to_jsonrpc_error),
        "prompts/get" => {
            let params = params.unwrap_or_else(|| json!({}));
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            gateway
                .get_prompt(name, arguments)
                .await
                .map(|result| serde_json::to_value(result).unwrap_or_else(|_| json!({})))
                .map_err(to_jsonrpc_error)
        }
        "sampling/create" => Err(JsonRpcError {
            code: crate::rpc::jsonrpc::METHOD_NOT_FOUND,
            message: "sampling is not supported".to_string(),
            data: None,
        }),
        "roots/list" => gateway
            .list_roots()
            .await
            .map_err(to_jsonrpc_error),
        _ => Err(JsonRpcError {
            code: crate::rpc::jsonrpc::METHOD_NOT_FOUND,
            message: "method not found".to_string(),
            data: None,
        }),
    }
}

fn to_jsonrpc_error(err: crate::CoreError) -> JsonRpcError {
    JsonRpcError {
        code: -32000,
        message: err.to_string(),
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::config::GatewayConfig;
    use crate::mcp::gateway::NoCredentials;

    #[tokio::test]
    async fn gateway_initialize_shape() {
        let gateway =
            McpGateway::new(GatewayConfig::default(), Arc::new(NoCredentials), None).unwrap();
        let result = handle_gateway_request(&gateway, "initialize", None)
            .await
            .unwrap();
        assert_eq!(result["serverInfo"]["name"], "tamtri-gateway");
        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
        assert!(result["capabilities"]["logging"].is_object());
        assert!(result["capabilities"]["sampling"].is_null());
        assert!(result["capabilities"]["tasks"].is_null());
    }

    #[tokio::test]
    async fn gateway_sampling_declined_cleanly() {
        let gateway =
            McpGateway::new(GatewayConfig::default(), Arc::new(NoCredentials), None).unwrap();
        let err = handle_gateway_request(&gateway, "sampling/create", Some(json!({})))
            .await
            .expect_err("sampling should be declined");
        assert_eq!(err.code, crate::rpc::jsonrpc::METHOD_NOT_FOUND);
        assert!(err.message.contains("sampling"));
    }
}
