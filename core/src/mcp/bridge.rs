use serde_json::json;

use crate::conversation::ContentBlock;
use crate::mcp::protocol::CallToolResult;

pub fn tool_call_block(id: &str, name: &str, arguments: &serde_json::Value) -> ContentBlock {
    ContentBlock::ToolCall {
        id: id.to_string(),
        name: name.to_string(),
        input: arguments.clone(),
    }
}

pub fn tool_result_block(call_id: &str, result: &CallToolResult) -> ContentBlock {
    ContentBlock::ToolResult {
        call_id: call_id.to_string(),
        output: json!({
            "content": result.content,
            "is_error": result.is_error,
            "structured_content": result.structured_content,
        }),
    }
}
