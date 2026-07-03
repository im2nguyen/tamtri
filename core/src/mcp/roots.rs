use async_trait::async_trait;
use serde_json::{Value, json};

use crate::conversation::Root;
use crate::mcp::jsonrpc::JsonRpcError;

#[async_trait]
pub trait RootsHandler: Send + Sync {
    async fn handle_list(&self) -> std::result::Result<Value, JsonRpcError>;
}

pub fn roots_list_result(roots: &[Root]) -> Value {
    json!({
        "roots": roots.iter().map(root_to_mcp).collect::<Vec<_>>()
    })
}

pub fn root_to_mcp(root: &Root) -> Value {
    let mut item = json!({ "uri": root.uri });
    if !root.name.is_empty() {
        item["name"] = json!(root.name);
    }
    item
}
