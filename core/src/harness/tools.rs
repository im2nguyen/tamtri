//! Transport-neutral gateway tool catalog entries for native adapters.

use serde::{Deserialize, Serialize};

use crate::mcp::gateway::GatewayTool;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCatalogEntry {
    pub exposed_name: String,
    pub server_id: String,
    pub original_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: serde_json::Value,
}

pub fn from_gateway_tools(tools: Vec<GatewayTool>) -> Vec<ToolCatalogEntry> {
    tools
        .into_iter()
        .map(|tool| ToolCatalogEntry {
            exposed_name: tool.exposed_name,
            server_id: tool.server_id,
            original_name: tool.original_name,
            description: tool.tool.description,
            input_schema: tool.tool.input_schema.clone(),
        })
        .collect()
}
